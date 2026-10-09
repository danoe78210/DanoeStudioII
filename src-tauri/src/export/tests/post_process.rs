//! Tests unitaires du post-traitement OOXML/ZIP (`export/post_process.rs`) :
//! injection d'en-têtes courants, folio, sauts de section `oddPage`, lettrines et
//! validation bas niveau des parties XML.

use super::*;

/// La partie d'en-tête courante doit être un **XML bien formé** : en particulier
/// un **espace sépare** `w:rFonts` de son premier attribut (`w:ascii`) — sans lui,
/// `<w:rFontsw:ascii=…>` est du XML invalide et Word ignore l'en-tête (retour à
/// un héritage, ex. le nom de l'Acte).
#[test]
fn running_header_xml_is_well_formed_and_carries_chapter_name() {
    let xml = running_header_xml("Chapitre 1", "Garamond");
    assert!(
        xml.contains("<w:rFonts w:ascii=\"Garamond\""),
        "espace présent entre « w:rFonts » et « w:ascii » : {xml}"
    );
    assert!(
        !xml.contains("<w:rFontsw:ascii"),
        "attribut collé (XML invalide) détecté : {xml}"
    );
    assert!(xml.contains("standalone=\"yes\"?>"), "déclaration XML");
    assert!(xml.contains("<w:hdr "), "racine w:hdr");
    assert!(xml.contains("Chapitre 1"), "texte de l'en-tête présent");
}

#[test]
fn insert_after_self_closing_places_snippet_after_anchor() {
    let xml = "<w:settings><w:zoom w:percent=\"100\"/><w:compat/></w:settings>";
    let out = insert_after_self_closing(xml, "<w:zoom", "<w:mirrorMargins />").expect("ancre");
    assert!(
        out.contains("<w:zoom w:percent=\"100\"/><w:mirrorMargins />"),
        "snippet inséré après l'auto-fermeture : {out}"
    );
    // Ancre absente → None (le repli est géré par l'appelant).
    assert!(insert_after_self_closing(xml, "<w:absent", "<X/>").is_none());
}

#[test]
fn apply_drop_caps_injects_attributes_once_and_is_idempotent() {
    let xml = "prefix<w:framePr w:wrap=\"around\"/>suffix";
    let out = apply_drop_caps(xml);
    assert!(
        out.contains("<w:framePr w:dropCap=\"drop\" w:lines=\"2\" w:wrap=\"around\"/>"),
        "attributs de lettrine injectés : {out}"
    );
    // Idempotence : un document déjà traité n'est pas re-modifié.
    assert_eq!(apply_drop_caps(&out), out);
    // Aucun cadre de lettrine → document inchangé.
    assert_eq!(apply_drop_caps("<w:body/>"), "<w:body/>");
}

#[test]
fn empty_header_xml_has_single_declaration_and_root() {
    let xml = empty_header_xml();
    assert_eq!(xml.matches("<?xml").count(), 1);
    assert!(xml.starts_with("<?xml"));
    assert!(xml.contains("<w:hdr"));
}

#[test]
fn header_reference_block_targets_default_even_and_first() {
    let block = header_reference_block("rIdH1", "rIdEmpty");
    assert!(block.contains("w:type=\"default\" r:id=\"rIdH1\""));
    assert!(block.contains("w:type=\"even\" r:id=\"rIdH1\""));
    // La première page pointe toujours vers la partie vide (pas d'en-tête).
    assert!(block.contains("w:type=\"first\" r:id=\"rIdEmpty\""));
}

#[test]
fn replace_section_headers_strips_old_and_inserts_new_at_top() {
    let sect = "<w:sectPr><w:headerReference w:type=\"default\" r:id=\"OLD\"/><w:pgSz/></w:sectPr>";
    let out = replace_section_headers(sect, &header_reference_block("rIdNew", "rIdEmpty"));
    assert!(!out.contains("r:id=\"OLD\""), "ancienne référence retirée");
    assert!(
        out.contains("r:id=\"rIdNew\""),
        "nouvelle référence insérée"
    );
    // Le bloc est inséré juste après l'ouverture de `<w:sectPr>`.
    assert!(out.starts_with("<w:sectPr><w:headerReference"));
}

