//! Client HTTP **LanguageTool** (`/v2/check`) — analyse grammaticale française.

use serde::Deserialize;

use crate::corrector::options::LtOptions;

/// Point d'entrée public de l'API LanguageTool.
const ENDPOINT: &str = "https://api.languagetool.org/v2/check";

/// Réponse racine de `/v2/check`.
#[derive(Debug, Deserialize)]
pub struct LtResponse {
    pub matches: Vec<LtMatch>,
}

/// Correspondance brute renvoyée par LanguageTool.
#[derive(Debug, Deserialize)]
pub struct LtMatch {
    pub offset: usize,
    pub length: usize,
    pub message: String,
    #[serde(default)]
    pub replacements: Vec<LtReplacement>,
    #[serde(default)]
    pub context: Option<LtContext>,
    #[serde(default)]
    pub rule: Option<LtRule>,
}

/// Remplacement suggéré par LanguageTool.
#[derive(Debug, Deserialize)]
pub struct LtReplacement {
    pub value: String,
}

/// Contexte textuel de la correspondance.
#[derive(Debug, Deserialize)]
pub struct LtContext {
    pub text: String,
}

/// Règle déclenchée.
#[derive(Debug, Deserialize)]
pub struct LtRule {
    pub id: String,
    /// Libellé **humain** de la règle (aucun jargon technique affiché à l'écran).
    #[serde(default)]
    pub description: Option<String>,
}

/// Correspondance **nettoyée**, transmise au frontend.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CorrectionMatch {
    /// Décalage du fragment fautif (en caractères).
    pub offset: usize,
    /// Longueur du fragment fautif (en caractères).
    pub length: usize,
    /// Message explicatif de LanguageTool.
    pub message: String,
    /// Remplacements suggérés (valeurs seules).
    pub replacements: Vec<String>,
    /// Phrase de contexte (`None` si absente).
    pub context: Option<String>,
    /// Identifiant de la règle déclenchée (`None` si absent).
    pub rule_id: Option<String>,
    /// Libellé **humain** de la règle (`None` si absent).
    pub rule_description: Option<String>,
}

impl From<LtMatch> for CorrectionMatch {
    fn from(value: LtMatch) -> Self {
        let (rule_id, rule_description) = match value.rule {
            Some(rule) => (Some(rule.id), rule.description),
            None => (None, None),
        };
        CorrectionMatch {
            offset: value.offset,
            length: value.length,
            message: value.message,
            replacements: value.replacements.into_iter().map(|r| r.value).collect(),
            context: value.context.map(|c| c.text),
            rule_id,
            rule_description,
        }
    }
}

/// Taille maximale d'un tronçon envoyé à l'API (marge sous la limite publique
/// de 20 Ko : évite les HTTP 500 sur les manuscrits volumineux, y compris en
/// mode « picky » où le calcul côté serveur est plus lourd).
const MAX_CHUNK_BYTES: usize = 8_000;

/// Analyse un texte **français** via LanguageTool, selon les `options` fournies
/// (`language`, `level=picky`, `disabledRules`).
///
/// Au-delà de [`MAX_CHUNK_BYTES`], le texte est découpé en **tronçons sûrs**
/// (jamais au milieu d'un mot) envoyés **en parallèle** (`join_all`) ; les
/// `offset` sont réajustés du décalage de chaque tronçon, puis l'ensemble est
/// concaténé dans l'ordre chronologique du texte.
///
/// **Tolérance aux erreurs partielles** : un tronçon en échec (HTTP 500,
/// timeout) est signalé en console sans interrompre les tronçons réussis.
///
/// Les défaillances globales (aucun tronçon exploitable) restent propagées en
/// `Result::Err` (politique « Tolérance Zéro »).
pub async fn check_french(
    text: &str,
    options: &LtOptions,
) -> Result<Vec<CorrectionMatch>, String> {
    // Découpe + décalage de départ **en caractères** de chaque tronçon.
    let mut planned: Vec<(&str, usize)> = Vec::new();
    let mut start = 0usize;
    for chunk in split_chunks(text) {
        planned.push((chunk, start));
        start += chunk.chars().count();
    }

    // Requêtes concurrentes : un tronçon en échec n'interrompt pas les autres.
    let requests = planned
        .into_iter()
        .map(|(chunk, chunk_start)| async move {
            (chunk_start, request_chunk(chunk, options).await)
        });
    let responses = futures::future::join_all(requests).await;

    let mut combined: Vec<CorrectionMatch> = Vec::new();
    let mut failures = 0usize;
    for (chunk_start, result) in responses {
        match result {
            Ok(mut matches) => {
                for entry in &mut matches {
                    entry.offset += chunk_start;
                }
                combined.append(&mut matches);
            }
            Err(error) => {
                failures += 1;
                println!("avertissement : tronçon du correcteur ignoré ({error})");
            }
        }
    }

    // Aucun tronçon exploitable : l'échec global est remonté (jamais silencieux).
    if failures > 0 && combined.is_empty() {
        return Err(format!(
            "Correcteur : {failures} tronçon(s) en échec, aucune suggestion disponible."
        ));
    }

    Ok(combined)
}

