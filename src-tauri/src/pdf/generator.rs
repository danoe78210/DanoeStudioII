//! Traducteur **AST Markdown → Typst** : blocs, mise en forme en ligne, notes,
//! lettrines, sommaire et glossaire.
//!
//! La géométrie (dimensions, gouttière, marges miroir) provient exclusivement de
//! [`crate::kdp`]. Les textes sont injectés comme **littéraux** Typst (`"..."`),
//! donc aucun balisage n'est interprété par erreur : seul l'échappement de
//! chaîne est nécessaire.
//!
//! Les **réglages typographiques** de l'UI (`LayoutConfig`) sont câblés :
//! police/taille du corps, justification, interligne, police/taille des titres
//! de chapitre (`level: 1`) et des sous-titres (`level: 2`). Chaque pile de
//! polices se termine par la police embarquée (repli garanti, jamais de panique).

use crate::kdp::KdpPageSpec;
use crate::markdown::{self, parse_inline, Block, BodyPiece, Inline};

/// Police par défaut (fournie par les polices embarquées `typst-assets`).
pub const DEFAULT_FONT_FAMILY: &str = "Libertinus Serif";
/// Taille du corps de texte par défaut (points).
pub const BODY_SIZE_PT: f64 = 11.0;
/// Interligne Typst implicite (~0,65 em) lorsque la config vaut 1.
const DEFAULT_LEADING_EM: f64 = 0.65;

/// Réglages typographiques issus de l'UI (`LayoutConfig`).
pub struct Typography<'a> {
    /// Police du corps de texte.
    pub body_font: &'a str,
    /// Taille du corps (points).
    pub body_size: f64,
    /// Interligne (multiplicateur, ex. 1.15).
    pub line_spacing: f64,
    /// Corps justifié (`true`) ou aligné à gauche (`false`).
    pub justify: bool,
    /// Lettrine active sur le premier paragraphe de chaque chapitre.
    pub drop_cap: bool,
    /// Police du titre de chapitre (niveau 1).
    pub chapter_font: &'a str,
    /// Taille du titre de chapitre (points).
    pub chapter_size: f64,
    /// Police des sous-titres (niveau 2).
    pub subtitle_font: &'a str,
    /// Taille des sous-titres (points).
    pub subtitle_size: f64,
}

/// Entrée de glossaire : numéro de renvoi, terme mis en évidence et définition.
pub struct GlossaryEntry {
    pub number: u32,
    pub term: String,
    pub definition: String,
}

/// Chapitre du glossaire (regroupement par chapitre d'origine).
pub struct GlossaryChapter {
    pub label: String,
    pub entries: Vec<GlossaryEntry>,
}

/// Document à rendre : métadonnées (en-têtes) + blocs Markdown de l'AST.
pub struct PdfDoc<'a> {
    /// Titre de l'œuvre (en-tête des pages impaires).
    pub title: &'a str,
    /// Nom de l'auteur (en-tête des pages paires).
    pub author: &'a str,
    /// Blocs de l'AST Markdown (`crate::markdown::parse_blocks`).
    pub blocks: &'a [Block],
    /// Réglages typographiques (UI → PDF).
    pub typography: &'a Typography<'a>,
    /// La page spéciale « Glossaire » est-elle présente dans l'arborescence ?
    pub glossary_present: bool,
    /// Entrées du glossaire final (cas « glossaire présent »).
    pub glossary: &'a [GlossaryChapter],
}

/// Convertit des twips (unité KDP) en points Typst (1 twip = 1/20 pt).
pub fn twips_to_points(twips: u32) -> f64 {
    twips as f64 / 20.0
}

