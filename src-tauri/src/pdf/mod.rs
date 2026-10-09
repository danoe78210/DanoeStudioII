//! Moteur PDF prêt-à-imprimer KDP (Typst embarqué).
//!
//! Pipeline : [`KdpPageSpec`](crate::kdp::KdpPageSpec) (règles physiques)
//! → balisage Typst ([`generator`]) → PDF ([`compiler`] + `typst`/`typst-pdf`).

pub mod compiler;
pub mod generator;

pub use compiler::TypstWorld;
pub use generator::{
    generate, GlossaryChapter, GlossaryEntry, PdfDoc, Typography,
};

/// Compile une source Typst isolée (sans ressources externes).
pub fn compile_to_pdf(source: &str) -> Result<Vec<u8>, String> {
    render(TypstWorld::new(source))
}

/// Compile une source Typst en résolvant les images depuis `root`.
pub fn compile_to_pdf_with_root(source: &str, root: &std::path::Path) -> Result<Vec<u8>, String> {
    render(TypstWorld::new_with_root(source, Some(root.to_path_buf())))
}

/// Met en page puis exporte un [`TypstWorld`] en octets PDF.
fn render(world: TypstWorld) -> Result<Vec<u8>, String> {
    let warned = typst::compile(&world);
    let document = warned
        .output
        .map_err(|errors| format!("échec de compilation Typst : {errors:?}"))?;
    typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|errors| format!("échec de l'export PDF : {errors:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kdp::KdpPageSpec;
    use crate::markdown::Block;

    /// Génère un PNG **valide** (2×2) via la crate `image`.
    fn tiny_png() -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([180, 40, 40, 255]));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut out, image::ImageFormat::Png)
            .expect("encodage PNG");
        out.into_inner()
    }

    #[test]
    fn full_ast_with_inline_styles_and_image_compiles_to_valid_pdf() {
        // 1) Spécification KDP : 6×9 po, 350 pages.
        let spec = KdpPageSpec::resolve("6x9", 350, false);

        // 2) AST factice complet : titre, gras/italique, scène, image.
        let blocks = vec![
            Block::Heading1("Chapitre Premier".to_string()),
            Block::Paragraph("Il était **une** fois un *cristal* de lumière.".to_string()),
            Block::SceneBreak,
            Block::Paragraph("Le **cristal** brille encore.".to_string()),
            Block::Image {
                target: "image.png".to_string(),
            },
        ];
        let typography = Typography {
            body_font: "Garamond",
            body_size: 11.0,
            line_spacing: 1.15,
            justify: true,
            drop_cap: true,
            chapter_font: "Garamond",
            chapter_size: 16.0,
            subtitle_font: "Garamond",
            subtitle_size: 14.0,
        };
        let doc = PdfDoc {
            title: "Nunael",
            author: "Danoë",
            blocks: &blocks,
            typography: &typography,
            glossary_present: false,
            glossary: &[],
        };

        // 3) Balisage Typst : mise en page + mapping de l'AST.
        let source = generate(&spec, &doc);
        assert!(source.contains("inside: 99.05pt"), "gouttière :\n{source}");
        assert!(source.contains("header: context"), "en-têtes courants");
        assert!(source.contains("footer: context"), "folio");
        assert!(
            source.contains("query(heading.where(level: 1))")
                && source.contains("line(length: 100%"),
            "en-tête = titre du chapitre courant + trait continu :\n{source}"
        );
        assert!(source.contains("#dropcap("), "lettrine");
        assert!(source.contains("weight: \"bold\""), "gras");
        assert!(source.contains("style: \"italic\""), "italique");
        assert!(source.contains("figure(image(\"image.png\""), "image");

        // 4) Ressources : racine + image factice.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join("assets");
        std::fs::create_dir_all(&dir).expect("racine assets");
        std::fs::write(dir.join("image.png"), tiny_png()).expect("image");
        std::fs::write(dir.join("doc.typ"), &source).expect("source Typst");

        // 5) Compilation Typst → PDF (résolution de l'image via `World::file`).
        let bytes = compile_to_pdf_with_root(&source, &dir).expect("compilation PDF");

        // 6) Validation.
        assert!(bytes.starts_with(b"%PDF-"), "en-tête PDF");
        assert!(
            bytes.len() > 1024,
            "PDF non trivial ({} octets)",
            bytes.len()
        );
        assert!(
            bytes.windows(5).any(|w| w == b"%%EOF"),
            "marqueur de fin %%EOF"
        );
        std::fs::write(dir.join("test_output.pdf"), &bytes).expect("écriture PDF");
        eprintln!("PDF AST complet : {} octets", bytes.len());
    }

    /// Le balisage généré câble les réglages typographiques de l'UI, le sommaire
    /// (`#outline`), les appels de note (`#super`) et le glossaire final.
    #[test]
    fn generate_wires_typography_notes_toc_and_glossary() {
        let spec = KdpPageSpec::resolve("6x9", 100, false);
        let blocks = vec![
            Block::Heading1("Chapitre 1".to_string()),
            Block::Paragraph("La flamme vacille.".to_string()),
            Block::Paragraph("Un esprit[^nahe] veille.".to_string()),
        ];
        let typography = Typography {
            body_font: "Cinzel",
            body_size: 12.0,
            line_spacing: 1.5,
            justify: false,
            drop_cap: true,
            chapter_font: "Garamond",
            chapter_size: 20.0,
            subtitle_font: "BodyX",
            subtitle_size: 13.0,
        };
        let glossary = vec![GlossaryChapter {
            label: "Chapitre 1".to_string(),
            entries: vec![GlossaryEntry {
                number: 1,
                term: "Nahe".to_string(),
                definition: "Esprit.".to_string(),
            }],
        }];
        let doc = PdfDoc {
            title: "Nunael",
            author: "Danoë",
            blocks: &blocks,
            typography: &typography,
            glossary_present: true,
            glossary: &glossary,
        };
        let src = generate(&spec, &doc);

        // Corps : police/taille câblées + repli embarqué garanti.
        assert!(
            src.contains("#set text(font: (\"Cinzel\", \"Libertinus Serif\"), size: 12pt)"),
            "corps :\n{src}"
        );
        // Titres de chapitre (niveau 1) et sous-titres (niveau 2).
        assert!(
            src.contains(
                "#show heading.where(level: 1): set text(font: (\"Garamond\", \"Libertinus Serif\"), size: 20pt)"
            ),
            "titre de chapitre :\n{src}"
        );
        assert!(
            src.contains(
                "#show heading.where(level: 2): set text(font: (\"BodyX\", \"Libertinus Serif\"), size: 13pt)"
            ),
            "sous-titres :\n{src}"
        );
        assert!(
            src.contains("#set par(justify: false, leading: 1.15em)"),
            "par :\n{src}"
        );
        // Sommaire (parité avec le champ TOC Word).
        assert!(
            src.contains("#outline(title: [Table des matières], depth: 2"),
            "sommaire :\n{src}"
        );
        // Notes : appel en exposant + glossaire final unifié.
        assert!(src.contains("#super[1]"), "appel de note :\n{src}");
        assert!(
            src.contains("#heading(level: 1)[Glossaire]"),
            "glossaire :\n{src}"
        );
        assert!(
            src.contains("#super[1] #text(\"Nahe\", weight: \"bold\") : #text(\"Esprit.\")"),
            "entrée de glossaire :\n{src}"
        );
        // Lettrine **jointive** : aucun espace parasite après l'initiale.
        assert!(
            src.contains("#dropcap(\"L\")[#text(\"a flamme vacille.\")]"),
            "lettrine jointive :\n{src}"
        );
        assert!(!src.contains("outset"), "plus de cadre à débordement :\n{src}");
    }
}
