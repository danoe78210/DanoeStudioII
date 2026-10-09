//! Tests unitaires des pages liminaires (`export/front_matter.rs`) : hauteurs de
//! ligne figées, paragraphes espaceurs ancrés et section de couverture.

use super::*;

#[test]
fn line_height_is_twelve_per_half_point() {
    assert_eq!(line_height_twips(20), 240);
    assert_eq!(line_height_twips(32), 384);
    assert_eq!(line_height_twips(0), 0);
}

#[test]
fn spacer_uses_exact_line_rule_and_optional_page_break() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    docx_rs::Docx::new()
        .add_paragraph(spacer(240, true))
        .add_paragraph(spacer(120, false))
        .build()
        .pack(&mut cursor)
        .expect("sérialisation du document de test");

    let document = read_entry(&cursor.into_inner(), "word/document.xml");
    // Hauteur de ligne **figée** (ancrage bas de page déterministe).
    assert!(document.contains("w:lineRule=\"exact\""));
    assert!(document.contains("w:line=\"240\""));
    // Le premier espaceur démarre une nouvelle page.
    assert!(document.contains("<w:pageBreakBefore"));
}

#[test]
fn cover_spec_with_image_is_unnumbered_and_headerless() {
    let payload = sample_payload();
    let spec = cover_spec(&payload, Some(TINY_PNG), "Garamond", 8640, 12960);
    assert!(!spec.page_numbered, "la couverture n'est pas foliotée");
    assert!(
        spec.running_label.is_none(),
        "aucun en-tête courant sur la couverture"
    );
    assert!(
        !spec.paragraphs.is_empty(),
        "au moins l'image de fond et le titre"
    );
}

#[test]
fn cover_spec_without_image_keeps_text_only() {
    let payload = sample_payload();
    let spec = cover_spec(&payload, None, "Garamond", 8640, 12960);
    assert!(
        !spec.paragraphs.is_empty(),
        "le titre reste présent sans image"
    );
}
