//! Rendu du corps de l'ouvrage : aplatissement de l'arborescence, lecture des
//! sources, blocs Markdown (lettrines, citations, listes) et images.

use docx_rs::*;

use crate::kdp::{
    gutter_twips, BOTTOM_MARGIN_TWIPS, EMU_PER_TWIP, SIDE_MARGIN_TWIPS, TOP_MARGIN_TWIPS,
};
use crate::markdown::{self, Block, BodyPiece, NoteDefinition};
use crate::project::{
    estimate_page_count, LayoutConfig, OrganizationNode, ProjectMetadata, ProjectPayload,
};

use super::helpers::*;
use super::ProgressReporter;

use super::SectionSpec;
/// Interligne **minimal** du corps de texte : 1,5 (aération romanesque). Une
/// configuration plus aérée (1,5 / 2) est conservée.
pub(super) const BODY_MIN_LINE_SPACING: f64 = 1.5;
/// Espacement **après** chaque paragraphe de corps (twips) : ~ une ligne blanche,
/// pour nettement séparer les paragraphes narratifs.
pub(super) const BODY_PARAGRAPH_SPACING_AFTER: u32 = 240;
/// Espacement vertical (twips) isolant une image du texte qui l'entoure.
pub(super) const IMAGE_SPACING_TWIPS: u32 = 240;

/// Type d'élément du corps de l'ouvrage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ItemKind {
    Act,
    Chapter,
    Special,
    /// Illustration pleine page (image de transition, ex. avant un acte).
    Image,
}

/// Élément aplati de l'arborescence, avec le contexte d'acte.
#[derive(Debug, Clone)]
pub(super) struct FlatItem {
    kind: ItemKind,
    node: OrganizationNode,
    act_label: Option<String>,
}

/// Entrée de glossaire : numéro de renvoi, terme affiché et définition.
pub(super) struct GlossaryEntry {
    /// Numéro du renvoi (exposant) tel qu'attribué dans le corps.
    pub number: u32,
    /// Terme mis en évidence (en gras) au début de la ligne : « Nahe ».
    pub term: String,
    /// Définition seule (le terme est retiré du texte s'il y figurait).
    pub definition: String,
}

/// Chapitre du glossaire (regroupement par chapitre d'origine).
pub(super) struct GlossaryChapter {
    label: String,
    entries: Vec<GlossaryEntry>,
}

/// Aplatit récursivement l'arborescence (actes → chapitres / pages spéciales).
pub(super) fn collect_items(
    nodes: &[OrganizationNode],
    act: Option<&str>,
    out: &mut Vec<FlatItem>,
) {
    for node in nodes {
        match node.kind.as_str() {
            "act" => {
                let label = node.display_name.clone().unwrap_or_default();
                out.push(FlatItem {
                    kind: ItemKind::Act,
                    node: node.clone(),
                    act_label: None,
                });
                let next = if label.is_empty() {
                    act
                } else {
                    Some(label.as_str())
                };
                collect_items(&node.children, next, out);
            }
            "chapter" => out.push(FlatItem {
                kind: ItemKind::Chapter,
                node: node.clone(),
                act_label: act.map(str::to_string),
            }),
            "special" => out.push(FlatItem {
                kind: ItemKind::Special,
                node: node.clone(),
                act_label: act.map(str::to_string),
            }),
            // Illustration pleine page (image de transition) : rendue sur sa page.
            "image" => out.push(FlatItem {
                kind: ItemKind::Image,
                node: node.clone(),
                act_label: act.map(str::to_string),
            }),
            _ => collect_items(&node.children, act, out),
        }
    }
}

/// La page spéciale « Glossaire » est-elle présente dans l'arborescence ?
pub(super) fn has_glossary(nodes: &[OrganizationNode]) -> bool {
    nodes
        .iter()
        .any(|node| node.role.as_deref() == Some("glossary") || has_glossary(&node.children))
}

/// Lit un fichier source texte (`txt`/`md`) depuis le dossier « Mes sources ».
pub(super) fn read_source(sources: &Option<String>, name: &str) -> Option<String> {
    let Some(dir) = sources.as_deref().filter(|d| !d.is_empty()) else {
        println!("aucun dossier « Mes sources » configuré : source « {name} » ignorée");
        return None;
    };
    let lower = name.to_lowercase();
    if !(lower.ends_with(".txt") || lower.ends_with(".md")) {
        println!("source ignorée (extension non prise en charge) : {name}");
        return None;
    }
    let path = std::path::Path::new(dir).join(name);
    match std::fs::read_to_string(&path) {
        // Frontmatter YAML (Obsidian) supprimé avant tout traitement : les
        // métadonnées (`title:`, `tome:`, `pov:`…) ne doivent jamais être écrites
        // dans le manuscrit Word.
        Ok(text) => Some(markdown::strip_frontmatter(&text)),
        Err(error) => {
            println!("lecture source impossible « {} » : {error}", path.display());
            None
        }
    }
}

/// Contenu texte associé à un élément (via le cache).
pub(super) fn source_text<'a>(
    item: &'a FlatItem,
    files: &'a std::collections::HashMap<String, String>,
) -> Option<&'a str> {
    item.node
        .source_file_name
        .as_deref()
        .filter(|n| !n.is_empty())
        .and_then(|name| files.get(name).map(String::as_str))
}

/// Libellé d'un élément (nom dans le livre, sinon valeur de repli).
pub(super) fn item_label(item: &FlatItem) -> String {
    item.node
        .display_name
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| match item.kind {
            ItemKind::Act => "Acte".to_string(),
            ItemKind::Chapter => "Chapitre".to_string(),
            ItemKind::Special => special_role_label(item.node.role.as_deref()),
            ItemKind::Image => "Illustration".to_string(),
        })
}

/// Libellé lisible d'un rôle de **page spéciale**. Repli : chaîne **vide**
/// (aucun titre parasite — l'ancien libellé générique « Page » est supprimé).
pub(super) fn special_role_label(role: Option<&str>) -> String {
    match role {
        Some("dedication") => "Dédicace".to_string(),
        Some("epigraph") => "Épigraphe".to_string(),
        Some("prologue") => "Prologue".to_string(),
        Some("epilogue") => "Épilogue".to_string(),
        Some("authorNote") => "Note de l'auteur".to_string(),
        Some("acknowledgements") => "Remerciements".to_string(),
        Some("glossary") => "Glossaire".to_string(),
        _ => String::new(),
    }
}

