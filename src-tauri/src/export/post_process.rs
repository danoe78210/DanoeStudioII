//! Post-traitement du paquet OOXML/ZIP : marges miroir, lettrines, sauts de
//! section `oddPage`, folio, en-têtes courants et validation bas niveau.

use docx_rs::*;

use super::helpers::*;
/// Épaisseur (1/8 pt) du trait de séparation sous l'en-tête courant.
pub(super) const HEADER_RULE_SIZE: usize = 6;
/// Taille (demi-points) du texte d'en-tête courant : **8 pt**.
pub(super) const HEADER_FONT_SIZE: usize = 16;

/// Préfixe des identifiants de relation des en-têtes courants.
pub(super) const RUNNING_HEADER_RID_PREFIX: &str = "rIdRunningHeader";
/// Suffixe de la partie d'en-tête **vide** (première page sans en-tête).
pub(super) const EMPTY_HEADER_SUFFIX: &str = "Empty";
/// Type de relation OOXML d'un en-tête.
pub(super) const HEADER_REL_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/header";
/// Type de contenu OOXML d'une partie d'en-tête.
pub(super) const HEADER_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml";

/// Partie d'en-tête (running head) complète : centrée, capitales, **8 pt**, avec
/// **uniquement** un trait de séparation inférieur (`w:pBdr`/`w:bottom`).
///
/// Ces parties sont générées **par post-traitement** (voir
/// `inject_running_headers`) : `docx-rs` associe les en-têtes aux sections de
/// façon erratique (décalage d'un cran), ce qui affichait le nom de l'acte au
/// lieu du nom du chapitre.
pub(super) fn running_header_xml(text: &str, font: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:hdr xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:p><w:pPr><w:rPr><w:sz w:val=\"{size}\" /><w:szCs w:val=\"{size}\" /><w:caps w:val=\"true\" />\
<w:rFonts w:ascii=\"{font}\" w:hAnsi=\"{font}\" /></w:rPr><w:jc w:val=\"center\" />\
<w:spacing w:after=\"120\" /><w:pBdr><w:bottom w:val=\"single\" w:space=\"4\" w:sz=\"{rule}\" w:color=\"808080\" /></w:pBdr>\
</w:pPr><w:r><w:t xml:space=\"preserve\">{text}</w:t></w:r></w:p></w:hdr>",
        size = HEADER_FONT_SIZE,
        rule = HEADER_RULE_SIZE,
        font = xml_escape(font),
        text = xml_escape(text),
    )
}

/// Partie d'en-tête **vide** (aucun contenu : masque tout en-tête hérité).
pub(super) fn empty_header_xml() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:hdr xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" />".to_string()
}

/// Construit le pied de page : numéro de page centré (police du corps, discrète).
///
/// Le champ `PAGE` est assemblé **run par run** (`fldChar` début / `instrText` /
/// `fldChar` séparateur / résultat / `fldChar` fin). C'est indispensable : le
/// helper `PageNum` de `docx-rs` regroupe tous les `fldChar` dans un **seul run**
/// (OOXML invalide), ce qui amène certains lecteurs à afficher le mot « PAGE » au
/// lieu du numéro.
pub(super) fn page_footer(font: &str) -> Footer {
    let paragraph = Paragraph::new()
        .align(AlignmentType::Center)
        .size(20)
        .fonts(fonts(font))
        .add_run(Run::new().add_field_char(FieldCharType::Begin, true))
        .add_run(Run::new().add_instr_text(InstrText::PAGE(InstrPAGE::new())))
        .add_run(Run::new().add_field_char(FieldCharType::Separate, false))
        .add_run(Run::new().add_text("1").size(20).fonts(fonts(font)))
        .add_run(Run::new().add_field_char(FieldCharType::End, false));
    Footer::new().add_paragraph(paragraph)
}

/// Insère `snippet` juste après l'élément **auto-fermé** débutant par `anchor`
/// (ex. `"<w:zoom"`). Renvoie `None` si l'ancre (ou sa fermeture `/>`) est absente.
pub(super) fn insert_after_self_closing(xml: &str, anchor: &str, snippet: &str) -> Option<String> {
    let start = xml.find(anchor)?;
    let close = xml[start..].find("/>")?;
    let at = start + close + 2;
    let mut out = String::with_capacity(xml.len() + snippet.len());
    out.push_str(&xml[..at]);
    out.push_str(snippet);
    out.push_str(&xml[at..]);
    Some(out)
}

