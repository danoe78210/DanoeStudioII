//! Commandes Tauri d'export — dont le **PDF prêt-à-imprimer** (Typst).
//!
//! Orchestration : parsing complet du projet → AST Markdown → balisage Typst
//! (`pdf::generate`) → compilation (`pdf::compile_to_pdf_with_root`) → boîte de
//! dialogue native d'enregistrement → écriture disque.
//!
//! **Tolérance zéro** : toute erreur (source introuvable, syntaxe Typst, I/O)
//! est propagée en `Result::Err` vers le frontend.

use serde_json::Value;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

use crate::kdp::KdpPageSpec;
use crate::markdown::{
    extract_definitions, number_refs, parse_blocks, strip_frontmatter, strip_refs, Block,
    NoteDefinition,
};
use crate::pdf::{
    self, generate, GlossaryChapter, GlossaryEntry, PdfDoc, Typography,
};
use crate::project::{estimate_page_count, OrganizationNode, ProjectPayload};

/// Nom de fichier de sortie proposé (slug du titre, extension `.pdf`).
pub fn default_pdf_name(payload: &ProjectPayload) -> String {
    let slug: String = payload
        .metadata
        .book_title
        .trim()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let slug = slug.trim_matches('_').to_string();
    if slug.is_empty() {
        "manuscrit.pdf".to_string()
    } else {
        format!("{slug}.pdf")
    }
}

/// Construit l'AST complet du manuscrit depuis l'arborescence du projet.
///
/// **Post-traitement aligné sur l'export Word** (parité d'architecture) :
/// - `extract_definitions` retire les lignes `[^label]: …` du corps ;
/// - si une page spéciale **Glossaire** est présente, les renvois `[^label]` sont
///   conservés (numérotés plus tard par le générateur) et les définitions sont
///   agrégées dans `glossary` ;
/// - sinon les renvois sont **retirés** (`strip_refs`).
///
/// Chaque nœud `image` devient un `Block::Image` ; chaque nœud textuel insère un
/// `Heading1` (titre du nœud) suivi des blocs de son fichier source. Une source
/// référencée mais **introuvable** est une erreur explicite (jamais silencieuse).
fn build_blocks(
    payload: &ProjectPayload,
    glossary_present: bool,
    glossary: &mut Vec<GlossaryChapter>,
) -> Result<Vec<Block>, String> {
    let root = payload.directories.sources.as_deref();
    let mut blocks = Vec::new();
    collect_blocks(
        &payload.organization,
        root,
        None,
        glossary_present,
        glossary,
        &mut blocks,
    )?;
    Ok(blocks)
}

/// Parcours récursif de l'arborescence → blocs Markdown (+ agrégation glossaire).
fn collect_blocks(
    nodes: &[OrganizationNode],
    root: Option<&str>,
    act: Option<&str>,
    glossary_present: bool,
    glossary: &mut Vec<GlossaryChapter>,
    out: &mut Vec<Block>,
) -> Result<(), String> {
    for node in nodes {
        match node.kind.as_str() {
            "image" => match node.source_file_name.as_deref().filter(|n| !n.is_empty()) {
                Some(name) => out.push(Block::Image {
                    target: name.to_string(),
                }),
                None => {
                    return Err(format!(
                        "Illustration « {} » sans fichier lié (onglet Organisation).",
                        node.display_name.as_deref().unwrap_or(&node.id)
                    ))
                }
            },
            "chapter" | "act" | "special" => {
                let title = node
                    .display_name
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                    .map(str::to_string);
                if let Some(title) = &title {
                    out.push(Block::Heading1(title.clone()));
                }
                if let Some(name) = node.source_file_name.as_deref().filter(|n| !n.is_empty()) {
                    let text = read_source(root, name)
                        .ok_or_else(|| format!("Fichier source introuvable : « {name} »."))?;
                    let body = strip_frontmatter(&text);
                    let (body, definitions) = extract_definitions(&body);
                    if glossary_present {
                        let mut parsed = parse_blocks(&body);
                        // Le titre du nœud (Organisation) fait office de titre de
                        // chapitre : on évite un doublon si la source commence
                        // elle aussi par un titre `#` (parité avec l'export Word).
                        if title.is_some() && matches!(parsed.first(), Some(Block::Heading1(_))) {
                            parsed.remove(0);
                        }
                        // Numérotation par chapitre, alignée sur le générateur
                        // (`render_blocks` réinitialise à chaque `Heading1`).
                        let mut numbers: Vec<(String, u32)> = Vec::new();
                        for block in &parsed {
                            if let Block::Paragraph(text) = block {
                                let _ = number_refs(text, &mut numbers);
                            }
                        }
                        let entries = definitions_to_entries(&definitions, &mut numbers);
                        if !entries.is_empty() {
                            glossary.push(GlossaryChapter {
                                label: glossary_group_label(act, title.as_deref()),
                                entries,
                            });
                        }
                        out.extend(parsed);
                    } else {
                        let mut parsed = parse_blocks(&strip_refs(&body));
                        if title.is_some() && matches!(parsed.first(), Some(Block::Heading1(_))) {
                            parsed.remove(0);
                        }
                        out.extend(parsed);
                    }
                }
            }
            _ => {}
        }
        // Contexte d'acte propagé aux enfants (pour le libellé de glossaire).
        let next_act = if node.kind == "act" {
            node.display_name
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .or(act)
        } else {
            act
        };
        collect_blocks(
            &node.children,
            root,
            next_act,
            glossary_present,
            glossary,
            out,
        )?;
    }
    Ok(())
}

