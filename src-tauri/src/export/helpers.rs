//! Utilitaires transverses : polices, échappement XML, conversions de dimensions,
//! résolution des chemins de fichiers de « Mes sources » et (dé)codage des images.

use docx_rs::*;
/// Facteur de corps appliqué à la lettrine (multiple de la taille du texte du corps).
pub(super) const DROP_CAP_SCALE: usize = 3;
/// Nombre de lignes occupées par la lettrine (`w:lines`).
pub(super) const DROP_CAP_LINES: u32 = 2;

/// Construit une pile de polices pour un nom donné.
pub(super) fn fonts(name: &str) -> RunFonts {
    RunFonts::new().ascii(name).hi_ansi(name)
}

/// Année courante (repli pour la mention de copyright).
pub(super) fn current_year() -> i32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    1970 + (secs / 31_556_952) as i32
}

/// Échappe un texte pour une insertion XML.
pub(super) fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Trio de pieds de page **vides** : masque le numéro hérité (les pages
/// liminaires et la table des matières ne sont pas foliotées).
pub(super) fn no_footers() -> (Footer, Footer, Footer) {
    (Footer::new(), Footer::new(), Footer::new())
}

/// Convertit une taille en points vers des demi-points (`w:sz`).
pub(super) fn points_to_half(points: f64) -> usize {
    (points * 2.0).round().max(2.0) as usize
}

/// Convertit un multiplicateur d'interligne vers `w:line` (1/240 de ligne).
pub(super) fn line_value(multiplier: f64) -> i32 {
    (multiplier * 240.0).round().max(240.0) as i32
}

/// Applique le **traitement colorimétrique** configuré aux octets d'une image.
///
/// En mode **Noir & Blanc** (`grayscale`), l'image est décodée, convertie en
/// niveaux de gris 8 bits (optimisé pour l'impression de livre) puis ré-encodée
/// en PNG. En cas d'échec de décodage/encodage (format exotique), les octets
/// d'origine sont conservés : la génération n'est jamais interrompue.
pub(super) fn apply_image_color_mode(bytes: Vec<u8>, grayscale: bool) -> Vec<u8> {
    if !grayscale {
        return bytes;
    }
    let decoded = match image::load_from_memory(&bytes) {
        Ok(decoded) => decoded,
        Err(error) => {
            println!("conversion N&B impossible (décodage) : {error}");
            return bytes;
        }
    };
    let mut encoded = std::io::Cursor::new(Vec::new());
    match decoded
        .to_luma8()
        .write_to(&mut encoded, image::ImageFormat::Png)
    {
        Ok(()) => encoded.into_inner(),
        Err(error) => {
            println!("conversion N&B impossible (encodage) : {error}");
            bytes
        }
    }
}

/// Les octets fournis sont-ils exploitables par `docx-rs::Pic::new` ?
///
/// Ce contrôle **réplique exactement** la logique de `docx-rs` 0.4 : un **PNG** est
/// accepté dès que ses **dimensions** sont lisibles dans l'en-tête (aucun décodage
/// complet n'est nécessaire), tandis que **tout autre format** doit être
/// **entièrement décodable** par la crate `image`. L'objectif est d'éviter le
/// `expect(...)` interne de `Pic::new`, qui **paniquerait** sur des octets non
/// décodables et interromprait brutalement tout l'export.
pub(super) fn image_is_rasterizable(bytes: &[u8]) -> bool {
    const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if bytes.starts_with(&PNG_SIGNATURE) {
        return image::ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .ok()
            .and_then(|reader| reader.into_dimensions().ok())
            .is_some();
    }
    image::load_from_memory(bytes).is_ok()
}

/// Recherche un fichier par nom dans `dir`, de façon **insensible à la casse** et
/// **récursive** (Obsidian résout une image par son seul nom de fichier, même si
/// elle se trouve dans un sous-dossier du coffre). Renvoie le chemin réel.
pub(super) fn find_file_ci(dir: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    // On compare le **nom de fichier** : une cible `images/planche.png` doit
    // pouvoir correspondre à un fichier situé dans un sous-dossier.
    let wanted = std::path::Path::new(name)
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| name.to_lowercase());

    let mut stack: Vec<std::path::PathBuf> = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let matches = path
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase() == wanted)
                .unwrap_or(false);
            if matches {
                return Some(path);
            }
        }
    }
    None
}

/// Collecte **récursivement** les fichiers d'image (par extension) d'un dossier.
/// Sert au diagnostic : quand un nœud image n'a **aucun** fichier source lié, on
/// liste les illustrations réellement disponibles dans « Mes sources ».
pub(super) fn collect_image_files(dir: &std::path::Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_image_files(&path, out);
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let lower = name.to_lowercase();
            let is_image = [
                ".png", ".jpg", ".jpeg", ".webp", ".tiff", ".tif", ".gif", ".bmp",
            ]
            .iter()
            .any(|ext| lower.ends_with(ext));
            if is_image {
                out.push(name.to_string());
            }
        }
    }
}

/// Résout le **chemin physique** d'un fichier de « Mes sources », de la façon la
/// plus robuste possible (l'image est sinon « ignorée en silence » alors qu'elle
/// est bien présente sur le disque) :
///
/// 1. `target` est un **chemin absolu** existant → utilisé tel quel (le frontend
///    peut ainsi injecter un chemin absolu) ;
/// 2. sinon `dossier_sources/target` (cas nominal, y compris sous-dossier relatif) ;
/// 3. sinon **recherche récursive insensible à la casse** par nom de fichier
///    (Obsidian résout une illustration par son seul nom, où qu'elle soit rangée).
pub(super) fn resolve_source_path(
    dir: &std::path::Path,
    target: &str,
) -> Option<std::path::PathBuf> {
    let candidate = std::path::Path::new(target);
    if candidate.is_absolute() && candidate.is_file() {
        return Some(candidate.to_path_buf());
    }
    let joined = dir.join(target);
    if joined.is_file() {
        return Some(joined);
    }
    find_file_ci(dir, target)
}

// NOTE : les marges KDP (`SIDE_MARGIN_TWIPS`, `TOP_MARGIN_TWIPS`,
// `BOTTOM_MARGIN_TWIPS`) ne sont **plus** définies ici. Source unique de vérité :
// le module central `crate::kdp` (`src/kdp.rs`), qui les expose et les teste.