/// Libellé de regroupement pour le glossaire (« Acte 2, chapitre 7 »).
pub(super) fn chapter_group_label(item: &FlatItem) -> String {
    let chapter = item_label(item);
    match item.act_label.as_deref() {
        Some(act) => format!("{act}, {}", chapter.to_lowercase()),
        None => chapter,
    }
}

/// Convertit les définitions en entrées numérotées (numéros attribués par les renvois).
pub(super) fn definitions_to_entries(
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
            // Terme affiché : « Terme : définition » explicite dans la source, sinon
            // **dérivé du libellé de note** `[^libellé]` (capitalisé). Le lecteur voit
            // ainsi toujours le mot défini, même avec une définition « nue ».
            let (term, definition) = match split_term(&definition.text) {
                Some((term, rest)) => (term.to_string(), rest.to_string()),
                None => (label_to_term(&definition.label), definition.text.clone()),
            };
            GlossaryEntry {
                number,
                term,
                definition,
            }
        })
        .collect();
    entries.sort_by_key(|entry| entry.number);
    entries
}

/// Dérive un terme lisible à partir du libellé de note `[^libellé]` : première
/// lettre capitalisée (« nahe » → « Nahe », « custodia » → « Custodia »).
fn label_to_term(label: &str) -> String {
    let mut chars = label.trim().chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Contexte de construction du corps de l'ouvrage.
pub(super) struct BodyLayout<'a> {
    payload: &'a ProjectPayload,
    font: &'a str,
    margin: &'a PageMargin,
    page_w: u32,
    page_h: u32,
    /// Largeur **utile** (zone d'impression) en twips : page − marges latérales − gouttière.
    content_w: u32,
    /// Hauteur **utile** (zone d'impression) en twips : page − marges haute et basse.
    content_h: u32,
    /// Taille du corps en demi-points.
    body_size: usize,
    /// Interligne en 1/240 de ligne.
    line: i32,
    glossary_present: bool,
    files: &'a std::collections::HashMap<String, String>,
    glossary: &'a [GlossaryChapter],
}

impl BodyLayout<'_> {
    /// Crée une section de corps (marges KDP, en-tête dynamique, pied folioté).
    fn section(&self, paragraphs: Vec<Paragraph>, label: &str) -> SectionSpec {
        SectionSpec {
            paragraphs,
            page_w: self.page_w,
            page_h: self.page_h,
            margin: self.margin.clone(),
            title_pg: true,
            start_page: None,
            // L'en-tête courant (nom du chapitre saisi dans Organisation) est
            // injecté par post-traitement : voir `inject_running_headers`.
            running_label: if label.trim().is_empty() {
                None
            } else {
                Some(label.to_string())
            },
            // Le folio est injecté par post-traitement (voir `post_process`) : une
            // section `docx-rs` ne peut porter en même temps un en-tête et un pied.
            footers: None,
            page_numbered: true,
            vertical_center: false,
        }
    }
}

/// Itère l'arborescence, lit les sources et produit les sections du corps.
///
/// **Échec explicite** : renvoie `Err` si une illustration référencée ne peut pas
/// être intégrée (fichier introuvable/illisible/non décodable). Aucune image n'est
/// silencieusement ignorée et aucune page blanche n'est produite.
pub(super) fn build_body(
    payload: &ProjectPayload,
    font: &str,
    margin: &PageMargin,
    page_w: u32,
    page_h: u32,
    progress: ProgressReporter,
) -> Result<Vec<SectionSpec>, String> {
    let layout = &payload.layout_config;
    let sources = payload.directories.sources.clone();
    let glossary_present = has_glossary(&payload.organization);

    let mut items: Vec<FlatItem> = Vec::new();
    collect_items(&payload.organization, None, &mut items);

    // Lecture (une seule fois) de tous les fichiers sources **texte** référencés
    // (les illustrations sont lues séparément, comme binaires).
    let mut files: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for item in items.iter().filter(|i| i.kind != ItemKind::Image) {
        if let Some(name) = item
            .node
            .source_file_name
            .as_deref()
            .filter(|n| !n.is_empty())
        {
            if !files.contains_key(name) {
                if let Some(text) = read_source(&sources, name) {
                    files.insert(name.to_string(), text);
                }
            }
        }
    }
    let count = |kind: ItemKind| items.iter().filter(|item| item.kind == kind).count();
    println!(
        "structure : {} acte(s), {} chapitre(s), {} page(s) spéciale(s), {} illustration(s)",
        count(ItemKind::Act),
        count(ItemKind::Chapter),
        count(ItemKind::Special),
        count(ItemKind::Image),
    );
    println!(
        "{} élément(s) structuré(s), {} fichier(s) source chargé(s)",
        items.len(),
        files.len()
    );
    // Trace des illustrations (nom de source) pour diagnostiquer l'injection.
    for item in items.iter().filter(|item| item.kind == ItemKind::Image) {
        match item.node.source_file_name.as_deref().filter(|n| !n.is_empty()) {
            Some(name) => {
                trace!("✓ illustration liée : « {name} » (source = {name:?})")
            }
            None => trace!(
                "⚠ ILLUSTRATION SANS FICHIER LIÉ (id={:?}, displayName={:?}) — sélectionnez une image dans l'onglet Organisation.",
                item.node.id, item.node.display_name
            ),
        }
    }
    progress("Lecture des sources…", 10);

    // Passe 1 — agrégation du glossaire (définitions par chapitre, cas B).
    let mut glossary: Vec<GlossaryChapter> = Vec::new();
    if glossary_present {
        for item in items.iter().filter(|i| i.kind == ItemKind::Chapter) {
            let Some(text) = source_text(item, &files) else {
                continue;
            };
            let (body, definitions) = markdown::extract_definitions(text);
            let mut numbers: Vec<(String, u32)> = Vec::new();
            for paragraph in markdown::split_paragraphs(&body) {
                let _ = markdown::number_refs(&paragraph, &mut numbers);
            }
            let entries = definitions_to_entries(&definitions, &mut numbers);
            if !entries.is_empty() {
                glossary.push(GlossaryChapter {
                    label: chapter_group_label(item),
                    entries,
                });
            }
        }
    }
    progress("Analyse du manuscrit…", 30);

    // Zone d'impression utile : sert à la mise à l'échelle stricte des images
    // (largeur utile = page − marges latérales − gouttière de reliure).
    let gutter = gutter_twips(estimate_page_count(payload));
    let content_w = page_w.saturating_sub((SIDE_MARGIN_TWIPS as u32) * 2 + gutter);
    let content_h = page_h.saturating_sub(TOP_MARGIN_TWIPS as u32 + BOTTOM_MARGIN_TWIPS as u32);

    let ctx = BodyLayout {
        payload,
        font,
        margin,
        page_w,
        page_h,
        content_w,
        content_h,
        body_size: points_to_half(layout.body_size),
        // Interligne du corps : au moins 1,5 (aération), config plus large honorée.
        line: line_value(layout.line_spacing.max(BODY_MIN_LINE_SPACING)),
        glossary_present,
        files: &files,
        glossary: &glossary,
    };

    // Passe 2 — génération des sections.
    let mut specs: Vec<SectionSpec> = Vec::new();
    for item in &items {
        // Un échec d'intégration d'illustration est **explicite** (remonté à
        // l'appelant) : jamais d'image ignorée en silence ni de page blanche.
        let spec = match item.kind {
            ItemKind::Act => act_spec(item, &ctx),
            ItemKind::Chapter => text_spec(item, &ctx)?,
            ItemKind::Special => special_spec(item, &ctx)?,
            ItemKind::Image => image_spec(item, &ctx)?,
        };
        specs.push(spec);
    }
    progress("Structuration des sections…", 60);

    // Table des matières : après les liminaires et les dédicaces/épigraphes d'ouverture.
    let mut toc_index = 0;
    for item in &items {
        let is_opening = item.kind == ItemKind::Special
            && matches!(
                item.node.role.as_deref(),
                Some("dedication") | Some("epigraph")
            );
        if is_opening {
            toc_index += 1;
        } else {
            break;
        }
    }
    specs.insert(toc_index, toc_spec(&ctx));

    // Achevé d'imprimer : ultime section (saut de page simple).
    specs.push(colophon_spec(&payload.metadata, &ctx));

    progress("Injection de la table des matières et du colophon…", 80);

    Ok(specs)
}

