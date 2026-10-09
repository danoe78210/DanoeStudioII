//! Module **Correcteur** — analyse grammaticale (LanguageTool) + dictionnaire local.
//!
//! Phase 1 : client HTTP, dictionnaire persistant, commande d'analyse.

pub mod client;
pub mod dictionary;
pub mod options;

use std::collections::BTreeSet;

use crate::corrector::client::CorrectionMatch;
use crate::corrector::options::LtOptions;

/// Extrait le fragment visé par une correspondance (offsets **en caractères**,
/// tels que renvoyés par LanguageTool).
fn match_token(text: &str, offset: usize, length: usize) -> String {
    text.chars().skip(offset).take(length).collect()
}

/// `true` si le fragment fautif est un mot du **dictionnaire local** (ignoré).
fn is_ignored(text: &str, entry: &CorrectionMatch, ignored: &BTreeSet<String>) -> bool {
    let token = match_token(text, entry.offset, entry.length)
        .trim()
        .to_lowercase();
    !token.is_empty() && ignored.contains(&token)
}

/// `true` si la règle de la correspondance est **désactivée** dans les options.
fn is_rule_disabled(entry: &CorrectionMatch, options: &LtOptions) -> bool {
    match &entry.rule_id {
        Some(id) => options.disabled_rules.contains(id),
        None => false,
    }
}

/// Heuristique : règle **orthographique** (MORFOLOGIK / SPELL / FRENCH_…).
fn is_spelling_rule(rule_id: Option<&str>) -> bool {
    let Some(id) = rule_id else {
        return false;
    };
    let id = id.to_uppercase();
    id.contains("MORFOLOGIK") || id.contains("SPELL") || id.contains("FRENCH_")
}

/// `true` si l'alerte est **orthographique** et porte sur un **toponyme connu**.
fn is_known_place(text: &str, entry: &CorrectionMatch, places: &BTreeSet<String>) -> bool {
    if !is_spelling_rule(entry.rule_id.as_deref()) {
        return false;
    }
    let token = match_token(text, entry.offset, entry.length)
        .trim()
        .to_lowercase();
    !token.is_empty() && places.contains(&token)
}

/// Masque la **syntaxe Markdown** par des **espaces** (longueur en caractères
/// préservée → offsets LanguageTool strictement intacts) : frontmatter YAML,
/// appels de notes `[^label]`, code inline `` `…` `` et barré `~~…~~`.
fn mask_markdown(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let starts = |at: usize, token: &str| {
        at + token.chars().count() <= len
            && chars.iter().skip(at).take(token.len()).copied().eq(token.chars())
    };
    let blank = |chars: &mut Vec<char>, from: usize, to: usize| {
        for slot in chars.iter_mut().take(to.min(len)).skip(from) {
            *slot = ' ';
        }
    };

    // Frontmatter YAML de tête (`---` … `---`).
    if starts(0, "---") {
        let mut cursor = 3;
        while cursor < len && chars[cursor] != '\n' {
            cursor += 1;
        }
        let mut end = len;
        while cursor < len {
            cursor += 1;
            if starts(cursor, "---") {
                end = cursor + 3;
                break;
            }
            while cursor < len && chars[cursor] != '\n' {
                cursor += 1;
            }
        }
        blank(&mut chars, 0, end);
    }

    // Appels de notes, code inline, barré.
    let mut index = 0;
    while index < len {
        if chars[index] == '[' && index + 1 < len && chars[index + 1] == '^' {
            if let Some(close) = (index..len).find(|&at| chars[at] == ']') {
                blank(&mut chars, index, close + 1);
                index = close + 1;
                continue;
            }
        }
        if chars[index] == '`' {
            if let Some(close) = (index + 1..len).find(|&at| chars[at] == '`') {
                blank(&mut chars, index, close + 1);
                index = close + 1;
                continue;
            }
        }
        if chars[index] == '~' && index + 1 < len && chars[index + 1] == '~' {
            if let Some(close) =
                (index + 2..len).find(|&at| chars[at] == '~' && at + 1 < len && chars[at + 1] == '~')
            {
                blank(&mut chars, index, close + 2);
                index = close + 2;
                continue;
            }
        }
        index += 1;
    }

    chars.into_iter().collect()
}

/// Analyse un chapitre : charge les options, **masque la syntaxe Markdown**,
/// appelle LanguageTool puis filtre (dictionnaire local, toponymes, règles off).
#[tauri::command]
pub async fn analyze_chapter(
    app: tauri::AppHandle,
    text: String,
) -> Result<Vec<CorrectionMatch>, String> {
    let options = options::load_options(&app)?;
    let masked = mask_markdown(&text);
    let matches = client::check_french(&masked, &options).await?;
    let ignored = dictionary::load_ignored(&app)?;
    let places = dictionary::load_places(&app)?;
    Ok(matches
        .into_iter()
        .filter(|entry| !is_ignored(&text, entry, &ignored))
        .filter(|entry| !is_known_place(&text, entry, &places))
        .filter(|entry| !is_rule_disabled(entry, &options))
        .collect())
}

/// Liste les **toponymes / noms propres** persistés.
#[tauri::command]
pub fn list_places(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    Ok(dictionary::load_places(&app)?.into_iter().collect())
}

/// Ajoute un terme aux toponymes (nettoyé) et renvoie la liste mise à jour.
#[tauri::command]
pub fn add_place(app: tauri::AppHandle, word: String) -> Result<Vec<String>, String> {
    let mut places = dictionary::load_places(&app)?;
    let cleaned = word.trim().to_lowercase();
    if !cleaned.is_empty() {
        places.insert(cleaned);
        dictionary::save_places(&app, &places)?;
    }
    Ok(places.into_iter().collect())
}

/// Options d'analyse persistées (défauts si aucune n'a encore été enregistrée).
#[tauri::command]
pub fn get_corrector_options(app: tauri::AppHandle) -> Result<LtOptions, String> {
    options::load_options(&app)
}

/// Remplace les options d'analyse (écriture atomique).
#[tauri::command]
pub fn set_corrector_options(app: tauri::AppHandle, options: LtOptions) -> Result<(), String> {
    crate::corrector::options::save_options(&app, &options)
}

/// Liste les mots ignorés persistés (dictionnaire local).
#[tauri::command]
pub fn list_ignored_words(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    Ok(dictionary::load_ignored(&app)?.into_iter().collect())
}

/// Remplace le dictionnaire local par la liste fournie (nettoyée + triée).
#[tauri::command]
pub fn update_ignored_words(app: tauri::AppHandle, words: Vec<String>) -> Result<(), String> {
    let set: BTreeSet<String> = words
        .into_iter()
        .map(|word| word.trim().to_lowercase())
        .filter(|word| !word.is_empty())
        .collect();
    dictionary::save_ignored(&app, &set)
}
