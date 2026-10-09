//! **Spécifications physiques Amazon KDP Print — source unique de vérité.**
//!
//! Ce module centralise *toutes* les règles d'impression broché KDP :
//! formats de coupe (Trim Sizes), formule de la gouttière (Gutter) selon le
//! nombre de pages, marges minimales obligatoires, marges de service appliquées
//! au corps, et règles de fond perdu (Bleed).
//!
//! Aucun autre module (moteur Word `.docx`, futur moteur PDF, EPUB) ne doit
//! redéfinir ces valeurs : ils consomment **exclusivement** cette API.
//!
//! Références : `specifications_roman_kdp.md` §4 (normes KDP broché).

// ---------------------------------------------------------------------------
// Unités de mesure
// ---------------------------------------------------------------------------

/// Twips par pouce (unité OOXML de référence : 1 po = 1440 twips).
pub const TWIPS_PER_INCH: f64 = 1440.0;
/// Millimètres par pouce (conversion exacte).
pub const MM_PER_INCH: f64 = 25.4;
/// Twips par millimètre : 1 mm = 1440 / 25,4 twips.
pub const TWIPS_PER_MM: f64 = TWIPS_PER_INCH / MM_PER_INCH;
/// English Metric Units (EMU) par twip — pour les dimensions d'image (`docx-rs`).
pub const EMU_PER_TWIP: u32 = 635;

/// Convertit des millimètres en twips (arrondi au twip le plus proche).
pub fn mm_to_twips(mm: f64) -> u32 {
    (mm * TWIPS_PER_MM).round() as u32
}

/// Convertit des pouces en twips (arrondi au twip le plus proche).
pub fn inches_to_twips(inches: f64) -> u32 {
    (inches * TWIPS_PER_INCH).round() as u32
}

/// Convertit des twips en millimètres.
pub fn twips_to_mm(twips: u32) -> f64 {
    twips as f64 / TWIPS_PER_MM
}

// ---------------------------------------------------------------------------
// Bornes de pagination KDP (broché)
// ---------------------------------------------------------------------------

/// Nombre de pages minimal accepté par KDP (broché).
pub const MIN_PAGES: u32 = 24;
/// Nombre de pages maximal accepté par KDP (broché).
pub const MAX_PAGES: u32 = 828;

/// Identifiant du format de coupe par défaut (standard roman).
pub const DEFAULT_TRIM_ID: &str = "6x9";

// ---------------------------------------------------------------------------
// Marges
// ---------------------------------------------------------------------------

/// Marge **minimale** KDP haut (mm) — garde obligatoire.
pub const MIN_MARGIN_TOP_MM: f64 = 6.4;
/// Marge **minimale** KDP bas (mm) — garde obligatoire.
pub const MIN_MARGIN_BOTTOM_MM: f64 = 6.4;
/// Marge **minimale** KDP extérieure (mm) — garde obligatoire.
pub const MIN_MARGIN_OUTER_MM: f64 = 9.6;

/// Marge **latérale** de service (twips) appliquée aux sections du corps (19,05 mm).
pub const SIDE_MARGIN_TWIPS: i32 = 1080;
/// Marge **supérieure** de service (twips) appliquée aux sections (25,4 mm).
pub const TOP_MARGIN_TWIPS: i32 = 1440;
/// Marge **inférieure** de service (twips) appliquée aux sections (19,05 mm).
pub const BOTTOM_MARGIN_TWIPS: i32 = 1080;
/// Distance du bord au **folio / en-tête courant** (twips).
pub const HEADER_MARGIN_TWIPS: i32 = 720;
/// Distance du bord au **pied de page** (twips).
pub const FOOTER_MARGIN_TWIPS: i32 = 720;

// ---------------------------------------------------------------------------
// Fond perdu (Bleed)
// ---------------------------------------------------------------------------

/// Fond perdu ajouté au **bord extérieur** (pouces) ≈ 3,2 mm.
pub const BLEED_OUTER_IN: f64 = 0.125;
/// Fond perdu ajouté à la **hauteur totale** (haut + bas, en pouces) ≈ 6,4 mm.
pub const BLEED_VERTICAL_IN: f64 = 0.25;

// ---------------------------------------------------------------------------
// Formats de coupe (Trim Sizes)
// ---------------------------------------------------------------------------

/// Format de coupe KDP : identifiant + dimensions physiques en pouces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrimSize {
    /// Identifiant stable consommé par le frontend (ex. « 6x9 »).
    pub id: &'static str,
    /// Largeur de coupe (pouces).
    pub width_in: f64,
    /// Hauteur de coupe (pouces).
    pub height_in: f64,
}

