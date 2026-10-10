//! **Inspecteur d'images de couverture** : dimensions, canal alpha et DPI effectif
//! par rapport au format de coupe cible.
//!
//! L'inspection lit uniquement l'**en-tête** du fichier (via un `ImageDecoder`
//! obtenu sans décoder les pixels) : aucune image n'est chargée intégralement en
//! mémoire.

use crate::kdp;
use image::{ColorType, ImageDecoder, ImageReader};

/// Résolution recommandée par KDP (DPI).
pub const REQUIRED_DPI: u32 = 300;

/// Seuil bas de la zone d'avertissement (DPI).
pub const WARNING_DPI: u32 = 250;

/// Statut de conformité d'une image (selon le DPI effectif).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InspectionStatus {
    /// `dpi >= 300`.
    Valid,
    /// `250 <= dpi < 300`.
    Warning,
    /// `dpi < 250`.
    Invalid,
}

impl InspectionStatus {
    /// Classe un DPI effectif.
    pub fn from_dpi(dpi: f64) -> Self {
        if dpi >= REQUIRED_DPI as f64 {
            InspectionStatus::Valid
        } else if dpi >= WARNING_DPI as f64 {
            InspectionStatus::Warning
        } else {
            InspectionStatus::Invalid
        }
    }
}

/// Résultat d'inspection d'une image de couverture.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInspection {
    /// Chemin inspecté.
    pub path: String,
    /// Largeur en pixels.
    pub width_px: u32,
    /// Hauteur en pixels.
    pub height_px: u32,
    /// Présence d'un canal alpha (`Rgba8` / `Rgba16` / `La*`).
    pub has_alpha: bool,
    /// Type de couleur détecté (ex. `Rgba8`, `Rgb8`).
    pub color_type: String,
    /// DPI horizontal effectif (`width_px / largeur de coupe`).
    pub dpi_horizontal: f64,
    /// DPI vertical effectif (`height_px / hauteur de coupe`).
    pub dpi_vertical: f64,
    /// DPI effectif retenu (min des deux axes).
    pub dpi: f64,
    /// Statut de conformité.
    pub status: InspectionStatus,
}

/// Rapport d'inspection des deux plats de la couverture.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInspectionReport {
    /// Format de coupe de référence.
    pub trim_size: String,
    /// DPI minimal recommandé.
    pub required_dpi: u32,
    /// Plat 1 (couverture avant).
    pub front: ImageInspection,
    /// Plat 4 (couverture arrière / dos).
    pub back: ImageInspection,
}

/// Arrondit une valeur de DPI à 2 décimales (sortie stable).
fn round_dpi(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// Inspecte une image par rapport à des dimensions de coupe en pouces.
pub fn inspect(
    path: &str,
    trim_width_in: f64,
    trim_height_in: f64,
) -> Result<ImageInspection, String> {
    if path.trim().is_empty() {
        return Err("Chemin d'image vide.".to_string());
    }

    let reader = ImageReader::open(path)
        .map_err(|e| format!("Image introuvable ou illisible « {path} » : {e}"))?
        .with_guessed_format()
        .map_err(|e| format!("Format d'image non reconnu « {path} » : {e}"))?;

    // `into_decoder` lit uniquement l'en-tête : aucun décodage de pixels.
    let decoder = reader
        .into_decoder()
        .map_err(|e| format!("En-tête d'image illisible « {path} » : {e}"))?;

    let (width_px, height_px) = decoder.dimensions();
    let color_type: ColorType = decoder.color_type();

    if width_px == 0 || height_px == 0 {
        return Err(format!("Dimensions d'image invalides « {path} »."));
    }
    if trim_width_in <= 0.0 || trim_height_in <= 0.0 {
        return Err("Dimensions de coupe invalides.".to_string());
    }

    let dpi_horizontal = width_px as f64 / trim_width_in;
    let dpi_vertical = height_px as f64 / trim_height_in;
    let dpi = dpi_horizontal.min(dpi_vertical);

    Ok(ImageInspection {
        path: path.to_string(),
        width_px,
        height_px,
        has_alpha: color_type.has_alpha(),
        color_type: format!("{color_type:?}"),
        dpi_horizontal: round_dpi(dpi_horizontal),
        dpi_vertical: round_dpi(dpi_vertical),
        dpi: round_dpi(dpi),
        status: InspectionStatus::from_dpi(dpi),
    })
}

/// Inspecte les deux images (plat 1 et plat 4) par rapport au format de coupe cible.
pub fn inspect_pair(
    front_path: &str,
    back_path: &str,
    trim_size: &str,
) -> Result<ImageInspectionReport, String> {
    let trim = kdp::trim_size(trim_size);
    Ok(ImageInspectionReport {
        trim_size: trim.id.to_string(),
        required_dpi: REQUIRED_DPI,
        front: inspect(front_path, trim.width_in, trim.height_in)?,
        back: inspect(back_path, trim.width_in, trim.height_in)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn dpi_status_thresholds() {
        assert_eq!(InspectionStatus::from_dpi(301.0), InspectionStatus::Valid);
        assert_eq!(InspectionStatus::from_dpi(300.0), InspectionStatus::Valid);
        assert_eq!(InspectionStatus::from_dpi(299.9), InspectionStatus::Warning);
        assert_eq!(InspectionStatus::from_dpi(250.0), InspectionStatus::Warning);
        assert_eq!(InspectionStatus::from_dpi(249.9), InspectionStatus::Invalid);
    }

    #[test]
    fn inspects_png_dimensions_and_alpha_without_full_decode() {
        let path = std::env::temp_dir().join(format!(
            "danoe_cover_inspector_{}.png",
            std::process::id()
        ));

        // 1800 × 2700 px = 300 DPI pour un 6 × 9 po (1800/6 = 2700/9 = 300).
        let mut buffer = RgbaImage::new(1800, 2700);
        buffer.put_pixel(0, 0, Rgba([12, 34, 56, 128]));
        buffer.save(&path).expect("écriture du PNG de test impossible");

        let report = inspect_pair(path.to_str().unwrap(), path.to_str().unwrap(), "6x9")
            .expect("inspection attendue réussie");

        assert_eq!(report.front.width_px, 1800);
        assert_eq!(report.front.height_px, 2700);
        assert!(report.front.has_alpha);
        assert_eq!(report.front.color_type, "Rgba8");
        assert!((report.front.dpi - 300.0).abs() < 1e-6);
        assert_eq!(report.front.status, InspectionStatus::Valid);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_file_is_rejected() {
        assert!(inspect("chemin/inexistant.png", 6.0, 9.0).is_err());
    }
}


