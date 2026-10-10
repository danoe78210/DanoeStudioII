//! Module **Couverture** (Cover Studio) : moteur géométrique KDP et inspecteur
//! d'images de couverture.
//!
//! Expose les structures et les commandes Tauri consommées par le frontend.

pub mod color_extractor;
pub mod geometry;
pub mod inspector;
pub mod typst_generator;

pub use color_extractor::SpineColorSuggestion;
pub use geometry::{BarcodeBox, CoverGeometry, PaperType, SpineInfo};
pub use inspector::{ImageInspection, ImageInspectionReport, InspectionStatus};
pub use typst_generator::{CoverRenderParams, ExportReport};

/// Calcule la géométrie complète d'une couverture (`plat 1 | tranche | plat 4`).
#[tauri::command]
pub fn calculate_cover_geometry(
    page_count: u32,
    paper_type: String,
    trim_size: String,
) -> Result<CoverGeometry, String> {
    geometry::compute(page_count, &paper_type, &trim_size)
}

/// Inspecte les images de couverture (plat 1 `front_path`, plat 4 `back_path`) :
/// dimensions, canal alpha et DPI effectif par rapport au format de coupe.
#[tauri::command]
pub fn inspect_cover_images(
    front_path: String,
    back_path: String,
    trim_size: String,
) -> Result<ImageInspectionReport, String> {
    inspector::inspect_pair(&front_path, &back_path, &trim_size)
}

/// Extrait la couleur de tranche suggérée depuis les images Recto (plat 1) et
/// Verso (plat 4).
#[tauri::command]
pub fn extract_spine_color(
    front_path: String,
    back_path: String,
) -> Result<SpineColorSuggestion, String> {
    color_extractor::suggest(&front_path, &back_path)
}

/// Exporte le **PDF complet de couverture** (300 DPI) à `output_pdf_path`.
#[tauri::command]
pub fn export_kdp_cover_pdf(
    params: CoverRenderParams,
    output_pdf_path: String,
) -> Result<ExportReport, String> {
    typst_generator::render_cover(&params, &output_pdf_path)
}

/// Ouvre le sélecteur natif d'**image** pour un plat de couverture et renvoie le
/// chemin absolu choisi (`None` si l'utilisateur annule).
#[tauri::command]
pub async fn pick_cover_image(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Choisir une image de couverture")
            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "tiff", "bmp"])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| format!("Ouverture de la boîte de dialogue impossible : {e}"))?;

    Ok(picked
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned()))
}

/// Ouvre le sélecteur natif d'**enregistrement** pour le PDF de couverture et
/// renvoie le chemin choisi (`None` si annulé).
#[tauri::command]
pub async fn pick_cover_output_path(
    app: tauri::AppHandle,
    default_name: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Enregistrer la couverture KDP")
            .set_file_name(&default_name)
            .add_filter("PDF", &["pdf"])
            .blocking_save_file()
    })
    .await
    .map_err(|e| format!("Ouverture de la boîte de dialogue impossible : {e}"))?;

    Ok(picked
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned()))
}
