//! Génération du fichier `.docx` (module racine) : orchestration de la
//! construction (couverture optionnelle, pages liminaires, corps), puis
//! post-traitement du paquet OOXML (en-têtes courants, folio, marges miroir).
//!
//! Le code est réparti en sous-modules :
//! - [`helpers`] : utilitaires transverses (polices, XML, chemins, images) ;
//! - [`front_matter`] : couverture + pages liminaires ;
//! - [`body`] : arborescence, sources, Markdown et illustrations ;
//! - [`post_process`] : manipulation ZIP/OOXML et en-têtes courants.

use docx_rs::*;
use tauri::Manager;

use crate::kdp::{
    gutter_twips, trim_size_twips, BOTTOM_MARGIN_TWIPS, SIDE_MARGIN_TWIPS, TOP_MARGIN_TWIPS,
};
use crate::project::{estimate_page_count, OrganizationNode, ProjectPayload};

/// Journal de diagnostic **conditionné à la compilation**.
///
/// Émet sur `stderr` **uniquement en build de développement** (`debug_assertions`) :
/// les traces verbeuses (payload reçu, arborescence, correspondance section ↔
/// en-tête) disparaissent des binaires *release*, éliminant la fuite d'internes et
/// le coût d'E/S en production. En release, les arguments sont tout de même
/// « consommés » (`format_args!`, paresseux) pour ne déclencher aucun warning
/// « variable inutilisée ».
macro_rules! trace {
    ($($arg:tt)*) => {{
        #[cfg(debug_assertions)]
        {
            eprintln!($($arg)*);
        }
        #[cfg(not(debug_assertions))]
        {
            let _ = format_args!($($arg)*);
        }
    }};
}

mod body;
mod front_matter;
mod helpers;
mod post_process;

#[cfg(test)]
mod tests;

use body::{build_body, bullet_numbering, decimal_numbering};
use front_matter::{build_front_matter, cover_spec};
use helpers::no_footers;
use post_process::post_process;

/// Nom du fichier de couverture embarqué dans les ressources.
const COVER_RESOURCE: &str = "assets/Couverture_Cuir_01.jpg";
/// Extensions acceptées pour une surcharge `couverture.*`.
const COVER_EXTENSIONS: [&str; 5] = [".jpg", ".jpeg", ".png", ".webp", ".tiff"];

/// Nom de fichier de sortie proposé à l'utilisateur (slug du titre).
pub fn default_file_name(payload: &ProjectPayload) -> String {
    let title = payload.metadata.book_title.trim();
    let slug: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let slug = slug.trim_matches('_').to_string();
    if slug.is_empty() {
        "manuscrit.docx".to_string()
    } else {
        format!("{slug}.docx")
    }
}

/// Charge les octets de la couverture : surcharge `couverture.*` (dossier « Mes sources »)
/// → ressource empaquetée → repli `src-tauri/assets/` (développement).
pub fn read_cover(app: &tauri::AppHandle, payload: &ProjectPayload) -> Option<Vec<u8>> {
    // 1) Surcharge personnalisée à la racine du dossier « Mes sources ».
    if let Some(dir) = payload
        .directories
        .sources
        .as_deref()
        .filter(|d| !d.is_empty())
    {
        if let Some(bytes) = read_custom_cover(std::path::Path::new(dir)) {
            return Some(bytes);
        }
    }
    // 2) Ressource empaquetée avec l'exécutable.
    if let Ok(path) = app
        .path()
        .resolve(COVER_RESOURCE, tauri::path::BaseDirectory::Resource)
    {
        if let Ok(bytes) = std::fs::read(&path) {
            return Some(bytes);
        }
    }
    // 3) Repli développement : dossier source du projet.
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("Couverture_Cuir_01.jpg"),
    )
    .ok()
}

/// Cherche un fichier `couverture.<ext>` (insensible à la casse) dans un dossier.
fn read_custom_cover(dir: &std::path::Path) -> Option<Vec<u8>> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name.starts_with("couverture.") && COVER_EXTENSIONS.iter().any(|ext| name.ends_with(ext))
        {
            if let Ok(bytes) = std::fs::read(entry.path()) {
                return Some(bytes);
            }
        }
    }
    None
}

/// Rappel de progression appelé aux étapes clés de la génération.
/// Reçoit un **libellé d'étape** et un **pourcentage global** (0–100).
pub type ProgressReporter<'a> = &'a dyn Fn(&str, u32);