/// Échappe une chaîne pour un **littéral** Typst (`"..."`).
fn lit(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Construit une **pile de polices** Typst terminée par la police embarquée.
///
/// Typst essaie les polices dans l'ordre : si la police demandée est absente du
/// système, il retombe sur `Libertinus Serif` (embarquée). Aucune panique.
fn font_stack(name: &str) -> String {
    let mut names: Vec<String> = Vec::new();
    for candidate in [name, DEFAULT_FONT_FAMILY] {
        let candidate = candidate.trim();
        if !candidate.is_empty() && !names.iter().any(|n| n == candidate) {
            names.push(candidate.to_string());
        }
    }
    let inner = names.iter().map(|n| lit(n)).collect::<Vec<_>>().join(", ");
    if names.len() == 1 {
        // Un tableau Typst à un seul élément exige une virgule finale.
        format!("({inner},)")
    } else {
        format!("({inner})")
    }
}

/// Rend une séquence d'`Inline` en contenu Typst (`#text(...)`), gras/italique.
fn inline_content(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for span in inlines {
        if span.text.is_empty() {
            continue;
        }
        out.push_str(&format!("#text({}", lit(&span.text)));
        if span.bold {
            out.push_str(", weight: \"bold\"");
        }
        if span.italic {
            out.push_str(", style: \"italic\"");
        }
        out.push(')');
    }
    out
}

/// Pièces (texte + renvois) → contenu Typst ; un renvoi devient `#super[n]`.
fn pieces_content(pieces: &[BodyPiece]) -> String {
    let mut out = String::new();
    for piece in pieces {
        match piece {
            BodyPiece::Text(text) => out.push_str(&inline_content(&parse_inline(text))),
            BodyPiece::NoteRef(number) => out.push_str(&format!("#super[{number}]")),
        }
    }
    out
}

/// Pièces d'un texte selon la **règle du livre** :
/// - glossaire présent → renvois numérotés en exposant (par chapitre) ;
/// - sinon → texte brut (les balises `[^…]` ont déjà été retirées en amont).
fn paragraph_pieces(
    text: &str,
    numbers: &mut Vec<(String, u32)>,
    glossary_present: bool,
) -> Vec<BodyPiece> {
    if glossary_present {
        markdown::number_refs(text, numbers)
    } else {
        vec![BodyPiece::Text(text.to_string())]
    }
}

/// Paragraphe, avec **lettrine** (drop cap) optionnelle sur sa première lettre.
///
/// La lettrine est rendue par `#dropcap(letter)[body]` : le balisage est
/// **jointif** (l'appel et le corps se suivent sans espace), ce qui évite tout
/// caractère d'espace parasite après l'initiale (« L a flamme » → « La flamme »).
fn paragraph_content(pieces: &[BodyPiece], drop_cap: bool) -> String {
    if !drop_cap {
        return format!("{}\n\n", pieces_content(pieces));
    }
    let Some(index) = pieces
        .iter()
        .position(|p| matches!(p, BodyPiece::Text(t) if !t.is_empty()))
    else {
        return format!("{}\n\n", pieces_content(pieces));
    };
    let mut pieces = pieces.to_vec();
    let BodyPiece::Text(text) = &pieces[index] else {
        return format!("{}\n\n", pieces_content(&pieces));
    };
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return format!("{}\n\n", pieces_content(&pieces));
    };
    let letter = first.to_string();
    pieces[index] = BodyPiece::Text(chars.as_str().to_string());
    format!("#dropcap({})[{}]\n\n", lit(&letter), pieces_content(&pieces))
}

/// Traduit les blocs de l'AST en corps Typst (notes incluses).
fn render_blocks(blocks: &[Block], typo: &Typography, glossary_present: bool) -> String {
    let mut out = String::new();
    // Numérotation des notes : réinitialisée à chaque titre de chapitre (`level: 1`).
    let mut numbers: Vec<(String, u32)> = Vec::new();
    let mut drop_cap_pending = typo.drop_cap;
    for block in blocks {
        match block {
            Block::Heading1(title) => {
                // Le saut de page et le centrage H+V sont assurés par la
                // show-rule `level: 1` (voir `generate`).
                out.push_str(&format!(
                    "#heading(level: 1)[{}]\n\n",
                    inline_content(&parse_inline(title))
                ));
                numbers.clear();
                drop_cap_pending = typo.drop_cap;
            }
            Block::Heading2(title) => {
                out.push_str(&format!(
                    "#heading(level: 2)[{}]\n\n",
                    inline_content(&parse_inline(title))
                ));
            }
            Block::Paragraph(text) => {
                let pieces = paragraph_pieces(text, &mut numbers, glossary_present);
                let empty = pieces
                    .iter()
                    .all(|p| matches!(p, BodyPiece::Text(t) if t.trim().is_empty()));
                let drop_cap = drop_cap_pending && !empty;
                drop_cap_pending = false;
                out.push_str(&paragraph_content(&pieces, drop_cap));
            }
            Block::Quote(text) => {
                let pieces = paragraph_pieces(text, &mut numbers, glossary_present);
                out.push_str(&format!(
                    "#quote(block: true)[{}]\n\n",
                    pieces_content(&pieces)
                ));
            }
            Block::ListItem { ordered, text } => {
                let marker = if *ordered { "+" } else { "-" };
                let pieces = paragraph_pieces(text, &mut numbers, glossary_present);
                out.push_str(&format!("{} {}\n", marker, pieces_content(&pieces)));
            }
            Block::SceneBreak => out.push_str("#align(center)[\\* \\* \\*]\n\n"),
            Block::Image { target } => out.push_str(&format!(
                "#pagebreak(weak: false)\n#block(width: 100%, height: 100%, align(center + horizon, figure(image({}, width: 100%))))\n\n",
                lit(target)
            )),
        }
    }
    out
}

/// Glossaire final unifié : chapitres regroupés, entrées numérotées en exposant.
fn render_glossary(glossary: &[GlossaryChapter]) -> String {
    if glossary.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str("#pagebreak(weak: true)\n");
    out.push_str("#heading(level: 1)[Glossaire]\n\n");
    for (index, chapter) in glossary.iter().enumerate() {
        // Séparateur de scène entre les groupes (jamais avant le premier).
        if index > 0 {
            out.push_str("#align(center)[\\* \\* \\*]\n\n");
        }
        out.push_str(&format!(
            "#heading(level: 2)[{}]\n\n",
            inline_content(&parse_inline(&chapter.label))
        ));
        for entry in &chapter.entries {
            out.push_str(&format!(
                "#super[{}] #text({}, weight: \"bold\") : {}\n\n",
                entry.number,
                lit(&entry.term),
                inline_content(&parse_inline(&entry.definition)),
            ));
        }
    }
    out
}

