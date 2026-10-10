//! **Moteur géométrique KDP** du gabarit de couverture complète.
//!
//! Calcule le « wrap » de couverture broché : `plat 1 | tranche | plat 4`, avec
//! fond perdu sur les 4 bords et la réserve code-barres.
//!
//! Règles (source unique de vérité) :
//! - **Parité** : le nombre de pages retenu pour la tranche est **pair** (arrondi
//!   supérieur si impair : `pages + (pages % 2)`).
//! - **Coefficients papier** (mm/page) : blanc `0,05720` · crème `0,06350` ·
//!   couleur `0,05960`.
//! - **Texte sur tranche** : éligible uniquement si `pages >= 80`.
//! - **Dimensions** : `total_width = 3,2 + trim_w + spine + trim_w + 3,2` et
//!   `total_height = 3,2 + trim_h + 3,2`.
//! - **Réserve code-barres** (50,8 × 30,5 mm) : coin inférieur droit du plat 4.

use crate::kdp;

/// Fond perdu KDP ajouté sur chacun des 4 bords (mm) ≈ 0,125 po.
pub const BLEED_MM: f64 = 3.2;

/// Marge de sécurité interne autour de la réserve code-barres (mm).
pub const BARCODE_SAFETY_MARGIN_MM: f64 = 6.4;

/// Largeur de la réserve code-barres KDP (mm).
pub const BARCODE_WIDTH_MM: f64 = 50.8;

/// Hauteur de la réserve code-barres KDP (mm).
pub const BARCODE_HEIGHT_MM: f64 = 30.5;

/// Seuil de pagination autorisant un texte sur la tranche.
pub const SPINE_TEXT_MIN_PAGES: u32 = 80;

/// Type de papier — détermine l'épaisseur du bloc (mm/page).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperType {
    /// Papier blanc.
    White,
    /// Papier crème.
    Cream,
    /// Papier couleur.
    Color,
}

impl PaperType {
    /// Épaisseur du bloc papier par page (mm).
    pub fn coefficient_mm_per_page(self) -> f64 {
        match self {
            PaperType::White => 0.05720,
            PaperType::Cream => 0.06350,
            PaperType::Color => 0.05960,
        }
    }

    /// Identifiant stable consommé par le frontend.
    pub fn id(self) -> &'static str {
        match self {
            PaperType::White => "white",
            PaperType::Cream => "cream",
            PaperType::Color => "color",
        }
    }

    /// Résout un identifiant papier (anglais ou français), insensible à la casse
    /// et aux accents. `Err` si l'identifiant est inconnu.
    pub fn from_id(id: &str) -> Result<Self, String> {
        let normalized = id.trim().to_lowercase().replace(['è', 'é'], "e");
        match normalized.as_str() {
            "white" | "blanc" | "bw" => Ok(PaperType::White),
            "cream" | "creme" => Ok(PaperType::Cream),
            "color" | "colour" | "couleur" => Ok(PaperType::Color),
            other => Err(format!(
                "Type de papier inconnu « {other} » (attendu : white/blanc, cream/crème, color/couleur)."
            )),
        }
    }
}

/// Bloc de tranche calculé.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpineInfo {
    /// Nombre de pages retenu (pair, arrondi supérieur si impair).
    pub pages_used: u32,
    /// Épaisseur calculée de la tranche (mm).
    pub width_mm: f64,
    /// Coefficient papier appliqué (mm/page).
    pub coefficient_mm_per_page: f64,
    /// `true` si un texte sur la tranche est autorisé (`pages >= 80`).
    pub text_eligible: bool,
}

/// Réserve code-barres (coin inférieur droit du plat 4).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BarcodeBox {
    /// Abscisse du coin supérieur gauche (mm, depuis le bord gauche).
    pub x_mm: f64,
    /// Ordonnée du coin supérieur gauche (mm, depuis le bord supérieur).
    pub y_mm: f64,
    /// Largeur (mm).
    pub width_mm: f64,
    /// Hauteur (mm).
    pub height_mm: f64,
}

/// Géométrie complète du gabarit de couverture (`plat 1 | tranche | plat 4`).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverGeometry {
    /// Identifiant du format de coupe résolu.
    pub trim_size: String,
    /// Identifiant du type de papier résolu.
    pub paper_type: String,
    /// Fond perdu appliqué sur chaque bord (mm).
    pub bleed_mm: f64,
    /// Largeur de coupe (mm).
    pub trim_width_mm: f64,
    /// Hauteur de coupe (mm).
    pub trim_height_mm: f64,
    /// Bloc de tranche.
    pub spine: SpineInfo,
    /// Largeur totale du gabarit (mm).
    pub total_width_mm: f64,
    /// Hauteur totale du gabarit (mm).
    pub total_height_mm: f64,
    /// Réserve code-barres.
    pub barcode: BarcodeBox,
}

