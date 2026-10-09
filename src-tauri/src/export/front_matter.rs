//! Pages liminaires : couverture, faux-titre, page de titre et copyright,
//! avec les calculs d'ancrage des blocs de bas de page.

use docx_rs::*;

use crate::kdp::{BOTTOM_MARGIN_TWIPS, EMU_PER_TWIP, TOP_MARGIN_TWIPS};
use crate::project::{ProjectMetadata, ProjectPayload};

use super::helpers::*;
use super::SectionSpec;
/// Couleur du texte superposé à la couverture (beige/blanc cassé lisible sur cuir sombre).
pub(super) const COVER_TEXT_COLOR: &str = "F5F5DC";

/// Marge de sécurité (twips) laissée au-dessus de la marge inférieure lors de
/// l'ancrage des blocs de bas de page (copyright, mention d'édition). Les hauteurs
/// de ligne étant **figées** (`lineRule="exact"`), le calcul est déterministe.
pub(super) const BOTTOM_ANCHOR_SAFETY: u32 = 60;
/// Décalage de tête (twips) de la page de titre principale (page 3).
pub(super) const TITLE_PAGE_TOP_OFFSET: u32 = 2880;

/// Section de **couverture** (image de fond pleine page + texte superposé).
///
/// Utilisée uniquement lorsque `include_cover` est vrai : le fichier **intérieur**
/// KDP Print n'embarque jamais la couverture (PDF de couverture séparé).
pub(super) fn cover_spec(
    payload: &ProjectPayload,
    cover: Option<&[u8]>,
    font: &str,
    page_w: u32,
    page_h: u32,
) -> SectionSpec {
    let md = &payload.metadata;

    // Image de fond (flottante, page entière) + texte superposé.
    let mut cover_para = Paragraph::new()
        .align(AlignmentType::Center)
        .line_spacing(LineSpacing::new().before(2880));

    if let Some(buf) = cover {
        let pic = Pic::new(buf)
            .size(page_w * EMU_PER_TWIP, page_h * EMU_PER_TWIP)
            .floating()
            .relative_from_h(RelativeFromHType::Page)
            .relative_from_v(RelativeFromVType::Page)
            .offset_x(0)
            .offset_y(0)
            .dist_t(0)
            .dist_b(0)
            .dist_l(0)
            .dist_r(0);
        cover_para = cover_para.add_run(Run::new().add_image(pic));
    }

    cover_para = cover_para.add_run(
        Run::new()
            .add_text(md.book_title.trim().to_uppercase())
            .size(84)
            .bold()
            .color(COVER_TEXT_COLOR)
            .fonts(fonts(font)),
    );

    let mut cover_paras = vec![cover_para];
    if !md.subtitle.trim().is_empty() {
        cover_paras.push(
            Paragraph::new()
                .align(AlignmentType::Center)
                .line_spacing(LineSpacing::new().before(240))
                .add_run(
                    Run::new()
                        .add_text(md.subtitle.trim().to_string())
                        .size(36)
                        .italic()
                        .color(COVER_TEXT_COLOR)
                        .fonts(fonts(font)),
                ),
        );
    }
    if !md.author_name.trim().is_empty() {
        cover_paras.push(
            Paragraph::new()
                .align(AlignmentType::Center)
                .line_spacing(LineSpacing::new().before(5040))
                .add_run(
                    Run::new()
                        .add_text(md.author_name.trim().to_string())
                        .size(48)
                        .color(COVER_TEXT_COLOR)
                        .fonts(fonts(font)),
                ),
        );
    }

    SectionSpec {
        paragraphs: cover_paras,
        page_w,
        page_h,
        // Couverture : plein bord, marges et gouttière nulles.
        margin: PageMargin::new()
            .top(0)
            .bottom(0)
            .left(0)
            .right(0)
            .header(0)
            .footer(0)
            .gutter(0),
        title_pg: false,
        start_page: None,
        running_label: None,
        footers: None,
        page_numbered: false,
        vertical_center: false,
    }
}

/// Hauteur de ligne (twips) pour une taille exprimée en demi-points, à ~1,2× la
/// taille de police. Les paragraphes ancrés en bas de page utilisent cette valeur
/// comme interligne **exact** (`lineRule="exact"`) : la hauteur est alors figée et
/// le calcul d'ancrage devient déterministe (indépendant de la fonte).
pub(super) fn line_height_twips(size_half_points: usize) -> u32 {
    (size_half_points as u32) * 12
}

