//! Cœur Rust de **Danoë Studio** — pont de communication frontend ↔ moteur d'export.

mod commands;
mod corrector;
mod epub;
mod export;
pub mod kdp;
mod markdown;
pub mod pdf;
mod project;

use serde_json::Value;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

use project::ProjectPayload;

/// Charge utile d'un événement de progression d'export (nom d'événement `export-progress`).
#[derive(Clone, serde::Serialize)]
struct ExportProgress {
    /// Libellé lisible de l'étape en cours.
    step: String,
    /// Pourcentage global d'avancement (0–100).
    progress: u32,
}

/// Émet une étape de progression vers le frontend (événement `export-progress`).
///
/// Un échec d'émission (frontend non abonné) est ignoré volontairement : il ne doit
/// jamais interrompre la génération du document.
pub(crate) fn emit_progress(app: &tauri::AppHandle, step: &str, progress: u32) {
    let _ = app.emit(
        "export-progress",
        ExportProgress {
            step: step.to_string(),
            progress,
        },
    );
}

/// Génère le manuscrit Word (couverture, liminaires, corps, TOC, glossaire, colophon),
/// émet la progression (`export-progress`), puis ouvre une **boîte de dialogue
/// d'enregistrement** (API Tauri Dialog) et écrit le fichier sur le disque.
///
/// `payload` = **intégralité du JSON projet** (Informations, Mes sources, Organisation…).
#[tauri::command]
async fn generate_docx(
    app: tauri::AppHandle,
    format: String,
    payload: Value,
) -> Result<String, String> {
    println!("──────── generate_docx ────────");
    println!("format : {format}");

    // Phase 1 : seul le format Word est pris en charge.
    let lowered = format.to_lowercase();
    if !lowered.contains("word") && !lowered.contains("docx") {
        return Err(format!(
            "Format « {format} » non pris en charge pour le moment — choisissez « Word (.docx) »."
        ));
    }

    let parsed: ProjectPayload =
        serde_json::from_value(payload).map_err(|e| format!("Payload de projet invalide : {e}"))?;

    // Fichier **intérieur** KDP Print : la couverture n'y figure **jamais** par
    // défaut (elle est produite séparément sous forme de PDF). Le frontend peut
    // demander un rendu « livre complet » (`options.includeCover = true`, contrôle
    // visuel) : le drapeau n'est donc **plus codé en dur** (fin du chemin mort
    // `read_cover`/`cover_spec`).
    let include_cover = parsed.options.include_cover();
    let cover = if include_cover {
        export::read_cover(&app, &parsed)
    } else {
        None
    };

    // Rappel de progression transmis au moteur de génération (événements `export-progress`).
    let progress = |step: &str, percent: u32| emit_progress(&app, step, percent);

    emit_progress(&app, "Préparation de l'export…", 2);
    let bytes = export::build_docx(&parsed, cover.as_deref(), include_cover, &progress)?;
    println!("document généré : {} octets", bytes.len());

    // Boîte de dialogue système pour choisir l'emplacement du fichier.
    // Exécutée hors du thread async (`spawn_blocking`) : l'appel natif est bloquant.
    let file_name = export::default_file_name(&parsed);
    emit_progress(&app, "Enregistrement du manuscrit…", 90);
    let dialog_app = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .set_title("Enregistrer le manuscrit (Word)")
            .set_file_name(&file_name)
            .add_filter("Document Word", &["docx"])
            .blocking_save_file()
    })
    .await
    .map_err(|e| format!("Ouverture de la boîte de dialogue impossible : {e}"))?;

    let Some(file_path) = picked else {
        emit_progress(&app, "Export annulé", 0);
        return Ok("Export annulé.".to_string());
    };
    let path = file_path
        .into_path()
        .map_err(|e| format!("Chemin de sortie invalide : {e}"))?;

    // Écriture effective sur le disque : 100 % (ou erreur explicite remontée au frontend).
    emit_progress(&app, "Écriture sur le disque…", 100);
    std::fs::write(&path, &bytes)
        .map_err(|e| format!("Écriture impossible dans « {} » : {e}", path.display()))?;
    println!("écrit : {}", path.display());

    Ok(format!("Manuscrit enregistré : {}", path.display()))
}

