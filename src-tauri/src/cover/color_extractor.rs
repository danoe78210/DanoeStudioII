//! **Extraction de couleur de tranche** : analyse les bords internes (côté
//! reliure) des images Recto (plat 1) et Verso (plat 4) et suggère une couleur
//! de tranche (unie ou dégradé).

use image::ImageReader;

/// Nombre de colonnes de pixels analysées sur chaque bord interne.
pub const EDGE_COLUMNS: u32 = 10;

/// Écart de couleur (distance euclidienne sRGB) au-delà duquel un dégradé est
/// recommandé pour la tranche.
pub const GRADIENT_THRESHOLD: f64 = 40.0;

/// Couleur suggérée pour la tranche.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpineColorSuggestion {
    /// Couleur dominante du bord interne du Recto (plat 1), `#rrggbb`.
    pub color_front: String,
    /// Couleur dominante du bord interne du Verso (plat 4), `#rrggbb`.
    pub color_back: String,
    /// `true` si l'écart dépasse 40 points (dégradé conseillé).
    pub gradient_recommended: bool,
}

/// Convertit un triplet RVB en chaîne hexadécimale `#rrggbb`.
pub fn to_hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

/// Distance euclidienne entre deux couleurs sRGB.
pub fn distance(a: [u8; 3], b: [u8; 3]) -> f64 {
    let dr = a[0] as f64 - b[0] as f64;
    let dg = a[1] as f64 - b[1] as f64;
    let db = a[2] as f64 - b[2] as f64;
    (dr * dr + dg * dg + db * db).sqrt()
}

/// Médiane d'une liste de valeurs (0 si vide).
fn median(values: &mut [u8]) -> u8 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    values[values.len() / 2]
}

/// Couleur dominante (médiane RVB) des `EDGE_COLUMNS` colonnes du bord interne.
///
/// `from_right = true` échantillonne le bord **droit** (Verso, plat 4) ;
/// `false` échantillonne le bord **gauche** (Recto, plat 1).
pub fn edge_color(path: &str, from_right: bool) -> Result<[u8; 3], String> {
    let image = ImageReader::open(path)
        .map_err(|e| format!("Image introuvable ou illisible « {path} » : {e}"))?
        .with_guessed_format()
        .map_err(|e| format!("Format d'image non reconnu « {path} » : {e}"))?
        .decode()
        .map_err(|e| format!("Décodage de l'image impossible « {path} » : {e}"))?
        .to_rgb8();

    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return Err(format!("Dimensions d'image invalides « {path} »."));
    }

    let columns = EDGE_COLUMNS.min(width);
    let mut reds = Vec::with_capacity((columns * height) as usize);
    let mut greens = Vec::with_capacity((columns * height) as usize);
    let mut blues = Vec::with_capacity((columns * height) as usize);

    for y in 0..height {
        for offset in 0..columns {
            let x = if from_right { width - 1 - offset } else { offset };
            let px = image.get_pixel(x, y).0;
            reds.push(px[0]);
            greens.push(px[1]);
            blues.push(px[2]);
        }
    }

    Ok([median(&mut reds), median(&mut greens), median(&mut blues)])
}

/// Suggère une couleur de tranche à partir des images Recto / Verso.
pub fn suggest(front_path: &str, back_path: &str) -> Result<SpineColorSuggestion, String> {
    let front = edge_color(front_path, false)?;
    let back = edge_color(back_path, true)?;
    Ok(SpineColorSuggestion {
        color_front: to_hex(front),
        color_back: to_hex(back),
        gradient_recommended: distance(front, back) > GRADIENT_THRESHOLD,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn write_solid(dir: &std::path::Path, name: &str, rgb: [u8; 3]) -> String {
        let path = dir.join(name);
        RgbaImage::from_pixel(20, 20, Rgba([rgb[0], rgb[1], rgb[2], 255]))
            .save(&path)
            .unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn divergent_edges_recommend_gradient() {
        let dir = std::env::temp_dir().join(format!("danoe_spine_color_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let front = write_solid(&dir, "front.png", [200, 20, 20]);
        let back = write_solid(&dir, "back.png", [20, 20, 200]);

        let suggestion = suggest(&front, &back).expect("suggestion attendue");
        assert_eq!(suggestion.color_front, "#c81414");
        assert_eq!(suggestion.color_back, "#1414c8");
        assert!(suggestion.gradient_recommended, "écart > 40 points");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn close_edges_keep_solid_color() {
        let dir = std::env::temp_dir().join(format!("danoe_spine_close_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let front = write_solid(&dir, "front.png", [10, 60, 30]);
        let back = write_solid(&dir, "back.png", [14, 62, 33]);

        let suggestion = suggest(&front, &back).expect("suggestion attendue");
        assert!(!suggestion.gradient_recommended, "écart faible");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn distance_is_euclidean_in_srgb() {
        assert!((distance([0, 0, 0], [0, 0, 0])).abs() < 1e-9);
        assert!((distance([0, 0, 0], [40, 0, 0]) - 40.0).abs() < 1e-9);
        assert!((distance([0, 0, 0], [30, 40, 0]) - 50.0).abs() < 1e-9);
    }
}