/// Interligne **exact** pour un paragraphe ancré en bas de page : espacement avant
/// `before` + hauteur de ligne figée (`lineRule="exact"`).
pub(super) fn anchored_spacing(before: u32, size_half_points: usize) -> LineSpacing {
    LineSpacing::new()
        .before(before)
        .line(line_height_twips(size_half_points) as i32)
        .line_rule(LineSpacingType::Exact)
}

/// Paragraphe **espaceur** invisible occupant une hauteur verticale exacte.
///
/// Contrairement à l'espacement avant (`w:before`), une **hauteur de ligne**
/// (`w:line` + `lineRule="exact"`) n'est jamais supprimée en tête de page : c'est
/// ce qui garantit un ancrage bas de page fiable, quel que soit le lecteur.
/// `page_break` place l'espaceur en tête de page (début de page liminaire).
pub(super) fn spacer(height_twips: u32, page_break: bool) -> Paragraph {
    let mut paragraph = Paragraph::new().line_spacing(
        LineSpacing::new()
            .line(height_twips as i32)
            .line_rule(LineSpacingType::Exact),
    );
    if page_break {
        paragraph = paragraph.page_break_before(true);
    }
    paragraph
}

/// Pages liminaires : faux-titre, page de titre, page de copyright.
///
/// Les blocs « bas de page » (copyright page 2, mention d'édition page 3) sont
/// ancrés en calculant leur espacement avant (`before`) à partir de la **hauteur
/// utile** de la page : ils viennent se plaquer contre la marge inférieure, quelle
/// que soit la présence du sous-titre, de la saga ou de l'auteur.
pub(super) fn build_front_matter(md: &ProjectMetadata, font: &str, page_h: u32) -> Vec<Paragraph> {
    let title = md.book_title.trim().to_string();
    let title_upper = title.to_uppercase();
    let subtitle = md.subtitle.trim().to_string();
    let author = md.author_name.trim().to_string();
    let saga = md.saga_title.trim().to_string();
    // Tome (facultatif) : affiché « Tome N » sous le titre du livre.
    let volume = md.volume_number.trim().to_string();
    let publisher = if md.publisher.trim().is_empty() {
        "Autoédition".to_string()
    } else {
        md.publisher.trim().to_string()
    };
    let year = if md.year.trim().is_empty() {
        current_year().to_string()
    } else {
        md.year.trim().to_string()
    };

    // Hauteur utile de la page (marges haute et basse déduites).
    let content_h = page_h.saturating_sub(TOP_MARGIN_TWIPS as u32 + BOTTOM_MARGIN_TWIPS as u32);

    let mut paragraphs = Vec::new();

    // --- Page 1 : faux-titre (saga + titre, tiers supérieur de la page). ---
    if !saga.is_empty() {
        paragraphs.push(
            Paragraph::new()
                .align(AlignmentType::Center)
                .line_spacing(LineSpacing::new().before(4320))
                .add_run(
                    Run::new()
                        .add_text(saga.to_uppercase())
                        .size(22)
                        .fonts(fonts(font)),
                ),
        );
    }
    paragraphs.push(
        Paragraph::new()
            .align(AlignmentType::Center)
            .line_spacing(LineSpacing::new().before(if saga.is_empty() { 4320 } else { 360 }))
            .add_run(Run::new().add_text(title_upper).size(48).fonts(fonts(font))),
    );

    // Tome : « Tome N » sur la ligne sous le titre du faux-titre (page 1).
    if !volume.is_empty() {
        paragraphs.push(
            Paragraph::new()
                .align(AlignmentType::Center)
                .line_spacing(LineSpacing::new().before(240))
                .add_run(
                    Run::new()
                        .add_text(format!("Tome {volume}"))
                        .size(28)
                        .fonts(fonts(font)),
                ),
        );
    }

    // --- Page 2 (gauche, dos du faux-titre) : copyright — bloc centré, ancré bas de page. ---
    let copyright = if md.copyright_text.trim().is_empty() {
        format!("© {year} {author}. Tous droits réservés.")
    } else {
        md.copyright_text.trim().to_string()
    };
    // Hauteur réservée au bloc (mention + ISBN éventuel + édition) : l'espacement
    // avant repousse le bloc tout en bas sans déborder sur la page suivante.
    let has_isbn = !md.isbn.trim().is_empty();
    let copyright_block = line_height_twips(20)
        + if has_isbn {
            180 + line_height_twips(20)
        } else {
            0
        }
        + 180
        + line_height_twips(20);
    let copyright_before = content_h.saturating_sub(BOTTOM_ANCHOR_SAFETY + copyright_block);
    // Espaceur de tête (saut de page) : garantit l'ancrage bas de page fiable.
    paragraphs.push(spacer(copyright_before, true));
    paragraphs.push(
        Paragraph::new()
            .align(AlignmentType::Center)
            .line_spacing(anchored_spacing(0, 20))
            .add_run(Run::new().add_text(copyright).size(20).fonts(fonts(font))),
    );

    if has_isbn {
        paragraphs.push(
            Paragraph::new()
                .align(AlignmentType::Center)
                .line_spacing(anchored_spacing(180, 20))
                .add_run(
                    Run::new()
                        .add_text(format!("ISBN : {}", md.isbn.trim()))
                        .size(20)
                        .fonts(fonts(font)),
                ),
        );
    }

    let mut edition = publisher.clone();
    if !md.print_location.trim().is_empty() {
        edition = format!("{edition} · Imprimé en {}", md.print_location.trim());
    }
    paragraphs.push(
        Paragraph::new()
            .align(AlignmentType::Center)
            .line_spacing(anchored_spacing(180, 20))
            .add_run(Run::new().add_text(edition).size(20).fonts(fonts(font))),
    );

    // --- Page 3 (droite) : page de titre principale (saga + titre, sous-titre, auteur, mention d'édition en bas). ---
    // `title_used` cumule la hauteur du bloc de titre (espacements avant + lignes)
    // afin de calculer l'espacement qui plaque la mention d'édition tout en bas.
    // Espaceur de tête (saut de page) : fiable même si l'espacement avant est
    // supprimé en tête de page.
    paragraphs.push(spacer(TITLE_PAGE_TOP_OFFSET, true));
    let mut title_used: u32 = TITLE_PAGE_TOP_OFFSET;
    if !saga.is_empty() {
        title_used += line_height_twips(22);
        paragraphs.push(
            Paragraph::new()
                .align(AlignmentType::Center)
                .line_spacing(anchored_spacing(0, 22))
                .add_run(
                    Run::new()
                        .add_text(saga.to_uppercase())
                        .size(22)
                        .fonts(fonts(font)),
                ),
        );
        title_used += 360 + line_height_twips(60);
        paragraphs.push(
            Paragraph::new()
                .align(AlignmentType::Center)
                .line_spacing(anchored_spacing(360, 60))
                .add_run(
                    Run::new()
                        .add_text(title)
                        .size(60)
                        .bold()
                        .fonts(fonts(font)),
                ),
        );
    } else {
        title_used += line_height_twips(60);
        paragraphs.push(
            Paragraph::new()
                .align(AlignmentType::Center)
                .line_spacing(anchored_spacing(0, 60))
                .add_run(
                    Run::new()
                        .add_text(title)
                        .size(60)
                        .bold()
                        .fonts(fonts(font)),
                ),
        );
    }

    // Tome : « Tome N » sur la ligne sous le titre de la page de titre principale.
    if !volume.is_empty() {
        title_used += 240 + line_height_twips(32);
        paragraphs.push(
            Paragraph::new()
                .align(AlignmentType::Center)
                .line_spacing(anchored_spacing(240, 32))
                .add_run(
                    Run::new()
                        .add_text(format!("Tome {volume}"))
                        .size(32)
                        .fonts(fonts(font)),
                ),
        );
    }

    if !subtitle.is_empty() {
        title_used += 240 + line_height_twips(32);
        paragraphs.push(
            Paragraph::new()
                .align(AlignmentType::Center)
                .line_spacing(anchored_spacing(240, 32))
                .add_run(
                    Run::new()
                        .add_text(subtitle)
                        .size(32)
                        .italic()
                        .fonts(fonts(font)),
                ),
        );
    }
    if !author.is_empty() {
        title_used += 720 + line_height_twips(40);
        paragraphs.push(
            Paragraph::new()
                .align(AlignmentType::Center)
                .line_spacing(anchored_spacing(720, 40))
                .add_run(
                    Run::new()
                        .add_text(author.clone())
                        .size(40)
                        .fonts(fonts(font)),
                ),
        );
    }
    // Mention d'édition ancrée **en bas** de la page de titre : un espaceur de
    // hauteur **exacte** complète la zone déjà utilisée (ancrage fiable).
    let edition_before =
        content_h.saturating_sub(BOTTOM_ANCHOR_SAFETY + title_used + line_height_twips(24));
    paragraphs.push(spacer(edition_before, false));
    paragraphs.push(
        Paragraph::new()
            .align(AlignmentType::Center)
            .line_spacing(anchored_spacing(0, 24))
            .add_run(
                Run::new()
                    .add_text(format!("{publisher} · {year}"))
                    .size(24)
                    .fonts(fonts(font)),
            ),
    );

    paragraphs
}