impl TrimSize {
    /// Largeur de coupe en millimètres.
    pub fn width_mm(&self) -> f64 {
        self.width_in * MM_PER_INCH
    }

    /// Hauteur de coupe en millimètres.
    pub fn height_mm(&self) -> f64 {
        self.height_in * MM_PER_INCH
    }

    /// Dimensions de coupe **(largeur, hauteur) en twips**.
    pub fn twips(&self) -> (u32, u32) {
        (
            inches_to_twips(self.width_in),
            inches_to_twips(self.height_in),
        )
    }

    /// Dimensions **avec fond perdu** (largeur, hauteur) en pouces.
    ///
    /// Le fond perdu KDP déborde au bord extérieur (+0,125 po) et en haut/bas
    /// (+0,25 po cumulés sur la hauteur), soit 6,125 × 9,25 po pour un 6×9.
    pub fn bleed_inches(&self) -> (f64, f64) {
        (
            self.width_in + BLEED_OUTER_IN,
            self.height_in + BLEED_VERTICAL_IN,
        )
    }

    /// Dimensions **avec fond perdu** (largeur, hauteur) en twips.
    pub fn bleed_twips(&self) -> (u32, u32) {
        let (w, h) = self.bleed_inches();
        (inches_to_twips(w), inches_to_twips(h))
    }
}

/// Tous les formats de coupe KDP broché supportés (les 7 presets du frontend).
pub const TRIM_SIZES: &[TrimSize] = &[
    TrimSize {
        id: "5x8",
        width_in: 5.0,
        height_in: 8.0,
    },
    TrimSize {
        id: "5.25x8",
        width_in: 5.25,
        height_in: 8.0,
    },
    TrimSize {
        id: "5.5x8.5",
        width_in: 5.5,
        height_in: 8.5,
    },
    TrimSize {
        id: "6x9",
        width_in: 6.0,
        height_in: 9.0,
    },
    TrimSize {
        id: "7x10",
        width_in: 7.0,
        height_in: 10.0,
    },
    TrimSize {
        id: "8x10",
        width_in: 8.0,
        height_in: 10.0,
    },
    // A4 (210 × 297 mm) exprimé en pouces pour une conversion unique et exacte.
    TrimSize {
        id: "a4",
        width_in: 210.0 / MM_PER_INCH,
        height_in: 297.0 / MM_PER_INCH,
    },
];

/// Résout un identifiant de format de coupe.
///
/// **Repli sûr** : un identifiant inconnu (ou vide) retombe sur le format
/// standard roman `6x9`.
pub fn trim_size(id: &str) -> TrimSize {
    let id = id.trim();
    TRIM_SIZES
        .iter()
        .copied()
        .find(|t| t.id == id)
        .unwrap_or_else(|| {
            *TRIM_SIZES
                .iter()
                .find(|t| t.id == DEFAULT_TRIM_ID)
                .expect("format par défaut présent")
        })
}

/// Convertit un identifiant de format KDP en dimensions (largeur, hauteur) **twips**.
pub fn trim_size_twips(id: &str) -> (u32, u32) {
    trim_size(id).twips()
}

// ---------------------------------------------------------------------------
// Gouttière (Gutter) — marge de reliure dynamique
// ---------------------------------------------------------------------------

/// Table des seuils de gouttière KDP (borne haute de pages → gouttière en mm).
///
/// Normes KDP broché (spec §4) : la marge intérieure croît avec l'épaisseur.
const GUTTER_TABLE: &[(u32, f64)] = &[
    (150, 9.6),
    (300, 12.7),
    (500, 15.9),
    (700, 19.1),
    (MAX_PAGES, 22.3),
];

/// Gouttière KDP (en millimètres) selon le nombre de pages estimé.
pub fn gutter_mm(page_count: u32) -> f64 {
    for (max_pages, mm) in GUTTER_TABLE {
        if page_count <= *max_pages {
            return *mm;
        }
    }
    GUTTER_TABLE.last().map(|(_, mm)| *mm).unwrap_or(22.3)
}

/// Gouttière KDP (en twips) selon le nombre de pages estimé (broché).
pub fn gutter_twips(page_count: u32) -> u32 {
    mm_to_twips(gutter_mm(page_count))
}

// ---------------------------------------------------------------------------
// Spécification de page complète (consommée par le moteur PDF / Word)
// ---------------------------------------------------------------------------