/// **Trace de debug** de l'arborescence « Organisation » reçue du frontend.
///
/// Imprimée sur `stderr` (`eprintln!`) au lancement de chaque export, elle rend
/// **visible dans la console Tauri** la structure exacte transmise par l'UI :
/// nature de chaque nœud, titre de chapitre (`displayName`), chemin de fichier
/// des images (`sourceFileName`) et rôle des pages spéciales. Cela permet de
/// distinguer instantanément un problème de **données** (champ vide côté UI) d'un
/// problème de **moteur**.
fn trace_organization(nodes: &[OrganizationNode], depth: usize) {
    for node in nodes {
        let pad = "  ".repeat(depth);
        trace!(
            "{pad}• type={:<8} displayName={:?} sourceFileName={:?} role={:?}",
            node.kind,
            node.display_name,
            node.source_file_name,
            node.role
        );
        trace_organization(&node.children, depth + 1);
    }
}

/// Construit le paquet `.docx` (fichier **intérieur** KDP Print : pages liminaires + corps).
///
/// La **couverture n'est pas incluse** par défaut (`include_cover == false`) : le
/// fichier intérieur d'un livre broché ne doit jamais la contenir (PDF séparé).
/// Passer `include_cover = true` produit un rendu « livre complet » (contrôle visuel).
///
/// `progress` est invoqué aux étapes clés (lecture des sources, analyse,
/// structuration, injection de la table des matières et du colophon).
pub fn build_docx(
    payload: &ProjectPayload,
    cover: Option<&[u8]>,
    include_cover: bool,
    progress: ProgressReporter,
) -> Result<Vec<u8>, String> {
    // **Traçage des données réelles** reçues du frontend (console Tauri / stderr,
    // développement uniquement).
    trace!("────────── Export Word : payload reçu du frontend ──────────");
    trace!("directories.sources = {:?}", payload.directories.sources);
    trace!(
        "options.imageColorMode = {:?}",
        payload.options.image_color_mode
    );
    trace!("organization ({} racine(s)) :", payload.organization.len());
    trace_organization(&payload.organization, 1);

    let md = &payload.metadata;
    let font = if payload.layout_config.body_font.trim().is_empty() {
        "Garamond".to_string()
    } else {
        payload.layout_config.body_font.clone()
    };

    let (page_w, page_h) = trim_size_twips(&payload.layout_config.trim_size);
    let gutter = gutter_twips(estimate_page_count(payload)) as i32;

    // Marges KDP : miroir (vis-à-vis) + gouttière de reliure ajoutée à la marge
    // intérieure (`w:gutter`). Le miroir est activé au niveau document via
    // `<w:mirrorMargins/>` dans `settings.xml` (voir `post_process`).
    let kdp_margin = PageMargin::new()
        .top(TOP_MARGIN_TWIPS)
        .bottom(BOTTOM_MARGIN_TWIPS)
        .left(SIDE_MARGIN_TWIPS)
        .right(SIDE_MARGIN_TWIPS)
        .header(720)
        .footer(720)
        .gutter(gutter);

    // --- Assemblage : [couverture optionnelle], pages liminaires, puis corps. ---
    let mut specs: Vec<SectionSpec> = Vec::new();

    // La couverture est **exclue** du fichier intérieur KDP Print (fournie via un PDF
    // séparé) : par défaut le document commence par les pages liminaires (faux-titre).
    if include_cover {
        specs.push(cover_spec(payload, cover, &font, page_w, page_h));
    }

    specs.push(SectionSpec {
        paragraphs: build_front_matter(md, &font, page_h),
        page_w,
        page_h,
        margin: kdp_margin.clone(),
        title_pg: true,
        start_page: Some(1),
        // Pages liminaires : **aucun** en-tête ni numéro de page.
        running_label: None,
        footers: Some(no_footers()),
        page_numbered: false,
        vertical_center: false,
    });
    specs.extend(build_body(
        payload,
        &font,
        &kdp_margin,
        page_w,
        page_h,
        progress,
    )?);

    // Sections précédant le corps (couverture éventuelle + liminaires) : seules les
    // sections du corps reçoivent un saut `oddPage`.
    let body_start = if include_cover { 2 } else { 1 };

    // Index (dans l'ordre du document) des sections recevant le **folio**, et
    // libellés d'en-tête courant par section : calculés **avant** de consommer
    // `specs` (l'assemblage ci-dessous se fait par déplacement).
    let numbered: Vec<usize> = specs
        .iter()
        .enumerate()
        .filter(|(_, spec)| spec.page_numbered)
        .map(|(index, _)| index)
        .collect();
    let section_labels: Vec<Option<String>> = specs
        .iter()
        .map(|spec| spec.running_label.clone())
        .collect();
    // Index des sections à **centrer verticalement** (illustrations isolées).
    let centered_sections: Vec<usize> = specs
        .iter()
        .enumerate()
        .filter(|(_, spec)| spec.vertical_center)
        .map(|(index, _)| index)
        .collect();

    // Assemblage **par déplacement** : les paragraphes sont transférés au document
    // (`std::mem::take`) sans clone profond — gain mémoire/CPU sur les gros
    // manuscrits, où chaque `Paragraph` peut porter des runs et des images.
    let mut doc = Docx::new();
    let last = specs.len().saturating_sub(1);
    for (index, mut spec) in specs.into_iter().enumerate() {
        for paragraph in std::mem::take(&mut spec.paragraphs) {
            doc = doc.add_paragraph(paragraph);
        }
        if index == last {
            doc = apply_section_to_doc(doc, &spec);
        } else {
            doc = doc.add_section(section_from_spec(&spec));
        }
    }

    // Pages paires et impaires différentes (global) + styles de titres + listes.
    // Les styles **Heading1/Heading2** doivent être déclarés globalement, porter un
    // **niveau hiérarchique** (`outlineLvl`) et le **nom intégré** Word (« heading N »)
    // pour que le champ TOC (`\o "1-2"`) les collecte — sinon Word signale « aucun style
    // de titre » et la table des matières reste vide.
    let doc = doc
        .settings(Settings::new().even_and_odd_headers())
        .add_style(
            Style::new("Heading1", StyleType::Paragraph)
                .name("heading 1")
                .based_on("Normal")
                .next("Normal")
                .outline_lvl(0)
                .size(32)
                .bold(),
        )
        .add_style(
            Style::new("Heading2", StyleType::Paragraph)
                .name("heading 2")
                .based_on("Normal")
                .next("Normal")
                .outline_lvl(1)
                .size(28)
                .bold(),
        )
        .add_abstract_numbering(bullet_numbering())
        .add_numbering(Numbering::new(1, 1))
        .add_abstract_numbering(decimal_numbering())
        .add_numbering(Numbering::new(2, 2));

    // Sérialisation OOXML, puis post-traitement (sections hors corps).
    let mut cursor = std::io::Cursor::new(Vec::new());
    doc.build()
        .pack(&mut cursor)
        .map_err(|e| format!("génération du .docx : {e}"))?;
    post_process(
        cursor.into_inner(),
        body_start,
        &numbered,
        &font,
        &section_labels,
        &centered_sections,
    )
}