/// Retrait de première ligne (0,5 cm ≈ 283 twips).
pub(super) const FIRST_LINE_TWIPS: i32 = 283;
/// Espacement avant d'une dédicace (~10 cm) pour la repousser en bas de page.
pub(super) const DEDICATION_SPACING_BEFORE: u32 = 5670;

/// Alignement du corps selon la configuration.
pub(super) fn body_alignment(layout: &LayoutConfig) -> AlignmentType {
    if layout.text_alignment == "left" {
        AlignmentType::Left
    } else {
        AlignmentType::Both
    }
}

/// Crée un run à partir d'un fragment en ligne (`**gras**`, `*italique*`).
pub(super) fn inline_run(span: &markdown::Inline, font: &str, size: usize) -> Run {
    let mut run = Run::new()
        .add_text(span.text.clone())
        .size(size)
        .fonts(fonts(font));
    if span.bold {
        run = run.bold();
    }
    if span.italic {
        run = run.italic();
    }
    run
}

/// Run d'appel de note (numéro en exposant).
pub(super) fn superscript_run(number: u32, font: &str, size: usize) -> Run {
    let mut run = Run::new()
        .add_text(number.to_string())
        .size(size)
        .fonts(fonts(font));
    run.run_property = run.run_property.vert_align(VertAlignType::SuperScript);
    run
}

/// Le paragraphe peut-il porter une lettrine sans que le cadre (`w:framePr`
/// `w:dropCap`, hauteur `DROP_CAP_LINES` lignes) **déborde** sous le texte ?
///
/// Un cadre de lettrine plus haut que son paragraphe s'étend sous celui-ci ; avec
/// `w:wrap="around"`, Word fait alors **remonter le paragraphe suivant** autour du
/// cadre, détruisant la mise en page. On applique donc la lettrine **uniquement**
/// lorsque le paragraphe couvre au moins la hauteur du cadre.
///
/// L'estimation est **prudente** : on suppose une largeur moyenne de caractère
/// volontairement faible (donc un nombre de caractères par ligne élevé, donc un
/// nombre de lignes **sous-estimé**) afin de ne **jamais** déclencher une lettrine
/// débordante, quelle que soit la longueur du premier paragraphe.
fn drop_cap_fits(text: &str, ctx: &BodyLayout) -> bool {
    if ctx.content_w == 0 {
        return false;
    }
    let chars = text.chars().filter(|c| !c.is_whitespace()).count() as f64;
    if chars == 0.0 {
        return false;
    }
    // Largeur utile (twips) ÷ largeur moyenne d'un caractère (~0,5 em). 1 em (corps)
    // = (demi-points / 2) points × 20 twips/point.
    let body_pt = (ctx.body_size as f64 / 2.0).max(1.0);
    let avg_char_twips = (body_pt * 0.5 * 20.0).max(1.0);
    let chars_per_line = (ctx.content_w as f64 / avg_char_twips).floor().max(1.0);
    // Le paragraphe doit contenir de quoi remplir franchement les lignes de la
    // lettrine, sinon le cadre déborderait (mise en page cassée).
    chars >= chars_per_line * DROP_CAP_LINES as f64
}