/// Spécification physique complète d'une page intérieure KDP.
///
/// Agrège format de coupe, fond perdu, marges de service et gouttière en une
/// structure unique : les moteurs (`.docx`, futur `pdf.rs`) n'ont plus à
/// recomposer ces règles eux-mêmes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KdpPageSpec {
    /// Format de coupe retenu.
    pub trim: TrimSize,
    /// Nombre de pages estimé (épaisseur → gouttière).
    pub page_count: u32,
    /// Fond perdu activé (illustrations à fond perdu).
    pub bleed: bool,
    /// Largeur de page finale en twips (avec fond perdu le cas échéant).
    pub page_width_twips: u32,
    /// Hauteur de page finale en twips (avec fond perdu le cas échéant).
    pub page_height_twips: u32,
    /// Marge haute de service (twips).
    pub margin_top_twips: u32,
    /// Marge basse de service (twips).
    pub margin_bottom_twips: u32,
    /// Marge latérale de service (une seule valeur, appliquée à gauche/droite).
    pub margin_side_twips: u32,
    /// Gouttière de reliure (twips), ajoutée à la marge intérieure.
    pub gutter_twips: u32,
}

impl KdpPageSpec {
    /// Résout la spécification complète d'une page (format + pagination [+ bleed]).
    pub fn resolve(trim_id: &str, page_count: u32, bleed: bool) -> Self {
        let trim = trim_size(trim_id);
        let (page_width_twips, page_height_twips) = if bleed {
            trim.bleed_twips()
        } else {
            trim.twips()
        };
        Self {
            trim,
            page_count,
            bleed,
            page_width_twips,
            page_height_twips,
            margin_top_twips: TOP_MARGIN_TWIPS as u32,
            margin_bottom_twips: BOTTOM_MARGIN_TWIPS as u32,
            margin_side_twips: SIDE_MARGIN_TWIPS as u32,
            gutter_twips: gutter_twips(page_count),
        }
    }

    /// Gouttière en millimètres (commodité pour la validation métier).
    pub fn gutter_mm(&self) -> f64 {
        gutter_mm(self.page_count)
    }

    /// Largeur **utile** d'impression (twips) : page − marges latérales − gouttière.
    pub fn content_width_twips(&self) -> u32 {
        self.page_width_twips
            .saturating_sub(self.margin_side_twips * 2 + self.gutter_twips)
    }

    /// Hauteur **utile** d'impression (twips) : page − marges haute et basse.
    pub fn content_height_twips(&self) -> u32 {
        self.page_height_twips
            .saturating_sub(self.margin_top_twips + self.margin_bottom_twips)
    }
}