/// La page spéciale « Glossaire » est-elle présente dans l'arborescence ?
fn has_glossary(nodes: &[OrganizationNode]) -> bool {
    nodes
        .iter()
        .any(|node| node.role.as_deref() == Some("glossary") || has_glossary(&node.children))
}

/// Libellé de regroupement du glossaire (« Acte 2, chapitre 7 »).
fn glossary_group_label(act: Option<&str>, chapter: Option<&str>) -> String {
    let chapter = chapter
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("Chapitre");
    match act.filter(|a| !a.trim().is_empty()) {
        Some(act) => format!("{act}, {}", chapter.to_lowercase()),
        None => chapter.to_string(),
    }
}

/// Convertit les définitions en entrées numérotées (numéros attribués par les
/// renvois du corps ; une définition non appelée reçoit le numéro suivant).
fn definitions_to_entries(
    definitions: &[NoteDefinition],
    numbers: &mut Vec<(String, u32)>,
) -> Vec<GlossaryEntry> {
    let mut entries: Vec<GlossaryEntry> = definitions
        .iter()
        .map(|definition| {
            let number = match numbers.iter().find(|(label, _)| *label == definition.label) {
                Some((_, n)) => *n,
                None => {
                    let n = numbers.len() as u32 + 1;
                    numbers.push((definition.label.clone(), n));
                    n
                }
            };
            let (term, text) = match split_term(&definition.text) {
                Some((term, rest)) => (term.to_string(), rest.to_string()),
                None => (label_to_term(&definition.label), definition.text.clone()),
            };
            GlossaryEntry {
                number,
                term,
                definition: text,
            }
        })
        .collect();
    entries.sort_by_key(|entry| entry.number);
    entries
}

/// Sépare le « terme » (avant les deux-points) de sa définition, si non formaté.
fn split_term(definition: &str) -> Option<(&str, &str)> {
    let position = definition.find(':')?;
    let prefix = definition[..position].trim();
    if prefix.is_empty() || prefix.len() > 40 || prefix.contains('*') {
        return None;
    }
    Some((prefix, definition[position + 1..].trim_start()))
}

/// Dérive un terme lisible depuis le libellé `[^libellé]` (1ʳᵉ lettre capitalisée).
fn label_to_term(label: &str) -> String {
    let mut chars = label.trim().chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Lit une source texte depuis le dossier « Mes sources » (insensible à la casse).
fn read_source(root: Option<&str>, name: &str) -> Option<String> {
    let dir = std::path::Path::new(root?);
    let direct = dir.join(name);
    if let Ok(text) = std::fs::read_to_string(&direct) {
        return Some(text);
    }
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        let matches = path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case(name))
            .unwrap_or(false);
        if matches {
            if let Ok(text) = std::fs::read_to_string(&path) {
                return Some(text);
            }
        }
    }
    None
}