/// Génère le balisage Typst complet (page KDP, en-têtes, folio, sommaire, contenu).
pub fn generate(page: &KdpPageSpec, doc: &PdfDoc) -> String {
    let width = twips_to_points(page.page_width_twips);
    let height = twips_to_points(page.page_height_twips);
    let top = twips_to_points(page.margin_top_twips);
    let bottom = twips_to_points(page.margin_bottom_twips);
    let outside = twips_to_points(page.margin_side_twips);
    let inside = twips_to_points(page.margin_side_twips + page.gutter_twips);

    // Réglages typographiques (UI → PDF) avec repli embarqué garanti.
    let typo = doc.typography;
    let body_stack = font_stack(typo.body_font);
    let chapter_stack = font_stack(typo.chapter_font);
    let subtitle_stack = font_stack(typo.subtitle_font);
    let body_size = if typo.body_size > 0.0 {
        typo.body_size
    } else {
        BODY_SIZE_PT
    };
    let chapter_size = if typo.chapter_size > 0.0 {
        typo.chapter_size
    } else {
        16.0
    };
    let subtitle_size = if typo.subtitle_size > 0.0 {
        typo.subtitle_size
    } else {
        14.0
    };
    // Interligne : 0,65 em implicite + le surplus demandé (multiplicateur − 1).
    let leading = (typo.line_spacing - 1.0).max(0.0) + DEFAULT_LEADING_EM;

    let mut out = String::new();
    // En-tête courant = **titre du chapitre en cours** (dernier titre de niveau 1
    // au-dessus de la page), centré, souligné d'un trait continu — parité avec
    // l'en-tête Word (`w:pBdr`/`w:bottom`).
    out.push_str(&format!(
        "#set page(\n  width: {width}pt,\n  height: {height}pt,\n  margin: (top: {top}pt, bottom: {bottom}pt, inside: {inside}pt, outside: {outside}pt),\n  header: context {{\n    let page = here().page()\n    let heads = query(heading.where(level: 1))\n    let structural = heads.any(h => h.location().page() == page) or query(figure).any(f => f.location().page() == page)\n    if not structural {{\n      let shown = heads.filter(h => h.location().page() < page)\n      if shown.len() > 0 {{\n        set text(size: 8pt)\n        stack(dir: ttb, spacing: 0.6em, align(center, shown.last().body), line(length: 100%, stroke: 0.4pt))\n      }}\n    }}\n  }},\n  footer: context {{\n    set text(size: 9pt)\n    align(center, counter(page).display())\n  }},\n)\n"
    ));
    // Corps : police/taille + justification + interligne.
    out.push_str(&format!(
        "#set text(font: {body_stack}, size: {body_size}pt)\n"
    ));
    out.push_str(&format!(
        "#set par(justify: {}, leading: {leading:.2}em)\n",
        typo.justify
    ));
    // Titres de chapitre (niveau 1) et sous-titres (niveau 2).
    out.push_str(&format!(
        "#show heading.where(level: 1): set text(font: {chapter_stack}, size: {chapter_size}pt)\n"
    ));
    out.push_str(&format!(
        "#show heading.where(level: 2): set text(font: {subtitle_stack}, size: {subtitle_size}pt)\n"
    ));
    // **Règles d'aération structurelles (source unique de vérité)** :
    // - Titre de niveau 1 (Acte/Chapitre) : page entière, seul, centré H+V ;
    // - Titre de niveau 2 (sous-titre) : saut de page systématique avant.
    // On conserve `it` (l'élément) pour que le sommaire (`#outline`) le collecte.
    out.push_str(
        "#show heading.where(level: 1): it => [\n  #pagebreak(weak: true)\n  #block(width: 100%, height: 100%, align(center + horizon, it))\n]\n",
    );
    out.push_str(
        "#show heading.where(level: 2): it => [\n  #pagebreak(weak: true)\n  #it\n]\n\n",
    );
    // Lettrine : balisage **jointif** (aucun espace parasite après l'initiale).
    out.push_str(
        "#let dropcap(letter, body) = [#text(size: 2.4em, weight: \"bold\")[#letter]#body]\n\n",
    );
    // Sommaire (parité avec le champ TOC natif de l'export Word).
    out.push_str(
        "#outline(title: [Table des matières], depth: 2, indent: 1.2em)\n#pagebreak(weak: false)\n\n",
    );
    out.push_str(&render_blocks(doc.blocks, typo, doc.glossary_present));
    out.push_str(&render_glossary(doc.glossary));
    out
}