/// Paragraphe de corps (justifié, interligne, retrait de première ligne).
///
/// Si `drop_cap` est vrai, la **première lettre** est détachée en **lettrine** : le
/// paragraphe reçoit un cadre (`w:framePr`, sans retrait de première ligne) et la lettre
/// un corps très supérieur. `docx-rs` n'exposant ni `w:dropCap` ni `w:lines` sur
/// `w:framePr`, ces attributs sont ajoutés en post-traitement (voir `apply_drop_caps`).
pub(super) fn body_paragraph(pieces: &[BodyPiece], ctx: &BodyLayout, drop_cap: bool) -> Paragraph {
    let mut paragraph = Paragraph::new()
        .align(body_alignment(&ctx.payload.layout_config))
        .line_spacing(
            LineSpacing::new()
                .line(ctx.line)
                .line_rule(LineSpacingType::Auto)
                // Espacement après le paragraphe : aère la lecture romanesque.
                .after(BODY_PARAGRAPH_SPACING_AFTER),
        );

    if drop_cap {
        // Lettrine : cadre enveloppant, sans retrait de première ligne.
        paragraph = paragraph.wrap("around").v_anchor("text").h_anchor("text");
    } else {
        paragraph = paragraph.indent(
            None,
            Some(SpecialIndentType::FirstLine(FIRST_LINE_TWIPS)),
            None,
            None,
        );
    }

    let mut drop_pending = drop_cap;
    for piece in pieces {
        match piece {
            BodyPiece::Text(text) => {
                if drop_pending {
                    let start = text.len() - text.trim_start().len();
                    let mut chars = text[start..].chars();
                    if let Some(first) = chars.next() {
                        drop_pending = false;
                        paragraph = paragraph.add_run(
                            Run::new()
                                .add_text(first.to_string())
                                .size(ctx.body_size.saturating_mul(DROP_CAP_SCALE))
                                .fonts(fonts(ctx.font)),
                        );
                        let remainder: String = chars.collect();
                        if !remainder.is_empty() {
                            for span in markdown::parse_inline(&remainder) {
                                paragraph =
                                    paragraph.add_run(inline_run(&span, ctx.font, ctx.body_size));
                            }
                        }
                        continue;
                    }
                }
                for span in markdown::parse_inline(text) {
                    paragraph = paragraph.add_run(inline_run(&span, ctx.font, ctx.body_size));
                }
            }
            BodyPiece::NoteRef(number) => {
                paragraph = paragraph.add_run(superscript_run(*number, ctx.font, ctx.body_size));
            }
        }
    }
    paragraph
}

/// Paragraphe de titre de partie/chapitre : **style Titre 1** (collecté par la TOC).
pub(super) fn heading_paragraph(label: &str, ctx: &BodyLayout, size: usize) -> Paragraph {
    Paragraph::new()
        .style("Heading1")
        .align(AlignmentType::Center)
        .outline_lvl(0)
        .line_spacing(LineSpacing::new().before(2400).after(720))
        .add_run(
            Run::new()
                .add_text(label.to_uppercase())
                .size(size)
                .bold()
                .fonts(fonts(ctx.font)),
        )
}

/// Renvoie les pièces d'un texte (renvois numérotés si glossaire actif).
pub(super) fn note_pieces(
    text: &str,
    ctx: &BodyLayout,
    numbers: &mut Vec<(String, u32)>,
) -> Vec<BodyPiece> {
    if ctx.glossary_present {
        markdown::number_refs(text, numbers)
    } else {
        vec![BodyPiece::Text(text.to_string())]
    }
}

/// Construit une liste de runs à partir de pièces (texte + appels de note).
pub(super) fn pieces_runs(pieces: &[BodyPiece], ctx: &BodyLayout, italic: bool) -> Vec<Run> {
    let mut runs = Vec::new();
    for piece in pieces {
        match piece {
            BodyPiece::Text(text) => {
                for span in markdown::parse_inline(text) {
                    let mut run = inline_run(&span, ctx.font, ctx.body_size);
                    if italic {
                        run = run.italic();
                    }
                    runs.push(run);
                }
            }
            BodyPiece::NoteRef(number) => {
                runs.push(superscript_run(*number, ctx.font, ctx.body_size));
            }
        }
    }
    runs
}

/// Paragraphe de titre (Titre 1 / Titre 2) : sans retrait de première ligne.
///
/// `page_break` : lorsqu'il vaut `true`, un **saut de page** (`w:pageBreakBefore`)
/// est inséré **avant** le paragraphe. Utilisé pour forcer un sous-titre `##`
/// isolé à démarrer une nouvelle page, sauf s'il suit immédiatement le titre
/// principal du chapitre.
pub(super) fn heading_styled(
    label: &str,
    ctx: &BodyLayout,
    level: u8,
    page_break: bool,
) -> Paragraph {
    // Espacement **après** généreux : aère la transition titre/sous-titre → récit.
    // Niveau 1 : repoussé vers le **centre vertical** de la page (isolement).
    let (style, size, before, after) = if level == 1 {
        let centered = (ctx.content_h * 30 / 100).max(480);
        ("Heading1", 32usize, centered, 480u32)
    } else {
        ("Heading2", 28usize, 360u32, 360u32)
    };
    let paragraph = Paragraph::new()
        .style(style)
        .align(AlignmentType::Center)
        .outline_lvl(if level == 1 { 0 } else { 1 })
        .line_spacing(LineSpacing::new().before(before).after(after))
        .add_run(
            Run::new()
                .add_text(label.to_string())
                .size(size)
                .bold()
                .fonts(fonts(ctx.font)),
        );
    if page_break {
        paragraph.page_break_before(true)
    } else {
        paragraph
    }
}

/// Séparateur de scène (`* * *`) : centré, sans retrait, large espacement.
pub(super) fn scene_break(ctx: &BodyLayout) -> Paragraph {
    Paragraph::new()
        .align(AlignmentType::Center)
        .line_spacing(LineSpacing::new().before(480).after(480))
        .add_run(
            Run::new()
                .add_text("* * *")
                .size(ctx.body_size)
                .fonts(fonts(ctx.font)),
        )
}