/// Rend les octets PDF du projet (sans dialogue ni écriture) — cœur testable.
pub fn render_pdf_bytes(payload: &ProjectPayload) -> Result<Vec<u8>, String> {
    let spec = KdpPageSpec::resolve(
        &payload.layout_config.trim_size,
        estimate_page_count(payload),
        false,
    );
    // **Règle du livre** : glossaire présent → notes numérotées + glossaire final.
    let glossary_present = has_glossary(&payload.organization);
    let mut glossary: Vec<GlossaryChapter> = Vec::new();
    let blocks = build_blocks(payload, glossary_present, &mut glossary)?;

    // Câblage typographique UI → PDF (le format de coupe est déjà géré par `kdp`).
    let layout = &payload.layout_config;
    let typography = Typography {
        body_font: layout.body_font.as_str(),
        body_size: layout.body_size,
        line_spacing: layout.line_spacing,
        justify: layout.text_alignment != "left",
        drop_cap: layout.drop_cap,
        chapter_font: layout.resolve_font(&layout.chapter_title_font),
        chapter_size: layout.chapter_title_size,
        subtitle_font: layout.resolve_font(&layout.subtitle_font),
        subtitle_size: layout.subtitle_size,
    };
    let doc = PdfDoc {
        title: &payload.metadata.book_title,
        author: &payload.metadata.author_name,
        blocks: &blocks,
        typography: &typography,
        glossary_present,
        glossary: &glossary,
    };
    let source = generate(&spec, &doc);
    let root = payload
        .directories
        .sources
        .as_deref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    pdf::compile_to_pdf_with_root(&source, &root)
}

/// Exporte le manuscrit en **PDF prêt-à-imprimer KDP** (moteur Typst).
#[tauri::command]
pub async fn export_pdf(app: tauri::AppHandle, payload: Value) -> Result<String, String> {
    println!("──────── export_pdf ────────");
    let parsed: ProjectPayload =
        serde_json::from_value(payload).map_err(|e| format!("Payload de projet invalide : {e}"))?;

    crate::emit_progress(&app, "Structuration du manuscrit…", 10);
    let bytes = render_pdf_bytes(&parsed)?;
    println!("PDF généré : {} octets", bytes.len());

    // Boîte de dialogue native (même logique que l'export Word), hors thread async.
    let file_name = default_pdf_name(&parsed);
    crate::emit_progress(&app, "Enregistrement du PDF…", 90);
    let dialog_app = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .set_title("Enregistrer le PDF prêt-à-imprimer")
            .set_file_name(&file_name)
            .add_filter("PDF", &["pdf"])
            .blocking_save_file()
    })
    .await
    .map_err(|e| format!("Ouverture de la boîte de dialogue impossible : {e}"))?;

    let Some(file_path) = picked else {
        crate::emit_progress(&app, "Export annulé", 0);
        return Ok("Export annulé.".to_string());
    };
    let path = file_path
        .into_path()
        .map_err(|e| format!("Chemin de sortie invalide : {e}"))?;

    crate::emit_progress(&app, "Écriture sur le disque…", 100);
    std::fs::write(&path, &bytes)
        .map_err(|e| format!("Écriture impossible dans « {} » : {e}", path.display()))?;
    println!("écrit : {}", path.display());

    crate::emit_progress(&app, "Terminé", 100);
    Ok(format!("PDF enregistré : {}", path.display()))
}

/// Rend le PDF prêt-à-imprimer **en mémoire** (aucun dialogue, aucune écriture
/// disque) — support de l'**aperçu interactif** (flipbook) du frontend.
///
/// Même pipeline que [`export_pdf`] (AST → balisage Typst → compilation), mais
/// les octets sont renvoyés tels quels via [`tauri::ipc::Response`] : le
/// frontend reçoit un `ArrayBuffer` brut, sans passer par une sérialisation JSON
/// (coût prohibitif pour un manuscrit complet).
#[tauri::command]
pub async fn render_pdf(payload: Value) -> Result<tauri::ipc::Response, String> {
    println!("──────── render_pdf (aperçu mémoire) ────────");
    let parsed: ProjectPayload =
        serde_json::from_value(payload).map_err(|e| format!("Payload de projet invalide : {e}"))?;
    let bytes = render_pdf_bytes(&parsed)?;
    println!("PDF aperçu généré : {} octets", bytes.len());
    Ok(tauri::ipc::Response::new(bytes))
}