/// Complète les cadres de **lettrine** produits par le moteur.
///
/// `docx-rs` expose `w:framePr` (via `Paragraph::wrap/v_anchor/h_anchor`) mais **pas**
/// les attributs `w:dropCap` / `w:lines` requis par Word pour un vrai drop cap. On les
/// injecte donc ici, sur les seuls `w:framePr` du document (aucun autre usage n'en fait).
pub(super) fn apply_drop_caps(xml: &str) -> String {
    if !xml.contains("<w:framePr") || xml.contains("w:dropCap=") {
        return xml.to_string();
    }
    xml.replace(
        "<w:framePr ",
        &format!("<w:framePr w:dropCap=\"drop\" w:lines=\"{DROP_CAP_LINES}\" "),
    )
}

/// Nom de partie et identifiant de relation du pied de page « **folio** » (numéro
/// de page), injecté par post-traitement dans les sections du corps.
pub(super) const FOLIO_FOOTER_PART: &str = "word/footerFolio.xml";
pub(super) const FOLIO_FOOTER_RID: &str = "rIdFooterFolio";
pub(super) const FOOTER_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml";
pub(super) const FOOTER_REL_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer";

/// XML de la partie pied de page « folio » (numéro de page centré).
///
/// `Footer::build()` produit déjà la **déclaration XML** et l'élément racine
/// `<w:ftr>` avec ses espaces de noms : ne rien ajouter (sinon déclaration
/// dupliquée → partie XML invalide → Word refuse d'ouvrir le fichier).
pub(super) fn folio_footer_xml(font: &str) -> String {
    String::from_utf8_lossy(&page_footer(font).build()).into_owned()
}

/// Injecte une **référence de pied de page** (`w:footerReference`) dans les sections
/// désignées par leur index (ordre des `<w:pgSz` dans le document).
pub(super) fn inject_folio_footers(xml: &str, numbered: &[usize]) -> String {
    if numbered.is_empty() {
        return xml.to_string();
    }
    let marker = "<w:pgSz";
    let reference = format!("<w:footerReference w:type=\"default\" r:id=\"{FOLIO_FOOTER_RID}\" />");
    let mut out = String::with_capacity(xml.len() + reference.len() * numbered.len());
    let mut rest = xml;
    let mut index = 0;
    while let Some(position) = rest.find(marker) {
        let (before, after) = rest.split_at(position);
        out.push_str(before);
        if numbered.contains(&index) {
            // Respecte l'ordre du schéma `CT_SectPr` : `footerReference` avant `w:type`.
            match before.rfind("<w:type") {
                Some(type_pos) => {
                    let at = out.len() - before.len() + type_pos;
                    out.insert_str(at, &reference);
                }
                None => out.push_str(&reference),
            }
        }
        out.push_str(marker);
        rest = &after[marker.len()..];
        index += 1;
    }
    out.push_str(rest);
    out
}

/// Post-traitement OOXML : image « derrière le texte » (`behindDoc`), marges miroir
/// et injection du **folio** dans les sections du corps.
/// Construit le bloc de références d'en-tête d'une section.
pub(super) fn header_reference_block(rid: &str, empty_rid: &str) -> String {
    format!(
        "<w:headerReference w:type=\"default\" r:id=\"{rid}\" />\
<w:headerReference w:type=\"even\" r:id=\"{rid}\" />\
<w:headerReference w:type=\"first\" r:id=\"{empty_rid}\" />"
    )
}