#[test]
fn inject_odd_page_marks_body_sections_but_not_the_last() {
    let xml = "<w:sectPr><w:pgSz/></w:sectPr>\
<w:sectPr><w:pgSz/></w:sectPr>\
<w:sectPr><w:pgSz/></w:sectPr>";
    // `body_start = 1` : la 1re section (liminaires) est exclue, la dernière
    // (colophon) aussi → une seule section du corps est marquée.
    let out = inject_odd_page(xml, 1);
    assert_eq!(out.matches("w:val=\"oddPage\"").count(), 1);
    // `body_start = 0` : deux sections marquées (la dernière reste exclue).
    assert_eq!(
        inject_odd_page(xml, 0).matches("w:val=\"oddPage\"").count(),
        2
    );
}

#[test]
fn inject_folio_footers_adds_reference_to_selected_sections() {
    let xml = "<w:sectPr><w:pgSz/></w:sectPr><w:sectPr><w:pgSz/></w:sectPr>";
    let out = inject_folio_footers(xml, &[1]);
    assert_eq!(out.matches("w:footerReference").count(), 1);
    assert!(out.contains(&format!("r:id=\"{FOLIO_FOOTER_RID}\"")));
    // Aucune section désignée → document inchangé.
    assert_eq!(inject_folio_footers(xml, &[]), xml);
}

#[test]
fn inject_running_headers_maps_each_section_to_its_label() {
    let mut map = std::collections::HashMap::new();
    map.insert("Chapitre 1".to_string(), "rIdRunningHeader1".to_string());
    let xml = "<w:sectPr><w:pgSz/></w:sectPr><w:sectPr><w:pgSz/></w:sectPr>";
    let labels = vec![Some("Chapitre 1".to_string()), None];
    let out = inject_running_headers(xml, &labels, &map, "rIdRunningHeaderEmpty");

    assert!(
        out.contains("r:id=\"rIdRunningHeader1\""),
        "en-tête du chapitre référencé"
    );
    assert!(
        out.contains("r:id=\"rIdRunningHeaderEmpty\""),
        "sections sans libellé → partie vide"
    );
    // Deux sections → deux blocs de références (chacun avec un `default`).
    assert_eq!(out.matches("w:type=\"default\"").count(), 2);
}

/// Le centrage vertical cible **uniquement** les sections d'illustration et place
/// `<w:vAlign w:val="center"/>` dans l'ordre du schéma (`avant <w:titlePg`).
#[test]
fn inject_vertical_centering_targets_only_centered_sections() {
    let xml = "<w:sectPr><w:pgSz/></w:sectPr><w:sectPr><w:titlePg/></w:sectPr>";
    let out = inject_vertical_centering(xml, &[1]);
    assert_eq!(
        out, "<w:sectPr><w:pgSz/></w:sectPr><w:sectPr><w:vAlign w:val=\"center\" /><w:titlePg/></w:sectPr>",
        "vAlign injecté dans la seule section ciblée, avant titlePg : {out}"
    );
    // Idempotent.
    assert_eq!(inject_vertical_centering(&out, &[1]), out);
    // Aucune section ciblée → XML inchangé.
    assert_eq!(inject_vertical_centering(xml, &[]), xml);
}

#[test]
fn folio_footer_xml_is_well_formed_with_single_declaration() {
    let xml = folio_footer_xml("Garamond");
    assert_eq!(xml.matches("<?xml").count(), 1);
    assert!(xml.starts_with("<?xml"));
    assert!(xml.contains("<w:ftr"));
    assert!(xml.contains("<w:instrText>PAGE</w:instrText>"));
}