/// Génère un **EPUB 3** minimal (scaffold) puis ouvre une boîte de dialogue d'enregistrement.
#[tauri::command]
async fn generate_epub(app: tauri::AppHandle, payload: Value) -> Result<String, String> {
    println!("──────── generate_epub ────────");

    let parsed: ProjectPayload =
        serde_json::from_value(payload).map_err(|e| format!("Payload de projet invalide : {e}"))?;

    emit_progress(&app, "Préparation de l'EPUB…", 5);
    emit_progress(&app, "Structuration des chapitres…", 40);
    let bytes = epub::build_epub(&parsed)?;
    emit_progress(&app, "Enregistrement de l'EPUB…", 90);

    let file_name = epub::default_epub_name(&parsed);
    let dialog_app = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .set_title("Enregistrer l'ebook (EPUB)")
            .set_file_name(&file_name)
            .add_filter("Ebook", &["epub"])
            .blocking_save_file()
    })
    .await
    .map_err(|e| format!("Ouverture de la boîte de dialogue impossible : {e}"))?;

    let Some(file_path) = picked else {
        emit_progress(&app, "Export annulé", 0);
        return Ok("Export annulé.".to_string());
    };
    let path = file_path
        .into_path()
        .map_err(|e| format!("Chemin de sortie invalide : {e}"))?;

    emit_progress(&app, "Écriture sur le disque…", 100);
    std::fs::write(&path, &bytes)
        .map_err(|e| format!("Écriture impossible dans « {} » : {e}", path.display()))?;

    Ok(format!("Ebook enregistré : {}", path.display()))
}

/// Ouvre l'explorateur natif pour choisir un **dossier** et renvoie son chemin
/// **absolu** (ou `None` si l'utilisateur annule).
///
/// En **Tauri v2**, les API de plugins ne sont **pas** exposées sur
/// `window.__TAURI__` (`withGlobalTauri` n'injecte que l'API cœur) : le frontend
/// passe donc par cette commande, qui s'appuie sur `tauri-plugin-dialog`
/// (`pick_folder` → `dialog.open({ directory: true })`).
#[tauri::command]
async fn pick_directory(app: tauri::AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .set_title("Choisir un dossier")
        .blocking_pick_folder()
        .and_then(|path| path.into_path().ok())
        .map(|path| path.to_string_lossy().into_owned())
}

/// Ouvre un sélecteur de **fichier** natif, puis **copie** le fichier choisi
/// dans le dossier « Mes sources » (renommage en `_copie`, `_copie2`, … en cas
/// de conflit). Renvoie le **nom** du fichier importé (`None` si annulé).
///
/// En **Tauri v2**, les API `dialog`/`fs` ne sont pas exposées sur
/// `window.__TAURI__` : cette commande remplace l'orchestration frontend
/// (`dialog.open` + `fs.copyFile`) utilisée en Tauri v1.
#[tauri::command]
async fn import_file_into_folder(
    app: tauri::AppHandle,
    directory: String,
) -> Result<Option<String>, String> {
    println!("──────── import_file_into_folder ────────");
    println!("destination : {directory}");

    let dir = std::path::PathBuf::from(&directory);
    if !dir.is_dir() {
        return Err(format!("Dossier de destination introuvable : {directory}"));
    }

    // Sélecteur natif (bloquant) exécuté hors du thread async.
    let dialog_app = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .set_title("Importer un fichier dans le projet")
            .blocking_pick_file()
    })
    .await
    .map_err(|e| format!("Ouverture de la boîte de dialogue impossible : {e}"))?;

    let Some(file_path) = picked else {
        return Ok(None);
    };
    let source = file_path
        .into_path()
        .map_err(|e| format!("Chemin source invalide : {e}"))?;

    let base = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "fichier".to_string());

    let target = free_destination(&dir, &base);
    std::fs::copy(&source, &target).map_err(|e| {
        format!(
            "Copie impossible de « {} » vers « {} » : {e}",
            source.display(),
            target.display()
        )
    })?;

    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or(base);
    println!("importé : {name}");
    Ok(Some(name))
}

/// Construit un chemin de destination **libre** dans `dir` (renommage
/// `_copie`, `_copie2`, … lorsqu'un fichier de même nom existe déjà).
fn free_destination(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let path = std::path::Path::new(name);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_string());
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let mut index = 1;
    loop {
        let suffix = if index == 1 {
            "_copie".to_string()
        } else {
            format!("_copie{index}")
        };
        let candidate = dir.join(format!("{stem}{suffix}{ext}"));
        if !candidate.exists() {
            return candidate;
        }
        index += 1;
    }
}

/// Liste les **fichiers** (et non les sous-dossiers) d'un dossier donné, et
/// renvoie leurs **noms** triés (ordre alphabétique insensible à la casse).
///
/// En **Tauri v2**, le plugin `fs` n'est pas exposé sur `window.__TAURI__`
/// (`withGlobalTauri` n'injecte que l'API cœur) : le frontend ne peut donc pas
/// énumérer un dossier via `fs.readDir`. Cette commande comble ce manque
/// (complément direct de `pick_directory`), afin de peupler la liste des
/// chapitres/images de l'onglet « Organisation ».
#[tauri::command]
async fn list_directory_files(path: String) -> Result<Vec<String>, String> {
    println!("──────── list_directory_files ────────");
    println!("dossier : {path}");

    let dir = std::path::Path::new(&path);
    if !dir.is_dir() {
        return Err(format!("Dossier introuvable : {path}"));
    }

    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Lecture du dossier impossible « {path} » : {e}"))?;

    let mut names: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        // Seuls les fichiers sont conservés (les sous-dossiers sont ignorés).
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort_by_key(|name| name.to_lowercase());
    println!("{} fichier(s) détecté(s)", names.len());
    Ok(names)
}