/// Remplace les `w:headerReference` d'une section par le bloc fourni (inséré en
/// tête de `w:sectPr`, conformément à la séquence `CT_SectPr`).
pub(super) fn replace_section_headers(sect: &str, block: &str) -> String {
    // 1) Retirer les références d'en-tête existantes.
    let mut cleaned = String::with_capacity(sect.len());
    let mut rest = sect;
    while let Some(index) = rest.find("<w:headerReference") {
        cleaned.push_str(&rest[..index]);
        match rest[index..].find("/>") {
            Some(end) => rest = &rest[index + end + 2..],
            None => {
                rest = "";
                break;
            }
        }
    }
    cleaned.push_str(rest);

    // 2) Insérer le bloc juste après l'ouverture `<w:sectPr ...>`.
    let Some(open) = cleaned.find("<w:sectPr") else {
        return cleaned;
    };
    let Some(gt) = cleaned[open..].find('>') else {
        return cleaned;
    };
    let insert_at = open + gt + 1;
    let mut out = String::with_capacity(cleaned.len() + block.len());
    out.push_str(&cleaned[..insert_at]);
    out.push_str(block);
    out.push_str(&cleaned[insert_at..]);
    out
}

/// Injecte les **en-têtes courants** : chaque section reçoit une référence
/// explicite vers la partie d'en-tête correspondant à **son** libellé (nom du
/// chapitre saisi dans « Organisation »). Les sections sans libellé pointent vers
/// la partie vide ; `first` pointe toujours vers la partie vide (première page
/// sans en-tête).
pub(super) fn inject_running_headers(
    xml: &str,
    section_labels: &[Option<String>],
    rid_for_label: &std::collections::HashMap<String, String>,
    empty_rid: &str,
) -> String {
    // **Garde anti-décalage** : l'association section → libellé repose sur l'ordre
    // des `<w:sectPr>` du document, qui doit correspondre **exactement** à celui des
    // sections produites (une par spécification, dans l'ordre). Si le décompte
    // diffère, un décalage d'un cran est possible : on le signale explicitement
    // plutôt que d'attribuer un mauvais en-tête en silence.
    let total_sections = xml.matches("<w:sectPr").count();
    if total_sections != section_labels.len() {
        trace!(
            "⚠ EN-TÊTES COURANTS : {total_sections} section(s) détectée(s) mais {} libellé(s) attendu(s) — correspondance chapitre ↔ section à vérifier.",
            section_labels.len()
        );
    } else {
        trace!(
            "→ en-têtes courants : {total_sections} section(s) ↔ {} libellé(s) (correspondance vérifiée).",
            section_labels.len()
        );
    }
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    let mut index = 0usize;
    while let Some(start) = rest.find("<w:sectPr") {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let end = after
            .find("</w:sectPr>")
            .map(|e| e + "</w:sectPr>".len())
            .unwrap_or(after.len());
        let sect = &after[..end];
        let label = section_labels
            .get(index)
            .and_then(|value| value.as_deref())
            .unwrap_or("");
        trace!(
            "→ section #{index} : en-tête = {}",
            if label.is_empty() {
                "(vide)".to_string()
            } else {
                format!("« {label} »")
            }
        );
        let rid = rid_for_label
            .get(label)
            .map(String::as_str)
            .unwrap_or(empty_rid);
        out.push_str(&replace_section_headers(
            sect,
            &header_reference_block(rid, empty_rid),
        ));
        rest = &after[end..];
        index += 1;
    }
    out.push_str(rest);
    out
}
/// Injecte `<w:vAlign w:val="center"/>` dans le `sectPr` des sections ciblées :
/// **centrage vertical absolu** de l'illustration, quelle que soit sa hauteur.
/// La balise est placée dans l'ordre du schéma (`CT_SectPr`) : juste avant
/// `<w:titlePg` s'il existe, sinon avant `</w:sectPr>`.
pub(super) fn inject_vertical_centering(xml: &str, centered: &[usize]) -> String {
    if centered.is_empty() {
        return xml.to_string();
    }
    let mut out = String::with_capacity(xml.len() + centered.len() * 32);
    let mut rest = xml;
    let mut index = 0usize;
    while let Some(start) = rest.find("<w:sectPr") {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let end = after
            .find("</w:sectPr>")
            .map(|e| e + "</w:sectPr>".len())
            .unwrap_or(after.len());
        let sect = &after[..end];
        if centered.contains(&index) {
            out.push_str(&insert_v_align(sect));
        } else {
            out.push_str(sect);
        }
        rest = &after[end..];
        index += 1;
    }
    out.push_str(rest);
    out
}

