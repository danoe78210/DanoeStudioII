//! Options du correcteur LanguageTool — persistance JSON dans `app_data_dir`
//! (données utilisateur, jamais purgées).

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::Manager;

/// Nom du fichier d'options (données utilisateur).
const OPTIONS_FILE: &str = "corrector_options.json";

/// Options d'analyse du correcteur.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LtOptions {
    /// Code de langue / région (`fr`, `fr-FR`, `fr-BE`, `fr-CA`, `fr-CH`).
    pub language: String,
    /// Mode « pointilleux » (style, typographie, sémantique avancée).
    pub picky: bool,
    /// IDs de règles désactivées.
    pub disabled_rules: BTreeSet<String>,
}

impl Default for LtOptions {
    fn default() -> Self {
        Self {
            language: "fr".to_string(),
            picky: false,
            disabled_rules: BTreeSet::new(),
        }
    }
}

/// Chemin du fichier d'options ; le dossier de données est créé si nécessaire.
fn options_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Dossier de données utilisateur introuvable : {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| {
        format!(
            "Création du dossier de données impossible « {} » : {e}",
            dir.display()
        )
    })?;
    Ok(dir.join(OPTIONS_FILE))
}

/// Charge les options (défauts si le fichier n'existe pas encore).
pub fn load_options(app: &tauri::AppHandle) -> Result<LtOptions, String> {
    let path = options_path(app)?;
    if !path.is_file() {
        return Ok(LtOptions::default());
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "Lecture des options du correcteur impossible « {} » : {e}",
            path.display()
        )
    })?;
    serde_json::from_str(&raw).map_err(|e| {
        format!(
            "Options du correcteur illisibles « {} » : {e}",
            path.display()
        )
    })
}

/// Écrit les options (JSON trié) via un **fichier temporaire + renommage** atomique.
pub fn save_options(app: &tauri::AppHandle, options: &LtOptions) -> Result<(), String> {
    let path = options_path(app)?;
    let json = serde_json::to_string_pretty(options)
        .map_err(|e| format!("Sérialisation des options impossible : {e}"))?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, json).map_err(|e| {
        format!(
            "Écriture des options impossible « {} » : {e}",
            temporary.display()
        )
    })?;
    std::fs::rename(&temporary, &path).map_err(|e| {
        format!(
            "Enregistrement des options impossible « {} » : {e}",
            path.display()
        )
    })
}
