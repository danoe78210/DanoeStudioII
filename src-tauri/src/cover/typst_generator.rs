//! **Génération Typst & export PDF de couverture complète** (300 DPI, KDP).
//!
//! Assemble le gabarit `plat 1 | tranche | plat 4` sur 4 calques, pré-traite les
//! images (aplatissement sur blanc si canal alpha) et compile le PDF via le
//! moteur Typst embarqué ([`crate::pdf`]).

use std::path::{Path, PathBuf};

use crate::cover::geometry::{self, CoverGeometry};
use crate::pdf;

/// DPI de sortie cible (impression KDP).
pub const TARGET_DPI: u32 = 300;

/// Taille de police par défaut du texte de tranche (pt).
pub const SPINE_TEXT_SIZE_PT: f64 = 11.0;

/// Paramètres de rendu d'une couverture complète (fournis par le frontend).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverRenderParams {
    /// Chemin du plat 1 (couverture avant).
    pub front_path: String,
    /// Chemin du plat 4 (couverture arrière / dos).
    pub back_path: String,
    /// Nombre de pages intérieures.
    pub page_count: u32,
    /// Type de papier (`white`/`cream`/`color`).
    pub paper_type: String,
    /// Format de coupe (`5.5x8.5`, `6x9`, …).
    pub trim_size: String,
    /// Texte de la tranche (titre / auteur) — optionnel.
    #[serde(default)]
    pub spine_text: Option<String>,
    /// Couleur de tranche côté plat 1 (dégradé) — optionnel.
    #[serde(default)]
    pub spine_color_front: Option<String>,
    /// Couleur de tranche côté plat 4 (dégradé) — optionnel.
    #[serde(default)]
    pub spine_color_back: Option<String>,
}

/// Rapport d'export d'une couverture.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    /// Chemin du PDF écrit.
    pub output_path: String,
    /// Taille du PDF (octets).
    pub bytes: usize,
    /// DPI de sortie.
    pub dpi: u32,
    /// Nombre de pages retenu pour la tranche (pair).
    pub page_count_used: u32,
    /// Largeur totale du gabarit (mm).
    pub total_width_mm: f64,
    /// Hauteur totale du gabarit (mm).
    pub total_height_mm: f64,
    /// Épaisseur de la tranche (mm).
    pub spine_width_mm: f64,
    /// `true` si un texte de tranche a été rendu (`pages >= 80`).
    pub spine_text_eligible: bool,
    /// Images aplaties sur blanc (noms de fichiers temporaires).
    pub flattened_images: Vec<String>,
}

/// Conversion millimètres → points typographiques (`pt = mm * 72 / 25,4`).
pub fn mm_to_pt(mm: f64) -> f64 {
    mm * 72.0 / 25.4
}