/// Spécification d'une section : contenu + propriétés + en-têtes/pieds.
struct SectionSpec {
    paragraphs: Vec<Paragraph>,
    page_w: u32,
    page_h: u32,
    margin: PageMargin,
    title_pg: bool,
    start_page: Option<u32>,
    /// **En-tête courant** à afficher sur les pages de la section (nom du chapitre
    /// saisi dans « Organisation »). `None` = aucun en-tête.
    ///
    /// Les parties d'en-tête sont créées **par post-traitement** (voir
    /// `inject_running_headers`) : `docx-rs` associe les en-têtes aux sections de
    /// façon erratique (décalage d'un cran), ce qui affichait le nom de l'acte sur
    /// les pages du chapitre.
    running_label: Option<String>,
    footers: Option<(Footer, Footer, Footer)>,
    /// La section reçoit-elle le **folio** (numéro de page) en pied de page ?
    /// Le folio est injecté par post-traitement (limitation `docx-rs` : une section
    /// ajoutée ne peut porter à la fois un en-tête et un pied de page).
    page_numbered: bool,
    /// La section doit-elle **centrer verticalement** son contenu ?
    /// Injecté par post-traitement comme `<w:vAlign w:val="center"/>` dans le
    /// `sectPr` (seule méthode fiable Word : illustration isolée & centrée).
    vertical_center: bool,
}

/// Construit une `Section` (saut de section) à partir d'une spécification.
///
/// Les **en-têtes** ne sont **pas** posés ici : ils sont injectés par
/// post-traitement (`inject_running_headers`) pour garantir que chaque section
/// référence le bon flux d'en-tête.
fn section_from_spec(spec: &SectionSpec) -> Section {
    let mut section = Section::new()
        .page_size(PageSize::new().size(spec.page_w, spec.page_h))
        .page_margin(spec.margin.clone());
    if spec.title_pg {
        section = section.title_pg();
    }
    if let Some(start) = spec.start_page {
        section = section.page_num_type(PageNumType::new().start(start));
    }
    if let Some((footer, even, first)) = &spec.footers {
        section = section
            .footer(footer.clone())
            .even_footer(even.clone())
            .first_footer(first.clone());
    }
    section
}

/// Applique une spécification au dernier segment (section de niveau document).
fn apply_section_to_doc(doc: Docx, spec: &SectionSpec) -> Docx {
    let mut doc = doc
        .page_size(spec.page_w, spec.page_h)
        .page_margin(spec.margin.clone());
    if spec.title_pg {
        doc = doc.title_pg();
    }
    if let Some(start) = spec.start_page {
        doc = doc.page_num_type(PageNumType::new().start(start));
    }
    if let Some((footer, even, first)) = &spec.footers {
        doc = doc
            .footer(footer.clone())
            .even_footer(even.clone())
            .first_footer(first.clone());
    }
    doc
}