/// Citation : retrait gauche/droite, justifiée, italique, sans retrait de première ligne.
pub(super) fn quote_paragraph(pieces: &[BodyPiece], ctx: &BodyLayout) -> Paragraph {
    let mut paragraph = Paragraph::new()
        .align(body_alignment(&ctx.payload.layout_config))
        .indent(Some(567), None, Some(567), None)
        .line_spacing(
            LineSpacing::new()
                .line(ctx.line)
                .line_rule(LineSpacingType::Auto),
        );
    for run in pieces_runs(pieces, ctx, true) {
        paragraph = paragraph.add_run(run);
    }
    paragraph
}

/// Item de liste native (à puces — `numId` 1 — ou numérotée — `numId` 2).
pub(super) fn list_paragraph(
    text: &str,
    ordered: bool,
    ctx: &BodyLayout,
    numbers: &mut Vec<(String, u32)>,
) -> Paragraph {
    let pieces = note_pieces(text, ctx, numbers);
    let mut paragraph = Paragraph::new()
        .numbering(
            NumberingId::new(if ordered { 2 } else { 1 }),
            IndentLevel::new(0),
        )
        .line_spacing(
            LineSpacing::new()
                .line(ctx.line)
                .line_rule(LineSpacingType::Auto),
        );
    for run in pieces_runs(&pieces, ctx, false) {
        paragraph = paragraph.add_run(run);
    }
    paragraph
}

/// Charge une image du dossier « Mes sources », applique le traitement
/// colorimétrique configuré puis la **met à l'échelle** pour tenir strictement
/// dans la zone d'impression utile : largeur = colonne utile, hauteur plafonnée à
/// `max_height_twips` (le ratio est préservé).
///
/// **Échec explicite** : renvoie un `Err` (message actionnable) si l'illustration
/// est introuvable, illisible ou de format non exploitable — l'export échoue
/// clairement plutôt que de produire une page blanche ou d'ignorer l'image en
/// silence.
pub(super) fn load_scaled_pic(
    target: &str,
    ctx: &BodyLayout,
    max_height_twips: u32,
) -> Result<Pic, String> {
    let dir = match ctx
        .payload
        .directories
        .sources
        .as_deref()
        .filter(|d| !d.is_empty())
    {
        Some(dir) => dir,
        None => {
            return Err(format!(
                "Illustration « {target} » non intégrable : aucun dossier « Mes sources » n'est configuré (onglet Réglages)."
            ));
        }
    };
    let base = std::path::Path::new(dir);
    // Chemin **absolu** construit en combinant le dossier des sources et le nom du
    // fichier sélectionné dans l'UI — journalisé pour lever toute ambiguïté.
    let candidate_absolute = base.join(target);
    trace!(
        "→ illustration « {target} » : chemin recherché (absolu) = « {} »",
        candidate_absolute.display()
    );
    let Some(path) = resolve_source_path(base, target) else {
        return Err(format!(
            "Illustration « {target} » introuvable à « {} » (ni en absolu, ni en relatif, ni en sous-dossier de « {dir} »). Sélectionnez le fichier dans l'onglet Organisation.",
            candidate_absolute.display()
        ));
    };
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Err(format!(
                "Illustration « {} » illisible : {error}",
                path.display()
            ));
        }
    };
    let grayscale = ctx.payload.options.grayscale_images();
    let bytes = apply_image_color_mode(bytes, grayscale);
    // `docx-rs::Pic::new` **panique** (via `expect`) si les octets ne sont pas une
    // image exploitable : on valide donc en amont (logique identique à `docx-rs`)
    // pour renvoyer une **erreur explicite** au lieu d'interrompre l'export.
    if !image_is_rasterizable(&bytes) {
        return Err(format!(
            "Illustration « {} » non décodable : format d'image non pris en charge (PNG, JPEG, GIF, BMP, TIFF, WebP attendus).",
            path.display()
        ));
    }
    let pic = Pic::new(&bytes);
    let (width, height) = pic.size;
    if width == 0 || height == 0 {
        return Err(format!(
            "Illustration « {} » de dimensions nulles ({}×{} px) : elle ne serait pas affichée par Word.",
            path.display(),
            width,
            height
        ));
    }
    trace!(
        "✓ ILLUSTRATION « {target} » CHARGÉE depuis « {} » ({} octets, mode {})",
        path.display(),
        bytes.len(),
        if grayscale { "Noir & Blanc" } else { "Couleur" }
    );
    let max_width = ctx.content_w * EMU_PER_TWIP;
    let max_height = max_height_twips * EMU_PER_TWIP;
    // **Ajustement strict** : l'illustration est mise à l'échelle pour exploiter
    // la **largeur utile maximale** (page de coupe KDP − marges − gouttière) —
    // agrandissement comme réduction —, bornée par la hauteur disponible.
    let scale = f64::min(
        max_width as f64 / width as f64,
        max_height as f64 / height as f64,
    );
    let new_w = ((width as f64) * scale).round().max(1.0) as u32;
    let new_h = ((height as f64) * scale).round().max(1.0) as u32;
    Ok(pic.size(new_w, new_h))
}

/// Image de corps (`![alt](cible)`) : centrée horizontalement, isolée par un
/// espacement vertical, mise à l'échelle de la zone de texte utile.
///
/// **Échec explicite** : propage l'erreur de [`load_scaled_pic`] (image introuvable
/// ou non décodable) au lieu d'ignorer le bloc en silence.
pub(super) fn image_paragraph(target: &str, ctx: &BodyLayout) -> Result<Paragraph, String> {
    let reserved = IMAGE_SPACING_TWIPS * 2;
    let pic = load_scaled_pic(target, ctx, ctx.content_h.saturating_sub(reserved))?;
    Ok(Paragraph::new()
        .align(AlignmentType::Center)
        .line_spacing(
            LineSpacing::new()
                .before(IMAGE_SPACING_TWIPS)
                .after(IMAGE_SPACING_TWIPS),
        )
        .add_run(Run::new().add_image(pic)))
}

