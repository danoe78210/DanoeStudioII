//! Environnement de compilation Typst (`typst::World`) : source principale,
//! polices embarquées et **résolution des fichiers/images** depuis une racine.

use std::collections::HashMap;
use std::path::PathBuf;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};

/// Nom virtuel de la source principale (unique) compilée.
const MAIN_VIRTUAL_PATH: &str = "main.typ";

/// Environnement de compilation Typst en mémoire.
pub struct TypstWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: FileId,
    sources: HashMap<FileId, Source>,
    /// Dossier racine pour résoudre les fichiers virtuels (images de l'AST).
    root: Option<PathBuf>,
}

impl TypstWorld {
    /// Monde sans résolution de fichiers (source isolée).
    pub fn new(source: impl Into<String>) -> Self {
        Self::new_with_root(source, None)
    }

    /// Compile `source` en résolvant les fichiers relatifs depuis `root`.
    pub fn new_with_root(source: impl Into<String>, root: Option<PathBuf>) -> Self {
        let fonts = load_fonts();
        let mut book = FontBook::new();
        for font in &fonts {
            book.push(font.info().clone());
        }

        let main = FileId::new(None, VirtualPath::new(MAIN_VIRTUAL_PATH));
        let mut sources = HashMap::new();
        sources.insert(main, Source::new(main, source.into()));

        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            main,
            sources,
            root,
        }
    }

    /// Chemin disque réel d'un fichier virtuel (résolu relativement à la racine).
    fn resolve(&self, id: FileId) -> Option<PathBuf> {
        let rel = id.vpath().as_rootless_path();
        let root = self.root.as_ref()?;
        if rel.is_absolute() {
            Some(rel.to_path_buf())
        } else {
            Some(root.join(rel))
        }
    }
}

impl World for TypstWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if let Some(source) = self.sources.get(&id) {
            return Ok(source.clone());
        }
        let path = self
            .resolve(id)
            .ok_or_else(|| FileError::NotFound(id.vpath().as_rootless_path().into()))?;
        let text = std::fs::read_to_string(&path).map_err(|e| FileError::from_io(e, &path))?;
        Ok(Source::new(id, text))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let path = self
            .resolve(id)
            .ok_or_else(|| FileError::NotFound(id.vpath().as_rootless_path().into()))?;
        std::fs::read(&path)
            .map(Bytes::from)
            .map_err(|e| FileError::from_io(e, &path))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}

/// Charge les polices : Libertinus embarqué (`typst-assets`) **puis** les polices
/// du système (recherche dynamique), avec repli ultime sur des candidats connus.
///
/// Aucune panique : si une police demandée par l'UI est introuvable, Typst
/// retombe sur la police embarquée (voir `generator::font_stack`).
fn load_fonts() -> Vec<Font> {
    // 1) Polices embarquées : garantes d'un rendu identique sur toute machine.
    let mut fonts: Vec<Font> = typst_assets::fonts()
        .filter_map(|data| Font::new(Bytes::from_static(data), 0))
        .collect();

    // 2) Polices système (le rendu suit alors la police choisie dans l'UI).
    for dir in system_font_dirs() {
        load_dir_fonts(&dir, &mut fonts);
        if fonts.len() >= MAX_FONTS {
            break;
        }
    }
    dedup_fonts(&mut fonts);

    // 3) Repli : candidats explicites si le scan n'a rien récupéré.
    if fonts.is_empty() {
        fonts = load_fallback_fonts();
    }
    if fonts.is_empty() {
        eprintln!("⚠ aucune police disponible : la compilation Typst échouera.");
    }
    fonts
}

/// Nombre maximal de polices chargées (garde-fou mémoire/temps d'analyse).
const MAX_FONTS: usize = 512;

/// Dossiers de polices du système (ordre stable).
fn system_font_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    #[cfg(target_os = "windows")]
    {
        if let Ok(windir) = std::env::var("WINDIR") {
            dirs.push(PathBuf::from(windir).join("Fonts"));
        } else {
            dirs.push(PathBuf::from("C:/Windows/Fonts"));
        }
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            dirs.push(
                PathBuf::from(local)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Fonts"),
            );
        }
    }
    #[cfg(target_os = "macos")]
    {
        dirs.push(PathBuf::from("/System/Library/Fonts"));
        dirs.push(PathBuf::from("/Library/Fonts"));
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join("Library/Fonts"));
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        dirs.push(PathBuf::from("/usr/share/fonts"));
        dirs.push(PathBuf::from("/usr/local/share/fonts"));
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join(".fonts"));
            dirs.push(PathBuf::from(home).join(".local/share/fonts"));
        }
    }
    dirs
}

/// Charge les polices d'un dossier (récursif) : `.ttf`, `.otf`, `.ttc`, `.otc`.
///
/// Les collections (`.ttc`/`.otc`) exposent plusieurs « faces » : on itère les
/// indices tant qu'une face valide est renvoyée.
fn load_dir_fonts(dir: &std::path::Path, fonts: &mut Vec<Font>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if fonts.len() >= MAX_FONTS {
            return;
        }
        let path = entry.path();
        if path.is_dir() {
            load_dir_fonts(&path, fonts);
            continue;
        }
        let is_font = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                matches!(
                    ext.to_ascii_lowercase().as_str(),
                    "ttf" | "otf" | "ttc" | "otc"
                )
            })
            .unwrap_or(false);
        if !is_font {
            continue;
        }
        let Ok(data) = std::fs::read(&path) else {
            continue;
        };
        let bytes = Bytes::from(data);
        let before = fonts.len();
        for index in 0..64u32 {
            match Font::new(bytes.clone(), index) {
                Some(font) => fonts.push(font),
                None => break,
            }
        }
        if fonts.len() == before {
            eprintln!("⚠ police illisible ignorée : {}", path.display());
        }
    }
}

/// Déduplique les polices (famille + variante) : la première occurrence gagne
/// (polices embarquées en premier, donc elles restent prioritaires).
fn dedup_fonts(fonts: &mut Vec<Font>) {
    let mut seen = std::collections::HashSet::new();
    fonts.retain(|font| {
        let info = font.info();
        let key = format!("{}|{:?}", info.family, info.variant);
        seen.insert(key)
    });
}

/// Repli : quelques polices fréquentes du système (aucune dépendance externe).
fn load_fallback_fonts() -> Vec<Font> {
    const CANDIDATES: &[&str] = &[
        "C:/Windows/Fonts/georgia.ttf",
        "C:/Windows/Fonts/times.ttf",
        "C:/Windows/Fonts/arial.ttf",
        "C:/Windows/Fonts/segoeui.ttf",
        "/System/Library/Fonts/Supplemental/Times New Roman.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
    ];
    let mut fonts = Vec::new();
    for path in CANDIDATES {
        if let Ok(data) = std::fs::read(path) {
            if let Some(font) = Font::new(Bytes::from(data), 0) {
                fonts.push(font);
            }
        }
    }
    fonts
}
