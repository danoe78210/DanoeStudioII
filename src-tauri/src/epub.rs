//! Scaffold d'export **EPUB 3** (crate `epub-builder`).
//!
//! Première itération (scaffold) : métadonnées, page de titre, un XHTML par acte /
//! chapitre / page spéciale (Markdown → titres et paragraphes), feuille de style minimale
//! et sommaire (nav) généré. Images et notes restent à traiter dans une itération ultérieure.

use epub_builder::{EpubBuilder, EpubContent, ReferenceType, ZipLibrary};

use crate::markdown::{self, Block};
use crate::project::{OrganizationNode, ProjectPayload};

/// Feuille de style minimale (lecture fluide + **aération structurelle**).
pub(crate) const EPUB_CSS: &str = "\
body { font-family: serif; line-height: 1.5; margin: 1em; }
h1, h2 { text-align: center; }
h1 { break-before: page; min-height: 100vh; display: flex; align-items: center; justify-content: center; }
h2 { break-before: page; }
p { text-align: justify; }
.title-page { min-height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; }
.figure-page { break-before: page; break-after: page; min-height: 100vh; display: flex; align-items: center; justify-content: center; }
.figure-page img { max-width: 100%; max-height: 100%; }
.saga { font-variant: small-caps; letter-spacing: 0.1em; }
.sep { text-align: center; }
";

/// Nom de fichier `.epub` proposé (slug du titre, comme le Word).
pub fn default_epub_name(payload: &ProjectPayload) -> String {
    let base = crate::export::default_file_name(payload);
    match base.strip_suffix(".docx") {
        Some(stem) => format!("{stem}.epub"),
        None => "manuscrit.epub".to_string(),
    }
}

/// Construit un EPUB minimal à partir du projet ; renvoie les octets du fichier `.epub`.
pub fn build_epub(payload: &ProjectPayload) -> Result<Vec<u8>, String> {
    let md = &payload.metadata;
    let zip = ZipLibrary::new().map_err(|e| e.to_string())?;
    let mut builder = EpubBuilder::new(zip).map_err(|e| e.to_string())?;

    builder
        .metadata("title", md.book_title.trim())
        .map_err(|e| e.to_string())?;
    builder
        .metadata("author", md.author_name.trim())
        .map_err(|e| e.to_string())?;
    builder.metadata("lang", "fr").map_err(|e| e.to_string())?;
    builder
        .stylesheet(EPUB_CSS.as_bytes())
        .map_err(|e| e.to_string())?;

    // --- Page de titre. ---
    let title = title_page_html(payload);
    builder
        .add_content(
            EpubContent::new("title.xhtml", title.as_bytes())
                .title("Page de titre")
                .reftype(ReferenceType::TitlePage),
        )
        .map_err(|e| e.to_string())?;

    // --- Sections (dans l'ordre de l'arborescence). ---
    let sources = payload.directories.sources.clone().unwrap_or_default();
    let mut flat: Vec<FlatEpub> = Vec::new();
    for node in &payload.organization {
        flatten(node, &mut flat);
    }

    for (index, item) in flat.iter().enumerate() {
        let href = format!("section{:03}.xhtml", index + 1);
        let html = section_html(item, &sources);
        builder
            .add_content(
                EpubContent::new(href, html.as_bytes())
                    .title(item.title.clone())
                    .reftype(ReferenceType::Text),
            )
            .map_err(|e| e.to_string())?;
    }

    let mut out = Vec::new();
    builder.generate(&mut out).map_err(|e| e.to_string())?;
    Ok(out)
}

/// Élément aplati (titre lisible + fichier source éventuel).
struct FlatEpub {
    title: String,
    source: Option<String>,
}

/// Aplatit récursivement l'arborescence (actes → parties, chapitres/pages → sections).
fn flatten(node: &OrganizationNode, out: &mut Vec<FlatEpub>) {
    let title = node
        .display_name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| match node.kind.as_str() {
            "act" => "Partie".to_string(),
            "special" => node.role.clone().unwrap_or_else(|| "Page".to_string()),
            _ => "Chapitre".to_string(),
        });

    if matches!(node.kind.as_str(), "act" | "chapter" | "special") {
        out.push(FlatEpub {
            title,
            source: node.source_file_name.clone().filter(|n| !n.is_empty()),
        });
    }

    for child in &node.children {
        flatten(child, out);
    }
}