/// Illustration pleine page (nœud `image` de l'organisation) : image **centrée
/// horizontalement**, isolée par un espacement vertical, mise à l'échelle pour
/// tenir dans la zone d'impression utile.
///
/// **Échec explicite** : renvoie `Err` si l'illustration n'a pas de source liée ou
/// si son intégration échoue (aucune page blanche n'est créée en silence).
pub(super) fn image_spec(item: &FlatItem, ctx: &BodyLayout) -> Result<SectionSpec, String> {
    // Source liée ; à défaut, on tente le **nom affiché** comme nom de fichier
    // (l'image est alors retrouvée dans « Mes sources », y compris en sous-dossier).
    let linked = item
        .node
        .source_file_name
        .as_deref()
        .filter(|n| !n.trim().is_empty());
    let name = match linked.or_else(|| {
        item.node
            .display_name
            .as_deref()
            .filter(|n| !n.trim().is_empty())
    }) {
        Some(name) => name,
        None => {
            // Diagnostic : liste les illustrations réellement présentes dans « Mes sources ».
            if let Some(dir) = ctx
                .payload
                .directories
                .sources
                .as_deref()
                .filter(|d| !d.is_empty())
            {
                let mut available: Vec<String> = Vec::new();
                collect_image_files(std::path::Path::new(dir), &mut available);
                available.sort();
                if available.is_empty() {
                    trace!("   (aucun fichier image détecté dans « {dir} »)");
                } else {
                    trace!("   illustrations disponibles dans « {dir} » :");
                    for name in &available {
                        trace!("     - {name}");
                    }
                }
            }
            return Err(format!(
                "Illustration sans fichier source lié (displayName = {:?}) : sélectionnez un fichier dans l'onglet Organisation.",
                item.node.display_name
            ));
        }
    };
    // **Dimensionnement KDP strict** : l'image touche les bords de la **zone de
    // contenu** (largeur utile OU hauteur utile, ratio conservé ; `<wp:extent>`).
    let pic = load_scaled_pic(name, ctx, ctx.content_h)?;
    // Centrage **horizontal** par alignement ; centrage **vertical** par
    // `<w:vAlign w:val="center"/>` injecté dans le `sectPr` de cette section.
    let paragraph = Paragraph::new()
        .align(AlignmentType::Center)
        .add_run(Run::new().add_image(pic));
    // **Planche hors-texte** : isolée sur sa propre page (sauts de section
    // `oddPage` avant/après), sans en-tête ni folio, centrée verticalement.
    let mut spec = ctx.section(vec![paragraph], "");
    spec.running_label = None;
    spec.page_numbered = false;
    spec.vertical_center = true;
    Ok(spec)
}

/// Concatène le **libellé d'Organisation** (numérotation, ex. « Chapitre 1 ») et
/// le **titre `#`** du fichier (ex. « Le Dernier Soir ») → « Chapitre 1 - Le Dernier Soir ».
///
/// Repli : si l'un est vide, l'autre est conservé ; s'ils sont identiques (le
/// titre contient déjà le numéro), aucun doublon n'est ajouté.
pub(super) fn combined_chapter_title(name: Option<&str>, title: &str) -> String {
    let name = name.map(str::trim).filter(|n| !n.is_empty());
    let title = title.trim();
    match name {
        Some(name) if title.is_empty() => name.to_string(),
        Some(name) if !name.eq_ignore_ascii_case(title) => format!("{name} - {title}"),
        Some(name) => name.to_string(),
        None => title.to_string(),
    }
}

/// Rend un fichier source en blocs Word ; capture le premier titre `#` (en-tête).
pub(super) fn item_content(
    item: &FlatItem,
    ctx: &BodyLayout,
) -> Result<(Vec<Paragraph>, Option<String>), String> {
    let Some(text) = source_text(item, ctx.files) else {
        return Ok((Vec::new(), None));
    };
    let (body, _definitions) = markdown::extract_definitions(text);
    let body = if ctx.glossary_present {
        body
    } else {
        markdown::strip_refs(&body)
    };

    let mut paragraphs = Vec::new();
    let mut captured: Option<String> = None;
    let mut numbers: Vec<(String, u32)> = Vec::new();
    // Lettrine : la lettrine en attente est destinée au **tout premier
    // paragraphe de corps** (`Block::Paragraph`) qui suit un Titre 1 — ou le
    // début de la section (titre synthétique). Elle est **posée** par `Heading1`
    // et **consommée** par le premier `Paragraph`. Les blocs NON textuels
    // (sous-titre `##`, séparateur, citation, liste, image) ne doivent **pas** la
    // consommer : sinon un simple sous-titre de chapitre (`#` puis `##`) la
    // supprimerait purement et simplement.
    let mut drop_cap_pending = true;

    for block in markdown::parse_blocks(&body) {
        match block {
            Block::Heading1(text) => {
                let is_first = captured.is_none();
                // **Numérotation + nom** : restaure « Chapitre 1 - Le Dernier Soir »
                // en concaténant le libellé d'Organisation et le titre `#` du fichier.
                let heading = if is_first && item.kind == ItemKind::Chapter {
                    combined_chapter_title(item.node.display_name.as_deref(), &text)
                } else {
                    text.clone()
                };
                if is_first && !heading.trim().is_empty() {
                    captured = Some(heading.clone());
                }
                let mut para = heading_styled(&heading, ctx, 1, false);
                if is_first {
                    // **Isolement de la page de chapitre** : saut de page obligatoire
                    // *après* le titre (`<w:br w:type="page"/>`). Le corps démarre
                    // impérativement sur la page suivante, même si une illustration
                    // précède le chapitre (la planche est dans sa propre section).
                    para = para.add_run(Run::new().add_break(BreakType::Page));
                }
                paragraphs.push(para);
                // Un nouveau titre de chapitre **arme** la lettrine pour le
                // prochain paragraphe de corps.
                drop_cap_pending = true;
            }
            Block::Heading2(text) => {
                // **Règle structurelle** : tout sous-titre (`##`) déclenche un saut
                // de page **systématique** avant lui (parité PDF/ePUB).
                // NB : le sous-titre ne consomme **pas** la lettrine en attente
                // (elle reste destinée au premier paragraphe de corps).
                paragraphs.push(heading_styled(&text, ctx, 2, true));
            }
            Block::SceneBreak => {
                paragraphs.push(scene_break(ctx));
            }
            Block::Quote(text) => {
                let pieces = note_pieces(&text, ctx, &mut numbers);
                paragraphs.push(quote_paragraph(&pieces, ctx));
            }
            Block::ListItem { ordered, text } => {
                paragraphs.push(list_paragraph(&text, ordered, ctx, &mut numbers));
            }
            Block::Image { target } => {
                // Échec explicite : une illustration d'édition introuvable ou non
                // décodable interrompt l'export avec un message actionnable.
                paragraphs.push(image_paragraph(&target, ctx)?);
            }
            Block::Paragraph(text) => {
                let pieces = note_pieces(&text, ctx, &mut numbers);
                // La lettrine n'est appliquée que si le paragraphe couvre au moins
                // la hauteur du cadre (`DROP_CAP_LINES` lignes) ; sinon le cadre
                // déborderait sous le texte et Word ferait remonter le paragraphe
                // suivant autour de la lettrine (mise en page détruite).
                let apply_drop = drop_cap_pending && drop_cap_fits(&text, ctx);
                paragraphs.push(body_paragraph(&pieces, ctx, apply_drop));
                // La lettrine est **consommée** par le premier paragraphe (qu'elle
                // soit appliquée ou non) : elle ne doit jamais affecter les suivants.
                drop_cap_pending = false;
            }
        }
    }

    Ok((paragraphs, captured))
}