/// Interroge LanguageTool sur **un tronçon** (aucun réajustement d'offset ici).
async fn request_chunk(
    text: &str,
    options: &LtOptions,
) -> Result<Vec<CorrectionMatch>, String> {
    let mut form: Vec<(String, String)> = vec![
        ("language".to_string(), options.language.clone()),
        ("text".to_string(), text.to_string()),
    ];
    if options.picky {
        form.push(("level".to_string(), "picky".to_string()));
    }

    // Identifiants de règles **nettoyés** (trim + jeu de caractères sûr) : aucune
    // chaîne vide ni caractère invalide n'est transmise à l'API.
    let disabled: Vec<&str> = options
        .disabled_rules
        .iter()
        .map(|rule| rule.trim())
        .filter(|rule| {
            !rule.is_empty()
                && rule
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        })
        .collect();
    if !disabled.is_empty() {
        form.push(("disabledRules".to_string(), disabled.join(",")));
    }

    let response = reqwest::Client::new()
        .post(ENDPOINT)
        .header("Accept", "application/json")
        .form(&form)
        .send()
        .await
        .map_err(|error| format!("Connexion au correcteur impossible : {error}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Correcteur : réponse HTTP {} ({ENDPOINT})",
            response.status()
        ));
    }

    let parsed: LtResponse = response
        .json()
        .await
        .map_err(|error| format!("Réponse du correcteur illisible : {error}"))?;

    Ok(parsed.matches.into_iter().map(CorrectionMatch::from).collect())
}

/// Découpe un texte en tronçons d'au plus [`MAX_CHUNK_BYTES`] octets, **sans
/// jamais couper au milieu d'un mot** : priorité aux doubles sauts de ligne
/// `\n\n`, puis aux sauts simples `\n`, puis à l'ultime espace, enfin à la
/// frontière de caractère la plus proche.
fn split_chunks(text: &str) -> Vec<&str> {
    if text.len() <= MAX_CHUNK_BYTES {
        return vec![text];
    }

    let mut chunks: Vec<&str> = Vec::new();
    let mut start = 0usize;
    while start < text.len() {
        let remaining = &text[start..];
        if remaining.len() <= MAX_CHUNK_BYTES {
            chunks.push(remaining);
            break;
        }
        let limit = floor_char_boundary(text, start + MAX_CHUNK_BYTES);
        let window = &text[start..limit];
        let cut = window
            .rfind("\n\n")
            .map(|at| at + 2)
            .or_else(|| window.rfind('\n').map(|at| at + 1))
            .or_else(|| window.rfind(char::is_whitespace))
            .unwrap_or(window.len());
        chunks.push(&text[start..start + cut]);
        start += cut;
    }
    chunks
}

/// Recule jusqu'à la **frontière de caractère** valide la plus proche (UTF-8).
fn floor_char_boundary(text: &str, index: usize) -> usize {
    if index >= text.len() {
        return text.len();
    }
    let mut at = index;
    while at > 0 && !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}