/// XHTML de la page de titre.
fn title_page_html(payload: &ProjectPayload) -> String {
    let md = &payload.metadata;
    let mut body = String::new();
    if !md.saga_title.trim().is_empty() {
        body.push_str(&format!("<p class=\"saga\">{}</p>", escape(&md.saga_title)));
    }
    body.push_str(&format!("<h1>{}</h1>", escape(&md.book_title)));
    if !md.subtitle.trim().is_empty() {
        body.push_str(&format!("<p><em>{}</em></p>", escape(&md.subtitle)));
    }
    if !md.author_name.trim().is_empty() {
        body.push_str(&format!("<p>{}</p>", escape(&md.author_name)));
    }
    xhtml_doc(
        &md.book_title,
        &format!("<div class=\"title-page\">{body}</div>"),
    )
}

/// XHTML d'une section (acte = titre seul ; chapitre/page = contenu Markdown converti).
fn section_html(item: &FlatEpub, sources: &str) -> String {
    let mut body = format!("<h1>{}</h1>", escape(&item.title));
    if let Some(name) = &item.source {
        let text = read_source(sources, name);
        body.push_str(&blocks_to_html(&text));
    }
    xhtml_doc(&item.title, &body)
}

/// Enveloppe un corps HTML dans un document XHTML valide.
fn xhtml_doc(title: &str, body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<!DOCTYPE html>\n\
<html xmlns=\"http://www.w3.org/1999/xhtml\" xml:lang=\"fr\">\n\
<head><meta charset=\"utf-8\" /><title>{}</title></head>\n\
<body>{}</body>\n</html>",
        escape(title),
        body
    )
}

/// Convertit le Markdown (sous-ensemble) en HTML.
pub(crate) fn blocks_to_html(text: &str) -> String {
    let (body, _definitions) = markdown::extract_definitions(text);
    let mut html = String::new();
    for block in markdown::parse_blocks(&body) {
        match block {
            Block::Heading1(t) => html.push_str(&format!("<h1>{}</h1>", escape(&t))),
            Block::Heading2(t) => html.push_str(&format!("<h2>{}</h2>", escape(&t))),
            Block::Paragraph(t) => html.push_str(&format!("<p>{}</p>", escape(&t))),
            Block::Quote(t) => {
                html.push_str(&format!("<blockquote><p>{}</p></blockquote>", escape(&t)))
            }
            Block::ListItem { text, .. } => html.push_str(&format!("<p>• {}</p>", escape(&text))),
            Block::SceneBreak => html.push_str("<p class=\"sep\">* * *</p>"),
            // Planche autonome : page dédiée, centrée (CSS `.figure-page`).
            Block::Image { target } => html.push_str(&format!(
                "<div class=\"figure-page\"><img src=\"{}\" alt=\"\" /></div>",
                escape(&target)
            )),
        }
    }
    html
}

/// Lecture tolérante d'un fichier source (`<sources>/<name>`).
///
/// Le **frontmatter YAML** (Obsidian) est supprimé, comme pour l'export Word :
/// les métadonnées (`title:`, `tome:`, `pov:`…) ne doivent pas apparaître dans
/// le contenu généré.
fn read_source(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        return String::new();
    }
    match std::fs::read_to_string(std::path::Path::new(dir).join(name)) {
        Ok(text) => markdown::strip_frontmatter(&text),
        Err(_) => String::new(),
    }
}