/// Acte : page de titre de partie (**style Titre 1**).
pub(super) fn act_spec(item: &FlatItem, ctx: &BodyLayout) -> SectionSpec {
    let label = item_label(item);
    let paragraph = Paragraph::new()
        .style("Heading1")
        .align(AlignmentType::Center)
        .outline_lvl(0)
        .line_spacing(LineSpacing::new().before(5040))
        .add_run(
            Run::new()
                .add_text(label.to_uppercase())
                .size(56)
                .bold()
                .fonts(fonts(ctx.font)),
        );
    ctx.section(vec![paragraph], &label)
}

/// Chapitre (ou page spéciale textuelle) : titre + corps.
pub(super) fn text_spec(item: &FlatItem, ctx: &BodyLayout) -> Result<SectionSpec, String> {
    let (paragraphs, captured) = item_content(item, ctx)?;
    let title = captured.filter(|text| !text.trim().is_empty());
    // En-tête courant = **nom du chapitre saisi dans « Organisation »**
    // (`displayName`). À défaut seulement, on retombe sur le titre `#` du fichier
    // source, puis sur un libellé générique.
    let org_name = item
        .node
        .display_name
        .clone()
        .filter(|name| !name.trim().is_empty());
    let label = org_name
        .or_else(|| title.clone())
        .unwrap_or_else(|| item_label(item));
    let mut all = Vec::new();
    // Sans titre `#` en tête de source, on conserve le libellé saisi comme titre.
    // Un libellé **vide** (page spéciale sans rôle reconnu) ne produit aucun titre
    // parasite.
    if title.is_none() {
        let fallback = item_label(item);
        if !fallback.trim().is_empty() {
            all.push(heading_paragraph(&fallback, ctx, 32));
        }
    }
    all.extend(paragraphs);
    Ok(ctx.section(all, &label))
}

/// Page spéciale : formatage adapté au rôle.
pub(super) fn special_spec(item: &FlatItem, ctx: &BodyLayout) -> Result<SectionSpec, String> {
    match item.node.role.as_deref() {
        Some("glossary") => Ok(glossary_spec(item, ctx)),
        Some("dedication") => Ok(dedication_spec(item, ctx)),
        _ => text_spec(item, ctx),
    }
}

/// Page « Glossaire » : définitions regroupées par chapitre d'origine.
pub(super) fn glossary_spec(item: &FlatItem, ctx: &BodyLayout) -> SectionSpec {
    let label = item_label(item);
    let mut paragraphs = vec![heading_paragraph(&label, ctx, 32)];

    if ctx.glossary.is_empty() {
        paragraphs.push(
            Paragraph::new().align(AlignmentType::Center).add_run(
                Run::new()
                    .add_text("Aucune entrée.")
                    .size(ctx.body_size)
                    .italic()
                    .fonts(fonts(ctx.font)),
            ),
        );
    }

    for (index, chapter) in ctx.glossary.iter().enumerate() {
        // **Séparateur de scène entre les groupes** de chapitres (aération), jamais
        // avant le tout premier groupe.
        if index > 0 {
            paragraphs.push(scene_break(ctx));
        }
        // Sous-titre de regroupement (« Acte 2, chapitre 7 ») en style Titre 2.
        paragraphs.push(
            Paragraph::new()
                .style("Heading2")
                .outline_lvl(1)
                .line_spacing(LineSpacing::new().before(480).after(120))
                .add_run(
                    Run::new()
                        .add_text(chapter.label.clone())
                        .size(28)
                        .bold()
                        .fonts(fonts(ctx.font)),
                ),
        );
        for entry in &chapter.entries {
            paragraphs.push(glossary_entry(entry, ctx));
        }
    }

    ctx.section(paragraphs, &label)
}

/// Dédicace : alignée à droite, italique, repoussée dans le tiers inférieur.
pub(super) fn dedication_spec(item: &FlatItem, ctx: &BodyLayout) -> SectionSpec {
    let label = item_label(item);
    let Some(text) = source_text(item, ctx.files) else {
        return ctx.section(Vec::new(), &label);
    };
    let (body, _definitions) = markdown::extract_definitions(text);
    let lines = markdown::split_paragraphs(&body);
    let mut paragraphs: Vec<Paragraph> = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        let mut spacing = LineSpacing::new()
            .line(ctx.line)
            .line_rule(LineSpacingType::Auto);
        if index == 0 {
            // Repousse le bloc dans le tiers inférieur de la page.
            spacing = spacing.before(DEDICATION_SPACING_BEFORE);
        }
        let mut paragraph = Paragraph::new()
            .align(AlignmentType::Right)
            .line_spacing(spacing);
        for span in markdown::parse_inline(line) {
            let run = inline_run(&span, ctx.font, ctx.body_size).italic();
            paragraph = paragraph.add_run(run);
        }
        paragraphs.push(paragraph);
    }

    ctx.section(paragraphs, &label)
}