/// Résout le chemin d'un fichier de « Mes sources » : nom direct, puis
/// **recherche récursive insensible à la casse** (couvre les sous-dossiers).
fn resolve_source_file(root: &std::path::Path, filename: &str) -> Option<std::path::PathBuf> {
    let candidate = std::path::Path::new(filename);
    if candidate.is_absolute() && candidate.is_file() {
        return Some(candidate.to_path_buf());
    }
    let joined = root.join(filename);
    if joined.is_file() {
        return Some(joined);
    }
    find_file_case_insensitive(root, filename)
}

/// Recherche récursive d'un fichier par son **nom** (insensible à la casse).
fn find_file_case_insensitive(
    dir: &std::path::Path,
    filename: &str,
) -> Option<std::path::PathBuf> {
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file_case_insensitive(&path, filename) {
                return Some(found);
            }
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case(filename))
        {
            return Some(path);
        }
    }
    None
}

/// Charge le payload du **projet persistant** (`app_data_dir/project.danoe`).
fn load_persisted_payload(app: &tauri::AppHandle) -> Result<ProjectPayload, String> {
    let path = crate::project_data_path(app)?;
    if !path.is_file() {
        return Err("Aucun projet enregistré : fichier de projet introuvable.".to_string());
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("Lecture du projet impossible « {} » : {e}", path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("Projet persistant illisible : {e}"))
}

/// Lit le **texte brut** d'un chapitre (fichier de « Mes sources ») pour le
/// Correcteur. Le dossier est celui du **projet persistant** (aucun chemin
/// arbitraire n'est exposé). Une source introuvable est une erreur explicite.
#[tauri::command]
pub fn read_chapter_file(app: tauri::AppHandle, filename: String) -> Result<String, String> {
    println!("──────── read_chapter_file ────────");
    println!("fichier : {filename}");
    let payload = load_persisted_payload(&app)?;
    let root = payload
        .directories
        .sources
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Dossier des sources non configuré (onglet Réglages).".to_string())?;
    let path = resolve_source_file(std::path::Path::new(root), &filename)
        .ok_or_else(|| format!("Fichier source introuvable : {filename}"))?;
    std::fs::read_to_string(&path)
        .map_err(|e| format!("Lecture impossible « {} » : {e}", path.display()))
}

/// Écrit le **texte brut** d'un chapitre (fichier de « Mes sources ») via une
/// **écriture atomique** (`.tmp` puis `rename`) : aucune corruption possible si
/// l'application est fermée pendant la sauvegarde.
#[tauri::command]
pub async fn write_chapter_file(
    app: tauri::AppHandle,
    filename: String,
    content: String,
) -> Result<(), String> {
    println!("──────── write_chapter_file ────────");
    println!("fichier : {filename} ({} octets)", content.len());
    let payload = load_persisted_payload(&app)?;
    let root = payload
        .directories
        .sources
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Dossier des sources non configuré (onglet Réglages).".to_string())?;
    let path = resolve_source_file(std::path::Path::new(root), &filename)
        .ok_or_else(|| format!("Fichier source introuvable : {filename}"))?;

    // Fichier temporaire **dans le même dossier** (rename atomique garanti).
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("chapter");
    let temporary = path.with_file_name(format!("{name}.tmp"));
    std::fs::write(&temporary, &content)
        .map_err(|e| format!("Écriture impossible « {} » : {e}", temporary.display()))?;
    std::fs::rename(&temporary, &path).map_err(|e| {
        format!(
            "Enregistrement impossible « {} » : {e}",
            path.display()
        )
    })
}

/// Ferme **proprement** l'application, à l'issue de l'animation 3D de fermeture
/// du livre : la fenêtre principale est **détruite** (libération ordonnée des
/// threads Chromium/WebView2 → évite l'erreur Win32 1412), puis `exit(0)`.
/// Le cache volatil est purgé automatiquement sur `RunEvent::Exit`.
#[tauri::command]
pub fn finalize_exit(app: tauri::AppHandle) {
    println!("──────── finalize_exit ────────");
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.destroy();
    }
    app.exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Encode un PNG valide (2×2) via la crate `image`.
    fn png_bytes() -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 80, 160, 255]));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut out, image::ImageFormat::Png)
            .expect("encodage PNG");
        out.into_inner()
    }

    #[test]
    fn default_pdf_name_slugifies_title() {
        let payload: ProjectPayload = serde_json::from_value(json!({
            "metadata": { "bookTitle": "La Nuit des Cristaux" }
        }))
        .unwrap();
        assert_eq!(default_pdf_name(&payload), "La_Nuit_des_Cristaux.pdf");
    }

    #[test]
    fn render_pdf_bytes_from_payload_with_chapter_and_image() {
        let dir = std::env::temp_dir().join(format!("danoe_pdf_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("chapitre-1.md"),
            "# Chapitre 1\n\nUn **cristal** de *lumière* vibre.\n",
        )
        .unwrap();
        std::fs::write(dir.join("scene.png"), png_bytes()).unwrap();

        let payload: ProjectPayload = serde_json::from_value(json!({
            "metadata": { "bookTitle": "Nunael", "authorName": "Danoë" },
            "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
            "directories": { "sources": dir.to_string_lossy() },
            "organization": [
                { "type": "act", "displayName": "Acte I", "children": [
                    { "type": "chapter", "displayName": "Chapitre 1", "sourceFileName": "chapitre-1.md" },
                    { "type": "image", "displayName": "Scène", "sourceFileName": "scene.png" }
                ]}
            ]
        }))
        .unwrap();

        let bytes = render_pdf_bytes(&payload).expect("compilation PDF depuis le payload");
        assert!(bytes.starts_with(b"%PDF-"), "en-tête PDF");
        assert!(bytes.windows(5).any(|w| w == b"%%EOF"), "%%EOF");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_source_file_is_an_explicit_error() {
        let dir = std::env::temp_dir().join(format!("danoe_pdf_missing_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let payload: ProjectPayload = serde_json::from_value(json!({
            "metadata": { "bookTitle": "X" },
            "directories": { "sources": dir.to_string_lossy() },
            "organization": [
                { "type": "chapter", "displayName": "C", "sourceFileName": "inexistant-xyz.md" }
            ]
        }))
        .unwrap();

        let err = render_pdf_bytes(&payload).unwrap_err();
        assert!(err.contains("introuvable"), "erreur explicite : {err}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Voie complète **notes + glossaire** : les définitions `[^…]` ne doivent
    /// plus fuiter dans le corps et le PDF (glossaire final + appels en exposant)
    /// doit se compiler sans erreur Typst.
    #[test]
    fn render_pdf_bytes_with_glossary_and_notes_compiles() {
        let dir = std::env::temp_dir().join(format!("danoe_pdf_notes_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("chapitre-1.md"),
            "Le neume[^neume] vibre.\n\n[^neume]: Neumes : premiers symboles.\n",
        )
        .unwrap();

        let payload: ProjectPayload = serde_json::from_value(json!({
            "metadata": { "bookTitle": "Nunael", "authorName": "Danoë" },
            "layoutConfig": {
                "trimSize": "6x9", "bodyFont": "Garamond",
                "chapterTitleFont": "body", "chapterTitleSize": 18,
                "subtitleFont": "body", "subtitleSize": 12
            },
            "directories": { "sources": dir.to_string_lossy() },
            "organization": [
                { "type": "act", "displayName": "Acte I", "children": [
                    { "type": "chapter", "displayName": "Chapitre 1", "sourceFileName": "chapitre-1.md" }
                ]},
                { "type": "special", "role": "glossary", "displayName": "Glossaire" }
            ]
        }))
        .unwrap();

        let bytes = render_pdf_bytes(&payload).expect("compilation PDF avec glossaire");
        assert!(bytes.starts_with(b"%PDF-"), "en-tête PDF");
        assert!(bytes.windows(5).any(|w| w == b"%%EOF"), "%%EOF");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Lit une partie XML d'un paquet `.docx` (test de parité).
    fn read_docx_part(bytes: &[u8], name: &str) -> String {
        use std::io::Read;
        let mut archive =
            zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).expect("zip lisible");
        let mut file = archive.by_name(name).expect("entrée présente");
        let mut content = String::new();
        file.read_to_string(&mut content).expect("lecture utf-8");
        content
    }

    /// **Preuve de parité** : le **même AST post-traité** produit, dans les deux
    /// pipelines (Word et PDF), un glossaire à **numérotation stricte**, **terme en
    /// gras** et **séparateur de chapitres** (`* * *`).
    #[test]
    fn test_glossary_parity() {
        use std::fs;
        let dir = std::env::temp_dir().join(format!("danoe_parity_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("c1.md"),
            "La Nahe[^nahe] gronde.\n\n[^nahe]: Rivière qui rejoint le Rhin.\n",
        )
        .unwrap();
        fs::write(
            dir.join("c2.md"),
            "La muraille[^mur] tient.\n\n[^mur]: Enceinte de pierre.\n",
        )
        .unwrap();

        let payload: ProjectPayload = serde_json::from_value(json!({
            "metadata": { "bookTitle": "Livre", "authorName": "A", "year": "2026" },
            "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
            "directories": { "sources": dir.to_string_lossy() },
            "organization": [
                { "type": "act", "displayName": "Acte I", "children": [
                    { "type": "chapter", "displayName": "Chapitre 1", "sourceFileName": "c1.md" },
                    { "type": "chapter", "displayName": "Chapitre 2", "sourceFileName": "c2.md" }
                ]},
                { "type": "special", "role": "glossary", "displayName": "Glossaire" }
            ]
        }))
        .unwrap();

        // ── Pipeline PDF : source Typst générée depuis l'AST post-traité ──
        let spec = KdpPageSpec::resolve("6x9", estimate_page_count(&payload), false);
        let mut glossary: Vec<GlossaryChapter> = Vec::new();
        let blocks = build_blocks(&payload, true, &mut glossary).unwrap();
        let layout = &payload.layout_config;
        let typography = Typography {
            body_font: layout.body_font.as_str(),
            body_size: layout.body_size,
            line_spacing: layout.line_spacing,
            justify: true,
            drop_cap: layout.drop_cap,
            chapter_font: layout.resolve_font(&layout.chapter_title_font),
            chapter_size: layout.chapter_title_size,
            subtitle_font: layout.resolve_font(&layout.subtitle_font),
            subtitle_size: layout.subtitle_size,
        };
        let doc = PdfDoc {
            title: "Livre",
            author: "A",
            blocks: &blocks,
            typography: &typography,
            glossary_present: true,
            glossary: &glossary,
        };
        let src = generate(&spec, &doc);

        // Appels de notes en exposant (jamais en texte brut).
        assert!(src.contains("#super["), "appels de notes en exposant :\n{src}");
        // Terme en gras + « : » ; numérotation stricte par chapitre.
        assert!(
            src.contains("#super[1] #text(\"Nahe\", weight: \"bold\") : "),
            "glossaire PDF « 1 Nahe : … » :\n{src}"
        );
        assert!(
            src.contains("#super[1] #text(\"Mur\", weight: \"bold\") : "),
            "glossaire PDF « 1 Mur : … » :\n{src}"
        );
        let glossary_src = src
            .split("#heading(level: 1)[Glossaire]")
            .nth(1)
            .expect("section glossaire présente");
        assert_eq!(
            glossary_src.matches("\\* \\* \\*").count(),
            1,
            "un séparateur entre les groupes de chapitres"
        );

        // ── Pipeline Word : XML du document ──
        let bytes = crate::export::build_docx(&payload, None, false, &|_, _| {})
            .expect("génération DOCX");
        let document = read_docx_part(&bytes, "word/document.xml");

        assert!(document.contains("Nahe :"), "terme « Nahe : » DOCX");
        assert!(document.contains("Mur :"), "terme « Mur : » DOCX");
        // Numérotation/terme en exposant strictement dans le glossaire.
        assert!(
            document.contains("superscript"),
            "appel/terme en exposant DOCX"
        );
        let glossary_docx = document
            .split("GLOSSAIRE")
            .nth(1)
            .expect("section glossaire DOCX");
        assert_eq!(
            glossary_docx.matches("* * *").count(),
            1,
            "un séparateur entre les groupes (DOCX)"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Concatène le texte de toutes les entrées d'un ZIP dont le nom finit par `suffix`.
    fn read_zip_by_suffix(bytes: &[u8], suffix: &str) -> String {
        use std::io::Read;
        let mut archive =
            zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).expect("zip lisible");
        let names: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
            .collect();
        let mut out = String::new();
        for name in names {
            if name.ends_with(suffix) {
                if let Ok(mut file) = archive.by_name(&name) {
                    let mut content = String::new();
                    let _ = file.read_to_string(&mut content);
                    out.push_str(&content);
                }
            }
        }
        out
    }

    /// **Preuve de parité — aération structurelle** : un même AST (Titre 1, Image,
    /// Titre 2) génère les sauts de page / centrages attendus dans les **trois**
    /// formats (PDF, DOCX, ePUB).
    #[test]
    fn test_layout_parity_page_breaks_across_formats() {
        use std::fs;
        let dir = std::env::temp_dir().join(format!("danoe_layout_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("planche.png"), png_bytes()).unwrap();
        fs::write(
            dir.join("c1.md"),
            "# Titre 1\n\nParagraphe d'ouverture.\n\n![Planche](planche.png)\n\n## Titre 2\n\nSuite du texte.\n",
        )
        .unwrap();

        let payload: ProjectPayload = serde_json::from_value(json!({
            "metadata": { "bookTitle": "Livre", "authorName": "A", "year": "2026" },
            "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
            "directories": { "sources": dir.to_string_lossy() },
            "organization": [
                { "type": "act", "displayName": "Acte 1", "children": [
                    { "type": "image", "displayName": "Planche", "sourceFileName": "planche.png" },
                    { "type": "chapter", "displayName": "Chapitre 1", "sourceFileName": "c1.md" }
                ] }
            ]
        }))
        .unwrap();

        // ── PDF (Typst) ──
        let spec = KdpPageSpec::resolve("6x9", estimate_page_count(&payload), false);
        let mut glossary: Vec<GlossaryChapter> = Vec::new();
        let blocks = build_blocks(&payload, false, &mut glossary).unwrap();
        let layout = &payload.layout_config;
        let typography = Typography {
            body_font: layout.body_font.as_str(),
            body_size: layout.body_size,
            line_spacing: layout.line_spacing,
            justify: true,
            drop_cap: layout.drop_cap,
            chapter_font: layout.resolve_font(&layout.chapter_title_font),
            chapter_size: layout.chapter_title_size,
            subtitle_font: layout.resolve_font(&layout.subtitle_font),
            subtitle_size: layout.subtitle_size,
        };
        let doc = PdfDoc {
            title: "Livre",
            author: "A",
            blocks: &blocks,
            typography: &typography,
            glossary_present: false,
            glossary: &glossary,
        };
        let src = generate(&spec, &doc);
        assert!(
            src.contains("#show heading.where(level: 1): it =>")
                && src.contains("align(center + horizon, it)"),
            "PDF — Titre 1 isolé (page + centrage H+V) :\n{src}"
        );
        assert!(
            src.contains("#show heading.where(level: 2): it =>")
                && src.contains("pagebreak(weak: true)"),
            "PDF — Titre 2 saut de page :\n{src}"
        );
        assert!(
            src.contains(
                "#pagebreak(weak: false)\n#block(width: 100%, height: 100%, align(center + horizon, figure(image("
            ),
            "PDF — Image isolée & centrée :\n{src}"
        );
        assert!(
            src.contains("let structural =") && src.contains("query(figure)") && src.contains("if not structural"),
            "PDF — en-tête masqué sur les pages structurelles (titres & illustrations) :\n{src}"
        );

        // ── DOCX ──
        let docx = crate::export::build_docx(&payload, None, false, &|_, _| {})
            .expect("génération DOCX");
        let document = read_docx_part(&docx, "word/document.xml");
        assert!(
            document.contains("oddPage"),
            "DOCX — sections isolées (saut de page oddPage) : Acte/Chapitre/Image"
        );
        assert!(
            document.contains("<w:drawing"),
            "DOCX — illustration présente"
        );
        assert!(
            document.contains("<w:vAlign w:val=\"center\""),
            "DOCX — centrage vertical absolu de l'illustration (vAlign)"
        );
        assert!(
            document.contains("<w:pageBreakBefore"),
            "DOCX — Titre 2 (sous-titre) déclenche un saut de page"
        );

        // ── ePUB ──
        assert!(
            crate::epub::EPUB_CSS.contains("break-before: page"),
            "ePUB — CSS titres : saut de page"
        );
        assert!(
            crate::epub::EPUB_CSS.contains(".figure-page")
                && crate::epub::EPUB_CSS.contains("min-height: 100vh"),
            "ePUB — CSS illustration : page dédiée centrée"
        );
        let epub = crate::epub::build_epub(&payload).expect("génération ePUB");
        assert!(
            read_zip_by_suffix(&epub, ".css").contains("break-before: page"),
            "ePUB — feuille de style embarquée"
        );
        assert!(
            read_zip_by_suffix(&epub, ".xhtml").contains("class=\"figure-page\""),
            "ePUB — illustration sur page dédiée"
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