/// Échappement HTML minimal.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload() -> ProjectPayload {
        serde_json::from_value(serde_json::json!({
            "metadata": {
                "bookTitle": "Nunael",
                "subtitle": "Chroniques du Nord",
                "sagaTitle": "Les Schattenjägers",
                "authorName": "Danoë",
                "publisher": "Autoédition"
            },
            "organization": [
                { "type": "act", "displayName": "Acte 1", "children": [
                    { "type": "chapter", "displayName": "Chapitre 1" }
                ] }
            ]
        }))
        .expect("payload valide")
    }

    #[test]
    fn build_epub_produces_a_zip() {
        let payload = sample_payload();
        let bytes = build_epub(&payload).expect("génération epub");
        assert!(!bytes.is_empty(), "epub non vide");
        // Un EPUB est une archive ZIP (signature « PK »).
        assert_eq!(&bytes[0..2], b"PK", "signature ZIP");
    }

    #[test]
    fn default_epub_name_uses_slug() {
        let payload = sample_payload();
        assert_eq!(default_epub_name(&payload), "Nunael.epub");
    }

    #[test]
    fn flatten_collects_acts_chapters_and_specials_in_order() {
        let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
            "organization": [
                { "type": "act", "displayName": "Acte 1", "children": [
                    { "type": "chapter", "displayName": "Chapitre 1", "sourceFileName": "c1.md" },
                    { "type": "image", "displayName": "Ignorée" }
                ] },
                { "type": "special", "role": "glossary" }
            ]
        }))
        .unwrap();

        let mut flat: Vec<FlatEpub> = Vec::new();
        for node in &payload.organization {
            flatten(node, &mut flat);
        }

        let titles: Vec<&str> = flat.iter().map(|item| item.title.as_str()).collect();
        assert_eq!(titles, vec!["Acte 1", "Chapitre 1", "glossary"]);
        // Un nœud `image` ne devient jamais une section.
        assert!(flat.iter().all(|item| item.title != "Ignorée"));
        // Le fichier source du chapitre est conservé ; les autres n'en ont pas.
        assert_eq!(flat[1].source.as_deref(), Some("c1.md"));
        assert!(flat[0].source.is_none());
    }

    #[test]
    fn flatten_uses_kind_fallback_when_name_missing() {
        let node: OrganizationNode =
            serde_json::from_value(serde_json::json!({ "type": "chapter" })).unwrap();
        let mut flat = Vec::new();
        flatten(&node, &mut flat);
        assert_eq!(flat[0].title, "Chapitre");
    }

    #[test]
    fn blocks_to_html_maps_markdown_blocks() {
        let html =
            blocks_to_html("# Titre\n\n## Sous\n\nPara.\n\n> Cit.\n\n- item\n\n1. ord\n\n---\n");
        assert!(html.contains("<h1>Titre</h1>"));
        assert!(html.contains("<h2>Sous</h2>"));
        assert!(html.contains("<p>Para.</p>"));
        assert!(html.contains("<blockquote><p>Cit.</p></blockquote>"));
        assert!(html.contains("<p>• item</p>"));
        assert!(html.contains("<p>• ord</p>"));
        assert!(html.contains("<p class=\"sep\">* * *</p>"));
    }

    #[test]
    fn html_escape_covers_special_characters() {
        assert_eq!(
            escape("a & b <c> \"d\""),
            "a &amp; b &lt;c&gt; &quot;d&quot;"
        );
    }

    #[test]
    fn xhtml_doc_wraps_a_valid_document() {
        let doc = xhtml_doc("Titre & Co", "<p>x</p>");
        assert!(doc.contains("<!DOCTYPE html>"));
        assert!(doc.contains("xmlns=\"http://www.w3.org/1999/xhtml\""));
        assert!(doc.contains("<title>Titre &amp; Co</title>"));
        assert!(doc.contains("<body><p>x</p></body>"));
    }

    #[test]
    fn read_source_strips_frontmatter_and_tolerates_missing() {
        use std::fs;

        let dir = std::env::temp_dir().join(format!("danoe-epub-src-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("c1.md"),
            "---\ntitle: X\n---\n# Chapitre\n\nCorps.\n",
        )
        .unwrap();

        let text = read_source(&dir.to_string_lossy(), "c1.md");
        assert!(!text.contains("title: X"), "frontmatter retiré");
        assert!(text.contains("Corps."), "corps conservé");
        // Fichier absent et dossier vide → chaîne vide (jamais d'erreur).
        assert_eq!(read_source(&dir.to_string_lossy(), "absent.md"), "");
        assert_eq!(read_source("", "c1.md"), "");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn title_page_html_includes_available_metadata() {
        let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
            "metadata": {
                "bookTitle": "Nunael",
                "subtitle": "Chroniques",
                "sagaTitle": "Les Schattenjägers",
                "authorName": "Danoë"
            }
        }))
        .unwrap();

        let html = title_page_html(&payload);
        assert!(html.contains("class=\"saga\""));
        assert!(html.contains("<h1>Nunael</h1>"));
        assert!(html.contains("<em>Chroniques</em>"));
        assert!(html.contains("Danoë"));
    }

    #[test]
    fn section_html_without_source_is_title_only() {
        let item = FlatEpub {
            title: "Acte 1".to_string(),
            source: None,
        };
        let html = section_html(&item, "");
        assert!(html.contains("<h1>Acte 1</h1>"));
        // Sans source, le corps reste limité au titre de section.
        assert!(!html.contains("<p>"));
    }
}