/// Entrée de glossaire : numéro en exposant + **terme en gras** + « : » + définition.
pub(super) fn glossary_entry(entry: &GlossaryEntry, ctx: &BodyLayout) -> Paragraph {
    let mut paragraph = Paragraph::new()
        .align(body_alignment(&ctx.payload.layout_config))
        .line_spacing(
            LineSpacing::new()
                .line(ctx.line)
                .line_rule(LineSpacingType::Auto),
        )
        .indent(
            Some(FIRST_LINE_TWIPS),
            Some(SpecialIndentType::Hanging(FIRST_LINE_TWIPS)),
            None,
            None,
        );

    // Numéro en exposant, puis espace.
    paragraph = paragraph.add_run(superscript_run(entry.number, ctx.font, ctx.body_size));
    paragraph = paragraph.add_run(
        Run::new()
            .add_text(" ")
            .size(ctx.body_size)
            .fonts(fonts(ctx.font)),
    );

    // **Terme en gras suivi de « : »**, toujours présent dès que le terme est connu
    // (dérivé du libellé de note ou d'un préfixe « Terme : » explicite).
    if !entry.term.is_empty() {
        paragraph = paragraph.add_run(
            Run::new()
                .add_text(format!("{} :", entry.term))
                .size(ctx.body_size)
                .bold()
                .fonts(fonts(ctx.font)),
        );
        paragraph = paragraph.add_run(
            Run::new()
                .add_text(" ")
                .size(ctx.body_size)
                .fonts(fonts(ctx.font)),
        );
    }

    for span in markdown::parse_inline(&entry.definition) {
        paragraph = paragraph.add_run(inline_run(&span, ctx.font, ctx.body_size));
    }

    paragraph
}

/// Sépare le « terme » (avant les deux-points) de sa définition, si non formaté.
pub(super) fn split_term(definition: &str) -> Option<(&str, &str)> {
    let position = definition.find(':')?;
    let prefix = definition[..position].trim();
    if prefix.is_empty() || prefix.len() > 40 || prefix.contains('*') {
        return None;
    }
    Some((prefix, definition[position + 1..].trim_start()))
}

/// Numérotation à puces (abstractNumId 1, numId 1).
pub(super) fn bullet_numbering() -> AbstractNumbering {
    AbstractNumbering::new(1).add_level(
        Level::new(
            0,
            Start::new(1),
            NumberFormat::new("bullet"),
            LevelText::new("\u{2022}"),
            LevelJc::new("left"),
        )
        .indent(Some(283), Some(SpecialIndentType::Hanging(283)), None, None),
    )
}

/// Numérotation décimale (abstractNumId 2, numId 2).
pub(super) fn decimal_numbering() -> AbstractNumbering {
    AbstractNumbering::new(2).add_level(
        Level::new(
            0,
            Start::new(1),
            NumberFormat::new("decimal"),
            LevelText::new("%1."),
            LevelJc::new("left"),
        )
        .indent(Some(283), Some(SpecialIndentType::Hanging(283)), None, None),
    )
}

/// Champ de table des matières natif (Titres 1 et 2), recalculé par Word.
pub(super) fn toc_paragraph(ctx: &BodyLayout) -> Paragraph {
    let instr = InstrToC::new().heading_styles_range(1, 2).hyperlink();
    Paragraph::new()
        .line_spacing(
            LineSpacing::new()
                .line(ctx.line)
                .line_rule(LineSpacingType::Auto),
        )
        .add_run(Run::new().add_field_char(FieldCharType::Begin, true))
        .add_run(Run::new().add_instr_text(InstrText::TOC(instr)))
        .add_run(Run::new().add_field_char(FieldCharType::Separate, false))
        .add_run(
            Run::new()
                .add_text("Sommaire — appuyez sur F9 pour actualiser les numéros de page.")
                .size(ctx.body_size)
                .italic()
                .fonts(fonts(ctx.font)),
        )
        .add_run(Run::new().add_field_char(FieldCharType::End, false))
}

/// Section « Table des matières » (page impaire, listant les Titres 1 et 2).
pub(super) fn toc_spec(ctx: &BodyLayout) -> SectionSpec {
    let paragraphs = vec![
        Paragraph::new()
            .align(AlignmentType::Center)
            .line_spacing(LineSpacing::new().after(480))
            .add_run(
                Run::new()
                    .add_text("Table des matières")
                    .size(36)
                    .bold()
                    .fonts(fonts(ctx.font)),
            ),
        toc_paragraph(ctx),
    ];
    let mut spec = ctx.section(paragraphs, "Sommaire");
    // La table des matières n'est ni en-têtée ni foliotée.
    spec.running_label = None;
    spec.footers = Some(no_footers());
    spec.page_numbered = false;
    spec
}

/// Achevé d'imprimer (colophon), en toute fin d'ouvrage (saut de page simple).
pub(super) fn colophon_spec(md: &ProjectMetadata, ctx: &BodyLayout) -> SectionSpec {
    let year = if md.year.trim().is_empty() {
        current_year().to_string()
    } else {
        md.year.trim().to_string()
    };
    let place = if md.print_location.trim().is_empty() {
        "France".to_string()
    } else {
        md.print_location.trim().to_string()
    };
    let paragraphs = vec![
        Paragraph::new()
            .align(AlignmentType::Center)
            .line_spacing(LineSpacing::new().before(6000))
            .add_run(
                Run::new()
                    .add_text(format!("Achevé d'imprimer en {year} par {place}."))
                    .size(20)
                    .fonts(fonts(ctx.font)),
            ),
        Paragraph::new()
            .align(AlignmentType::Center)
            .line_spacing(LineSpacing::new().before(120))
            .add_run(
                Run::new()
                    .add_text(format!("Dépôt légal : {year}."))
                    .size(20)
                    .fonts(fonts(ctx.font)),
            ),
    ];
    let mut spec = ctx.section(paragraphs, md.book_title.trim());
    // L'achevé d'imprimer (colophon) n'est ni en-têté ni folioté.
    spec.running_label = None;
    spec.footers = Some(no_footers());
    spec.page_numbered = false;
    spec
}