// ---------------------------------------------------------------------------
// Tests métier purs (indépendants de tout format de fichier généré)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_conversions_are_exact() {
        assert_eq!(mm_to_twips(25.4), 1440);
        assert_eq!(inches_to_twips(1.0), 1440);
        assert_eq!(inches_to_twips(6.0), 8640);
        assert!((twips_to_mm(1440) - 25.4).abs() < 1e-9);
        // 1 twip = 635 EMU (référence docx-rs).
        assert_eq!(EMU_PER_TWIP, 635);
    }

    #[test]
    fn trim_sizes_match_kdp_dimensions() {
        // Formats en pouces : 1 po = 1440 twips.
        assert_eq!(trim_size_twips("6x9"), (8640, 12960));
        assert_eq!(trim_size_twips("5x8"), (7200, 11520));
        assert_eq!(trim_size_twips("5.25x8"), (7560, 11520));
        assert_eq!(trim_size_twips("5.5x8.5"), (7920, 12240));
        assert_eq!(trim_size_twips("7x10"), (10080, 14400));
        assert_eq!(trim_size_twips("8x10"), (11520, 14400));
        // A4 : conversion millimètres → twips (1440 / 25,4).
        assert_eq!(trim_size_twips("a4"), (11906, 16838));
        // Format inconnu (ou vide) → repli 6×9 (standard roman).
        assert_eq!(trim_size_twips("inconnu"), (8640, 12960));
        assert_eq!(trim_size_twips(""), (8640, 12960));
        assert_eq!(trim_size_twips("  6x9  "), (8640, 12960));
    }

    #[test]
    fn trim_size_exposes_metric_dimensions() {
        let t = trim_size("6x9");
        assert!((t.width_mm() - 152.4).abs() < 1e-6);
        assert!((t.height_mm() - 228.6).abs() < 1e-6);
    }

    #[test]
    fn every_trim_has_distinct_id_and_positive_size() {
        for trim in TRIM_SIZES {
            assert!(trim.width_in > 0.0 && trim.height_in > 0.0, "{}", trim.id);
            assert_eq!(
                TRIM_SIZES.iter().filter(|t| t.id == trim.id).count(),
                1,
                "identifiant dupliqué : {}",
                trim.id
            );
        }
        // Le format par défaut doit exister dans la table.
        assert!(TRIM_SIZES.iter().any(|t| t.id == DEFAULT_TRIM_ID));
    }

    #[test]
    fn gutter_follows_kdp_page_thresholds() {
        // Normes KDP broché : mm → twips, seuils par tranche de pages.
        assert_eq!(gutter_mm(0), 9.6);
        assert_eq!(gutter_mm(150), 9.6); // borne haute de la 1re tranche
        assert_eq!(gutter_mm(151), 12.7);
        assert_eq!(gutter_mm(300), 12.7);
        assert_eq!(gutter_mm(301), 15.9);
        assert_eq!(gutter_mm(500), 15.9);
        assert_eq!(gutter_mm(501), 19.1);
        assert_eq!(gutter_mm(700), 19.1);
        assert_eq!(gutter_mm(701), 22.3);
        assert_eq!(gutter_mm(MAX_PAGES), 22.3); // borne haute KDP

        assert_eq!(gutter_twips(0), 544); // 9,6 mm
        assert_eq!(gutter_twips(150), 544);
        assert_eq!(gutter_twips(151), 720); // 12,7 mm
        assert_eq!(gutter_twips(300), 720);
        assert_eq!(gutter_twips(301), 901); // 15,9 mm
        assert_eq!(gutter_twips(500), 901);
        assert_eq!(gutter_twips(501), 1083); // 19,1 mm
        assert_eq!(gutter_twips(700), 1083);
        assert_eq!(gutter_twips(701), 1264); // 22,3 mm
        assert_eq!(gutter_twips(828), 1264);
    }

    #[test]
    fn gutter_is_monotonic_with_thickness() {
        let small = gutter_twips(100);
        let large = gutter_twips(600);
        assert!(large > small, "gouttière croissante avec l'épaisseur");
        // Aucune régression sur toute la plage de pagination KDP.
        for pages in MIN_PAGES..MAX_PAGES {
            assert!(gutter_twips(pages + 1) >= gutter_twips(pages));
        }
    }

    #[test]
    fn six_by_nine_350_pages_returns_expected_gutter_and_margins() {
        // Cas métier de référence : 6×9 po, 350 pages.
        // → tranche 301–500 pages → gouttière 15,9 mm = 901 twips.
        let spec = KdpPageSpec::resolve("6x9", 350, false);
        assert_eq!(spec.gutter_twips, 901);
        assert_eq!(spec.gutter_mm(), 15.9);
        assert_eq!(spec.page_width_twips, 8640);
        assert_eq!(spec.page_height_twips, 12960);
        // Marges de service : haut 25,4 mm, bas 19,05 mm, latérales 19,05 mm.
        assert_eq!(spec.margin_top_twips, 1440);
        assert_eq!(spec.margin_bottom_twips, 1080);
        assert_eq!(spec.margin_side_twips, 1080);
        // Largeur utile = 8640 − 2×1080 − 901 = 5579 ; hauteur utile = 12960 − 2520.
        assert_eq!(spec.content_width_twips(), 5579);
        assert_eq!(spec.content_height_twips(), 10440);
    }

    #[test]
    fn service_margins_respect_kdp_minimums() {
        assert!(twips_to_mm(TOP_MARGIN_TWIPS as u32) >= MIN_MARGIN_TOP_MM);
        assert!(twips_to_mm(BOTTOM_MARGIN_TWIPS as u32) >= MIN_MARGIN_BOTTOM_MM);
        assert!(twips_to_mm(SIDE_MARGIN_TWIPS as u32) >= MIN_MARGIN_OUTER_MM);
    }

    #[test]
    fn bleed_enlarges_page_on_outer_edge_and_height() {
        // 6×9 + fond perdu → 6,125 × 9,25 po (exemple de la spec).
        let t = trim_size("6x9");
        let (w, h) = t.bleed_inches();
        assert!((w - 6.125).abs() < 1e-9);
        assert!((h - 9.25).abs() < 1e-9);
        assert_eq!(trim_size("6x9").bleed_twips(), (8820, 13320));

        let spec = KdpPageSpec::resolve("6x9", 350, true);
        assert!(spec.bleed);
        assert_eq!(spec.page_width_twips, 8820);
        assert_eq!(spec.page_height_twips, 13320);
        // La gouttière ne dépend pas du fond perdu.
        assert_eq!(spec.gutter_twips, 901);
    }

    #[test]
    fn resolved_spec_never_underflows_content_area() {
        // Même sur le plus petit format et la plus forte gouttière, la zone
        // d'impression reste strictement positive (pas de `saturating_sub` à 0).
        let spec = KdpPageSpec::resolve("5x8", MAX_PAGES, false);
        assert!(spec.content_width_twips() > 0);
        assert!(spec.content_height_twips() > 0);
    }
}
