//! Dictionnaire local (**mots ignorés** + **toponymes / noms propres**) —
//! persistance JSON sécurisée dans le répertoire de données de l'application
//! (`app_data_dir`, survit aux mises à jour et n'est jamais purgé).

use std::collections::BTreeSet;
use std::path::PathBuf;

use tauri::Manager;

/// Nom du fichier de mots ignorés (données utilisateur).
const DICTIONARY_FILE: &str = "ignored_words.json";
/// Nom du fichier de **toponymes / noms propres** (données utilisateur).
const PLACES_FILE: &str = "places.json";

/// Chemin d'un fichier de dictionnaire ; le dossier de données est créé si nécessaire.
fn dictionary_path(app: &tauri::AppHandle, file: &str) -> Result<PathBuf, String> {
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
    Ok(dir.join(file))
}

/// Lecture d'un ensemble de termes (`BTreeSet` vide si le fichier n'existe pas).
fn load_set(app: &tauri::AppHandle, file: &str) -> Result<BTreeSet<String>, String> {
    let path = dictionary_path(app, file)?;
    if !path.is_file() {
        return Ok(BTreeSet::new());
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("Lecture du dictionnaire impossible « {} » : {e}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|e| format!("Dictionnaire local illisible « {} » : {e}", path.display()))
}

/// Écrit un ensemble (JSON trié) via un **fichier temporaire + renommage**
/// atomique : une interruption ne corrompt jamais le fichier existant.
fn save_set(app: &tauri::AppHandle, file: &str, words: &BTreeSet<String>) -> Result<(), String> {
    let path = dictionary_path(app, file)?;
    let json = serde_json::to_string_pretty(words)
        .map_err(|e| format!("Sérialisation du dictionnaire impossible : {e}"))?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, json).map_err(|e| {
        format!(
            "Écriture du dictionnaire impossible « {} » : {e}",
            temporary.display()
        )
    })?;
    std::fs::rename(&temporary, &path).map_err(|e| {
        format!(
            "Enregistrement du dictionnaire impossible « {} » : {e}",
            path.display()
        )
    })
}

/// Charge les **mots ignorés**.
pub fn load_ignored(app: &tauri::AppHandle) -> Result<BTreeSet<String>, String> {
    load_set(app, DICTIONARY_FILE)
}

/// Écrit les **mots ignorés**.
pub fn save_ignored(app: &tauri::AppHandle, words: &BTreeSet<String>) -> Result<(), String> {
    save_set(app, DICTIONARY_FILE, words)
}

/// Charge les **toponymes / noms propres** connus.
pub fn load_places(app: &tauri::AppHandle) -> Result<BTreeSet<String>, String> {
    load_set(app, PLACES_FILE)
}

/// Écrit les **toponymes / noms propres** connus.
pub fn save_places(app: &tauri::AppHandle, words: &BTreeSet<String>) -> Result<(), String> {
    save_set(app, PLACES_FILE, words)
}