/// Supprime **récursivement** le contenu d'un dossier (sans supprimer le dossier
/// lui-même). Les erreurs d'entrée sont journalisées et ignorées : la purge du
/// cache ne doit jamais empêcher la fermeture de l'application.
fn purge_directory(dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let result = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        if let Err(error) = result {
            println!("purge impossible « {} » : {error}", path.display());
        }
    }
}

/// Sous-dossier de **cache volatile** de l'application.
///
/// Il est **strictement distinct** des données utilisateur : le purger ne touche
/// donc jamais le projet ni le profil WebView (donc `localStorage`).
fn volatile_cache_dir(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    app.path()
        .app_cache_dir()
        .ok()
        .map(|dir| dir.join("volatile"))
}

/// Chemin du fichier de projet **persistant** (données utilisateur).
///
/// Stocké dans `app_data_dir()` (`%APPDATA%\<identifiant>` sous Windows, en
/// roaming) : il **survit aux mises à jour** de l'exécutable et n'est **jamais**
/// touché par une purge de cache.
fn project_data_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
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
    Ok(dir.join("project.danoe"))
}

/// Lit le projet persistant (données utilisateur). `None` si aucun projet enregistré.
#[tauri::command]
async fn read_project_data(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let path = project_data_path(&app)?;
    if !path.is_file() {
        println!("aucun projet persistant : {}", path.display());
        return Ok(None);
    }
    println!("lecture du projet persistant : {}", path.display());
    std::fs::read_to_string(&path)
        .map(Some)
        .map_err(|e| format!("Lecture du projet impossible « {} » : {e}", path.display()))
}

/// Écrit le projet persistant (données utilisateur) — survit aux mises à jour.
#[tauri::command]
async fn write_project_data(app: tauri::AppHandle, contents: String) -> Result<(), String> {
    let path = project_data_path(&app)?;
    std::fs::write(&path, contents)
        .map_err(|e| format!("Écriture du projet impossible « {} » : {e}", path.display()))
}

/// Purge le **cache volatile** (fichiers temporaires) puis ferme proprement
/// l'application (`exit(0)`).
///
/// ⚠️ **Aucune donnée utilisateur n'est touchée** : ni `app_data_dir()`
/// (projet persistant), ni `app_local_data_dir()` / le profil WebView
/// (qui contient `localStorage`). Seul le sous-dossier `volatile` est vidé.
#[tauri::command]
async fn clear_cache_and_exit(app: tauri::AppHandle) -> Result<(), String> {
    println!("──────── clear_cache_and_exit ────────");

    if let Some(volatile) = volatile_cache_dir(&app) {
        if volatile.is_dir() {
            println!("purge du cache volatil : {}", volatile.display());
            purge_directory(&volatile);
        } else {
            println!("aucun cache volatil à purger : {}", volatile.display());
        }
    }

    // Fermeture native de l'application.
    app.exit(0);
    Ok(())
}

/// Point d'entrée Tauri de l'application de bureau.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // Fermeture de fenêtre **interceptée** : l'UI joue d'abord la cinématique
        // 3D (événement `app-close-requested`), puis appelle `finalize_exit`.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.emit("app-close-requested", ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            generate_docx,
            generate_epub,
            commands::export_pdf,
            commands::render_pdf,
            corrector::analyze_chapter,
            corrector::list_ignored_words,
            corrector::update_ignored_words,
            corrector::get_corrector_options,
            corrector::set_corrector_options,
            corrector::list_places,
            corrector::add_place,
            commands::read_chapter_file,
            commands::write_chapter_file,
            commands::finalize_exit,
            pick_directory,
            list_directory_files,
            import_file_into_folder,
            read_project_data,
            write_project_data,
            clear_cache_and_exit
        ])
        .build(tauri::generate_context!())
        .expect("erreur au lancement de Danoë Studio");

    // **Nettoyage garanti & synchrone** : la purge du cache volatil s'exécute sur
    // `RunEvent::Exit`, quel que soit le chemin de fermeture (bouton de l'app,
    // croix de la fenêtre, Alt+F4, `app.exit()`). Elle s'exécute avant l'arrêt du
    // thread principal.
    //
    // NB : le log natif WebView2 « Failed to unregister class Chrome_WidgetWin_0.
    // Error = 1412 » est un **bruit upstream** (race interne à Chromium lors de la
    // destruction des fenêtres) : il n'affecte ni le code de sortie (0) ni la
    // terminaison du processus (aucun processus fantôme). Aucune correction requise.
    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            if let Some(volatile) = volatile_cache_dir(app_handle) {
                if volatile.is_dir() {
                    println!(
                        "purge du cache volatil (RunEvent::Exit) : {}",
                        volatile.display()
                    );
                    purge_directory(&volatile);
                }
            }
        }
    });
}