/// Échappe une chaîne pour le balisage Typst (mode texte).
pub fn escape_markup(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(
            ch,
            '\\' | '#' | '[' | ']' | '$' | '*' | '_' | '`' | '@' | '<' | '>' | '~'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Expression Typst de remplissage de la tranche : dégradé si les deux couleurs
/// diffèrent, sinon couleur unie (défaut : cuir sombre).
pub fn spine_fill(params: &CoverRenderParams) -> String {
    let front = params
        .spine_color_front
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let back = params
        .spine_color_back
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    match (front, back) {
        (Some(f), Some(b)) if f != b => format!("gradient.linear(rgb(\"{f}\"), rgb(\"{b}\"))"),
        (Some(c), _) | (None, Some(c)) => format!("rgb(\"{c}\")"),
        (None, None) => "rgb(\"#1c140e\")".to_string(),
    }
}

/// Prépare une image dans le dossier de travail Typst.
///
/// Si l'image porte un canal **alpha**, elle est aplatie sur fond blanc
/// (`#ffffff`) et ré-encodée en PNG ; sinon le fichier est copié tel quel
/// (extension préservée). Renvoie le nom de fichier référencé côté Typst, et
/// pousse ce nom dans `flattened` si un aplatissement a eu lieu.
fn prepare_image(
    src: &str,
    work_dir: &Path,
    stem: &str,
    flattened: &mut Vec<String>,
) -> Result<String, String> {
    let path = Path::new(src);
    let bytes =
        std::fs::read(path).map_err(|e| format!("Lecture de l'image impossible « {src} » : {e}"))?;
    let decoded = image::load_from_memory(&bytes)
        .map_err(|e| format!("Décodage de l'image impossible « {src} » : {e}"))?;

    if decoded.color().has_alpha() {
        let rgba = decoded.to_rgba8();
        let (width, height) = rgba.dimensions();
        let mut out = image::RgbImage::new(width, height);
        for (x, y, px) in rgba.enumerate_pixels() {
            let alpha = px[3] as f32 / 255.0;
            let blend = |channel: u8| {
                (channel as f32 * alpha + 255.0 * (1.0 - alpha)).round() as u8
            };
            out.put_pixel(
                x,
                y,
                image::Rgb([blend(px[0]), blend(px[1]), blend(px[2])]),
            );
        }
        let name = format!("{stem}_flat.png");
        out.save(work_dir.join(&name))
            .map_err(|e| format!("Écriture de l'image aplatie impossible : {e}"))?;
        flattened.push(name.clone());
        Ok(name)
    } else {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
            .to_ascii_lowercase();
        let name = format!("{stem}.{extension}");
        std::fs::write(work_dir.join(&name), &bytes)
            .map_err(|e| format!("Copie de l'image impossible : {e}"))?;
        Ok(name)
    }
}

/// Crée un dossier de travail temporaire unique (`std::env::temp_dir()`).
fn temp_work_dir() -> Result<PathBuf, String> {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("danoe_cover_{}_{}", std::process::id(), unique));
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Création du dossier temporaire impossible : {e}"))?;
    Ok(dir)
}

/// Construit le balisage Typst de la couverture complète (4 calques).
pub fn build_source(
    geo: &CoverGeometry,
    front_file: &str,
    back_file: &str,
    params: &CoverRenderParams,
) -> String {
    let page_w = mm_to_pt(geo.total_width_mm);
    let page_h = mm_to_pt(geo.total_height_mm);
    let bleed = mm_to_pt(geo.bleed_mm);
    let trim_w = mm_to_pt(geo.trim_width_mm);
    let spine_w = mm_to_pt(geo.spine.width_mm);

    let back_dx = 0.0;
    let back_w = bleed + trim_w;
    let spine_dx = bleed + trim_w;
    let front_dx = bleed + trim_w + spine_w;
    let front_w = trim_w + bleed;

    let fill = spine_fill(params);
    let mut src = String::new();

    src.push_str(&format!(
        "#set page(width: {page_w:.3}pt, height: {page_h:.3}pt, margin: 0pt, fill: white)\n"
    ));

    // Calque 1 — plat 4 (couverture arrière).
    src.push_str(&format!(
        "#place(dx: {back_dx:.3}pt, dy: 0pt, image(\"{back_file}\", width: {back_w:.3}pt, height: {page_h:.3}pt, fit: \"cover\"))\n"
    ));

    // Calque 2 — plat 1 (couverture avant).
    src.push_str(&format!(
        "#place(dx: {front_dx:.3}pt, dy: 0pt, image(\"{front_file}\", width: {front_w:.3}pt, height: {page_h:.3}pt, fit: \"cover\"))\n"
    ));

    // Calque 3 — tranche vectorielle (fond uni ou dégradé).
    src.push_str(&format!(
        "#place(dx: {spine_dx:.3}pt, dy: 0pt, rect(width: {spine_w:.3}pt, height: {page_h:.3}pt, fill: {fill}))\n"
    ));

    // Texte de tranche (uniquement si éligible : pages >= 80).
    if geo.spine.text_eligible {
        if let Some(text) = params
            .spine_text
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
        {
            let text_color = params
                .spine_color_back
                .as_deref()
                .or(params.spine_color_front.as_deref())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("#ffffff");
            src.push_str(&format!(
                "#place(dx: {spine_dx:.3}pt, dy: 0pt, block(width: {spine_w:.3}pt, height: {page_h:.3}pt, align(center + horizon, rotate(-90deg, text(size: {size:.3}pt, fill: rgb(\"{text_color}\"), font: (\"Libertinus Serif\"))[ {escaped} ]))))\n",
                size = SPINE_TEXT_SIZE_PT,
                escaped = escape_markup(text),
            ));
        }
    }

    // Calque 4 — réserve blanche de code-barres (coin inférieur droit du plat 4).
    let bc_x = mm_to_pt(geo.barcode.x_mm);
    let bc_y = mm_to_pt(geo.barcode.y_mm);
    let bc_w = mm_to_pt(geo.barcode.width_mm);
    let bc_h = mm_to_pt(geo.barcode.height_mm);
    src.push_str(&format!(
        "#place(dx: {bc_x:.3}pt, dy: {bc_y:.3}pt, rect(width: {bc_w:.3}pt, height: {bc_h:.3}pt, fill: rgb(\"#ffffff\")))\n"
    ));

    src
}

/// Rend une couverture complète en PDF et l'écrit à `output_pdf_path`.
pub fn render_cover(
    params: &CoverRenderParams,
    output_pdf_path: &str,
) -> Result<ExportReport, String> {
    let geo = geometry::compute(params.page_count, &params.paper_type, &params.trim_size)?;
    let work_dir = temp_work_dir()?;

    let result = (|| -> Result<ExportReport, String> {
        let mut flattened = Vec::new();
        let front_file = prepare_image(&params.front_path, &work_dir, "plat1", &mut flattened)?;
        let back_file = prepare_image(&params.back_path, &work_dir, "plat4", &mut flattened)?;

        let source = build_source(&geo, &front_file, &back_file, params);
        let bytes = pdf::compile_to_pdf_with_root(&source, &work_dir)?;

        let output = Path::new(output_pdf_path);
        if let Some(parent) = output.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).ok();
            }
        }
        std::fs::write(output, &bytes)
            .map_err(|e| format!("Écriture du PDF impossible « {output_pdf_path} » : {e}"))?;

        let spine_text_rendered = geo.spine.text_eligible
            && params
                .spine_text
                .as_deref()
                .map(str::trim)
                .is_some_and(|t| !t.is_empty());

        Ok(ExportReport {
            output_path: output_pdf_path.to_string(),
            bytes: bytes.len(),
            dpi: TARGET_DPI,
            page_count_used: geo.spine.pages_used,
            total_width_mm: geo.total_width_mm,
            total_height_mm: geo.total_height_mm,
            spine_width_mm: geo.spine.width_mm,
            spine_text_eligible: spine_text_rendered,
            flattened_images: flattened,
        })
    })();

    let _ = std::fs::remove_dir_all(&work_dir);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    /// Écrit un PNG RGBA uni (avec canal alpha) et renvoie son chemin.
    fn write_png(dir: &Path, name: &str, color: Rgba<u8>) -> String {
        let path = dir.join(name);
        RgbaImage::from_pixel(8, 8, color).save(&path).unwrap();
        path.to_string_lossy().into_owned()
    }

    /// Écrit un PNG RVB uni (sans canal alpha) et renvoie son chemin.
    fn write_rgb_png(dir: &Path, name: &str, rgb: [u8; 3]) -> String {
        let path = dir.join(name);
        image::RgbImage::from_pixel(8, 8, image::Rgb(rgb))
            .save(&path)
            .unwrap();
        path.to_string_lossy().into_owned()
    }

    fn base_params() -> CoverRenderParams {
        CoverRenderParams {
            front_path: String::new(),
            back_path: String::new(),
            page_count: 80,
            paper_type: "white".into(),
            trim_size: "6x9".into(),
            spine_text: None,
            spine_color_front: None,
            spine_color_back: None,
        }
    }

    #[test]
    fn mm_converts_to_points() {
        assert!((mm_to_pt(25.4) - 72.0).abs() < 1e-9);
    }

    #[test]
    fn full_cover_render_produces_valid_pdf() {
        let dir = std::env::temp_dir().join(format!("danoe_cover_render_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // PNG RVB (sans canal alpha) → copié tel quel, non aplati.
        let front = write_rgb_png(&dir, "front.png", [200, 30, 30]);
        // PNG RGBA (canal alpha) → aplati sur blanc avant compilation.
        let back = write_png(&dir, "back.png", Rgba([30, 30, 200, 0]));
        let pdf_path = dir.join("cover.pdf");

        let params = CoverRenderParams {
            front_path: front,
            back_path: back,
            page_count: 120,
            paper_type: "cream".to_string(),
            trim_size: "6x9".to_string(),
            spine_text: Some("NUNAEL — Danoë".to_string()),
            spine_color_front: Some("#c81e1e".to_string()),
            spine_color_back: Some("#1e1ec8".to_string()),
        };

        let report = render_cover(&params, pdf_path.to_str().unwrap()).expect("rendu du PDF");
        let bytes = std::fs::read(&pdf_path).expect("lecture du PDF");

        assert!(bytes.starts_with(b"%PDF"), "signature PDF");
        assert!(bytes.windows(5).any(|w| w == b"%%EOF"), "marqueur %%EOF");
        assert_eq!(report.dpi, TARGET_DPI);
        assert_eq!(report.page_count_used, 120);
        assert!(report.spine_text_eligible);
        assert_eq!(report.flattened_images.len(), 1, "une image aplatie (alpha)");
        assert!(report.bytes > 1000, "PDF non trivial");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_source_places_all_four_layers() {
        let geo = geometry::compute(300, "white", "6x9").unwrap();
        let params = CoverRenderParams {
            front_path: "f.png".into(),
            back_path: "b.png".into(),
            page_count: 300,
            paper_type: "white".into(),
            trim_size: "6x9".into(),
            spine_text: Some("Titre".into()),
            spine_color_front: None,
            spine_color_back: None,
        };
        let src = build_source(&geo, "plat1.png", "plat4.png", &params);
        assert_eq!(src.matches("fit: \"cover\"").count(), 2, "2 images en cover");
        assert!(src.contains("rotate(-90deg"), "texte de tranche tourné");
        assert!(src.contains("rgb(\"#ffffff\")"), "réserve code-barres");
        assert!(src.contains("#1c140e"), "tranche cuir par défaut");
        // 4 calques + le texte de tranche (calque vectoriel associé à la tranche).
        assert_eq!(src.matches("#place(").count(), 5, "calques + texte de tranche");
    }

    #[test]
    fn spine_fill_uses_gradient_when_colors_differ() {
        let mut params = base_params();
        params.spine_color_front = Some("#111111".into());
        params.spine_color_back = Some("#222222".into());
        assert!(spine_fill(&params).starts_with("gradient.linear"));
        params.spine_color_back = Some("#111111".into());
        assert_eq!(spine_fill(&params), "rgb(\"#111111\")");
        params.spine_color_front = None;
        params.spine_color_back = None;
        assert_eq!(spine_fill(&params), "rgb(\"#1c140e\")");
    }
}