/// Arrondit une valeur millimétrique à 3 décimales (sortie stable).
fn round_mm(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

/// Calcule la géométrie complète de la couverture.
///
/// - `page_count` : nombre de pages intérieures du bloc.
/// - `paper_type` : identifiant papier (`white`/`cream`/`color`, ou FR).
/// - `trim_size` : identifiant de format de coupe (`kdp::trim_size` applique le repli 6×9).
pub fn compute(page_count: u32, paper_type: &str, trim_size: &str) -> Result<CoverGeometry, String> {
    let paper = PaperType::from_id(paper_type)?;
    let trim = kdp::trim_size(trim_size);

    let trim_width_mm = trim.width_mm();
    let trim_height_mm = trim.height_mm();

    // Parité : arrondi supérieur à l'entier pair (pages + (pages % 2)).
    let pages_used = page_count + (page_count % 2);
    let coefficient = paper.coefficient_mm_per_page();
    let spine_width_mm = pages_used as f64 * coefficient;

    let total_width_mm = BLEED_MM + trim_width_mm + spine_width_mm + trim_width_mm + BLEED_MM;
    let total_height_mm = BLEED_MM + trim_height_mm + BLEED_MM;

    // Coin inférieur droit du plat 4, avec marge de sécurité interne.
    let barcode_x = BLEED_MM + trim_width_mm - BARCODE_SAFETY_MARGIN_MM - BARCODE_WIDTH_MM;
    let barcode_y = total_height_mm - BLEED_MM - BARCODE_SAFETY_MARGIN_MM - BARCODE_HEIGHT_MM;

    Ok(CoverGeometry {
        trim_size: trim.id.to_string(),
        paper_type: paper.id().to_string(),
        bleed_mm: BLEED_MM,
        trim_width_mm: round_mm(trim_width_mm),
        trim_height_mm: round_mm(trim_height_mm),
        spine: SpineInfo {
            pages_used,
            width_mm: round_mm(spine_width_mm),
            coefficient_mm_per_page: coefficient,
            text_eligible: page_count >= SPINE_TEXT_MIN_PAGES,
        },
        total_width_mm: round_mm(total_width_mm),
        total_height_mm: round_mm(total_height_mm),
        barcode: BarcodeBox {
            x_mm: round_mm(barcode_x),
            y_mm: round_mm(barcode_y),
            width_mm: BARCODE_WIDTH_MM,
            height_mm: BARCODE_HEIGHT_MM,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Comparaison millimétrique tolérante (précision d'arrondi 3 décimales).
    fn approx(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-3,
            "attendu {expected}, obtenu {actual}"
        );
    }

    #[test]
    fn parity_rounds_up_to_even() {
        assert_eq!(compute(120, "cream", "6x9").unwrap().spine.pages_used, 120);
        assert_eq!(compute(121, "cream", "6x9").unwrap().spine.pages_used, 122);
        assert_eq!(compute(0, "cream", "6x9").unwrap().spine.pages_used, 0);
    }

    #[test]
    fn cream_120_and_300_pages_five_by_eight_and_half() {
        // 5,5 × 8,5 po → coupe 139,7 × 215,9 mm.
        let g120 = compute(120, "crème", "5.5x8.5").unwrap();
        approx(g120.spine.width_mm, 7.62);
        approx(g120.total_width_mm, 293.42);
        approx(g120.total_height_mm, 222.3);

        let g300 = compute(300, "cream", "5.5x8.5").unwrap();
        approx(g300.spine.width_mm, 19.05);
        approx(g300.total_width_mm, 304.85);
        approx(g300.total_height_mm, 222.3);
    }

    #[test]
    fn cream_120_and_300_pages_six_by_nine() {
        // 6 × 9 po → coupe 152,4 × 228,6 mm.
        let g120 = compute(120, "cream", "6x9").unwrap();
        approx(g120.spine.width_mm, 7.62);
        approx(g120.total_width_mm, 318.82);
        approx(g120.total_height_mm, 235.0);
        // Réserve code-barres : coin inférieur droit du plat 4.
        approx(g120.barcode.x_mm, 98.4);
        approx(g120.barcode.y_mm, 194.9);
        assert_eq!(g120.barcode.width_mm, 50.8);
        assert_eq!(g120.barcode.height_mm, 30.5);

        let g300 = compute(300, "cream", "6x9").unwrap();
        approx(g300.spine.width_mm, 19.05);
        approx(g300.total_width_mm, 330.25);
        approx(g300.total_height_mm, 235.0);
    }

    #[test]
    fn spine_text_requires_eighty_pages() {
        assert!(!compute(78, "white", "6x9").unwrap().spine.text_eligible);
        assert!(compute(80, "white", "6x9").unwrap().spine.text_eligible);
        assert!(compute(300, "white", "6x9").unwrap().spine.text_eligible);
    }

    #[test]
    fn unknown_paper_type_is_rejected() {
        assert!(compute(120, "papyrus", "6x9").is_err());
    }
}