/// Ajoute `<w:vAlign w:val="center"/>` à un `sectPr` (idempotent).
fn insert_v_align(sect: &str) -> String {
    if sect.contains("<w:vAlign") {
        return sect.to_string();
    }
    if let Some(position) = sect.find("<w:titlePg") {
        let mut out = String::with_capacity(sect.len() + 28);
        out.push_str(&sect[..position]);
        out.push_str("<w:vAlign w:val=\"center\" />");
        out.push_str(&sect[position..]);
        out
    } else {
        sect.replace("</w:sectPr>", "<w:vAlign w:val=\"center\" /></w:sectPr>")
    }
}



/// Post-traitement du paquet OOXML : images « derrière le texte », lettrines,
/// sauts de section `oddPage`, folio et **en-têtes courants**.
pub(super) fn post_process(
    bytes: Vec<u8>,
    body_start: usize,
    numbered: &[usize],
    font: &str,
    section_labels: &[Option<String>],
    centered_sections: &[usize],
) -> Result<Vec<u8>, String> {
    use std::io::{Cursor, Read, Write};

    // Parties d'en-tête à créer : une par libellé **distinct**, plus une vide.
    let mut rid_for_label: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut header_parts: Vec<(String, String, String)> = Vec::new(); // (rid, cible, xml)
    for label in section_labels.iter().filter_map(|label| label.as_ref()) {
        if !rid_for_label.contains_key(label) {
            let index = rid_for_label.len() + 1;
            let rid = format!("{RUNNING_HEADER_RID_PREFIX}{index}");
            let target = format!("headerRunning{index}.xml");
            rid_for_label.insert(label.clone(), rid.clone());
            header_parts.push((rid, target, running_header_xml(label, font)));
        }
    }
    let empty_rid = format!("{RUNNING_HEADER_RID_PREFIX}{EMPTY_HEADER_SUFFIX}");
    header_parts.push((
        empty_rid.clone(),
        format!("headerRunning{EMPTY_HEADER_SUFFIX}.xml"),
        empty_header_xml(),
    ));

    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| format!("lecture du paquet .docx : {e}"))?;
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let mut buffer = Vec::new();
        entry.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
        drop(entry);

        if name == "word/document.xml" {
            // `behindDoc` est codé en dur à 0 par docx-rs → « derrière le texte ».
            let xml = String::from_utf8_lossy(&buffer)
                .replace("behindDoc=\"0\"", "behindDoc=\"1\"")
                .replace("allowOverlap=\"0\"", "allowOverlap=\"1\"")
                .replace(
                    "<wp:wrapSquare wrapText=\"bothSides\" />",
                    "<wp:wrapNone />",
                )
                .replace("<wp:wrapSquare wrapText=\"bothSides\"/>", "<wp:wrapNone />");
            // Lettrines : ajoute `w:dropCap`/`w:lines` aux cadres produits par le moteur.
            let xml = apply_drop_caps(&xml);
            // Sauts de section `oddPage` pour les sections du corps de l'ouvrage.
            // NB : l'éventuelle page « fantôme » insérée par Word avant un saut `oddPage`
            // appartient à la section précédente et hérite de son pied de page ; sa
            // dé-foliotation n'est pas exprimable statiquement en OOXML (elle dépend du
            // rendu de mise en page) — voir SPECIFICATIONS §20 (phase 3, limite connue).
            // En-têtes courants : chaque section référence SA partie d'en-tête.
            let xml = inject_running_headers(
                &inject_vertical_centering(&inject_odd_page(&xml, body_start), centered_sections),
                section_labels,
                &rid_for_label,
                &empty_rid,
            );
            // Folio : référence de pied de page dans les sections du corps.
            buffer = inject_folio_footers(&xml, numbered).into_bytes();
        } else if name == "word/_rels/document.xml.rels" {
            let mut xml = String::from_utf8_lossy(&buffer).to_string();
            // Relations vers les parties d'en-tête courantes.
            for (rid, target, _) in &header_parts {
                if !xml.contains(rid.as_str()) {
                    let rel = format!(
                        "<Relationship Id=\"{rid}\" Type=\"{HEADER_REL_TYPE}\" Target=\"{target}\"/>"
                    );
                    xml = xml.replace("</Relationships>", &format!("{rel}</Relationships>"));
                }
            }
            // Relation vers la partie pied de page « folio » (si un folio est requis).
            if !numbered.is_empty() && !xml.contains(FOLIO_FOOTER_RID) {
                let rel = format!(
                    "<Relationship Id=\"{FOLIO_FOOTER_RID}\" Type=\"{FOOTER_REL_TYPE}\" Target=\"footerFolio.xml\"/>"
                );
                xml = xml.replace("</Relationships>", &format!("{rel}</Relationships>"));
            }
            buffer = xml.into_bytes();
        } else if name == "[Content_Types].xml" {
            let mut xml = String::from_utf8_lossy(&buffer).to_string();
            // Types de contenu des parties d'en-tête courantes.
            for (_, target, _) in &header_parts {
                let part = format!("word/{target}");
                if !xml.contains(&part) {
                    let declaration = format!(
                        "<Override PartName=\"/{part}\" ContentType=\"{HEADER_CONTENT_TYPE}\"/>"
                    );
                    xml = xml.replace("</Types>", &format!("{declaration}</Types>"));
                }
            }
            // Déclaration du type de contenu de la partie pied de page « folio ».
            if !numbered.is_empty() && !xml.contains(FOLIO_FOOTER_PART) {
                let declaration = format!(
                    "<Override PartName=\"/{FOLIO_FOOTER_PART}\" ContentType=\"{FOOTER_CONTENT_TYPE}\"/>"
                );
                xml = xml.replace("</Types>", &format!("{declaration}</Types>"));
            }
            buffer = xml.into_bytes();
        } else if name == "word/settings.xml" {
            let mut xml = String::from_utf8_lossy(&buffer).to_string();
            // `<w:mirrorMargins/>` (marges en vis-à-vis / miroir). Il DOIT précéder
            // `w:compat` et `w:evenAndOddHeaders` pour respecter la séquence du schéma
            // (`CT_Settings`) — sinon Word peut ignorer l'élément (et le miroir du dos).
            if !xml.contains("<w:mirrorMargins") {
                xml = insert_after_self_closing(&xml, "<w:zoom", "<w:mirrorMargins />")
                    .unwrap_or_else(|| {
                        xml.replace("</w:settings>", "<w:mirrorMargins /></w:settings>")
                    });
            }
            // Demande à Word de recalculer les champs (table des matières) à l'ouverture.
            if !xml.contains("<w:updateFields") {
                xml = xml.replace(
                    "</w:settings>",
                    "<w:updateFields w:val=\"true\" /></w:settings>",
                );
            }
            buffer = xml.into_bytes();
        }

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        writer
            .start_file(name, options)
            .map_err(|e| e.to_string())?;
        writer.write_all(&buffer).map_err(|e| e.to_string())?;
    }

    // Parties d'en-tête courantes (une par libellé distinct + une vide).
    for (_, target, xml) in &header_parts {
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        writer
            .start_file(format!("word/{target}"), options)
            .map_err(|e| e.to_string())?;
        writer
            .write_all(xml.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    // Partie pied de page « folio » : créée une seule fois, référencée par les
    // sections du corps (numéro de page).
    if !numbered.is_empty() {
        let footer = folio_footer_xml(font);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        writer
            .start_file(FOLIO_FOOTER_PART, options)
            .map_err(|e| e.to_string())?;
        writer
            .write_all(footer.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    writer
        .finish()
        .map(|cursor| cursor.into_inner())
        .map_err(|e| format!("écriture du paquet .docx : {e}"))
}

/// Injecte `<w:type w:val="oddPage"/>` dans les `<w:sectPr>` du **corps** (celles
/// après les `body_start` premières : couverture et pages liminaires).
pub(super) fn inject_odd_page(xml: &str, body_start: usize) -> String {
    let marker = "<w:pgSz";
    // Dernière section = colophon (saut de page simple) → exclue des sauts `oddPage`.
    let total = xml.matches(marker).count();
    let mut out = String::with_capacity(xml.len() + 64);
    let mut rest = xml;
    let mut index = 0;
    while let Some(position) = rest.find(marker) {
        let (before, after) = rest.split_at(position);
        out.push_str(before);
        if index >= body_start && index + 1 < total {
            out.push_str("<w:type w:val=\"oddPage\" />");
        }
        out.push_str(marker);
        rest = &after[marker.len()..];
        index += 1;
    }
    out.push_str(rest);
    out
}
