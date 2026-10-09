//! Désérialisation du payload de projet (frontend).
//!
//! Les **spécifications physiques KDP** (format de coupe, gouttière, marges,
//! fond perdu) ne sont **pas** définies ici : elles vivent dans le module
//! central [`crate::kdp`] (source unique de vérité). Ce module se limite au
//! modèle de données transmis par le frontend et à l'estimation du nombre de
//! pages (bornée par les limites KDP).

use serde::Deserialize;

/// Métadonnées du roman (onglet « Informations »). Champs camelCase côté JSON.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectMetadata {
    pub saga_title: String,
    pub book_title: String,
    pub subtitle: String,
    pub volume_number: String,
    pub author_name: String,
    pub contributor: String,
    pub year: String,
    pub isbn: String,
    pub publisher: String,
    pub print_location: String,
    pub copyright_text: String,
    pub website: String,
    pub other_books: String,
}

/// Configuration de mise en page du manuscrit exporté.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LayoutConfig {
    /// Format de coupe KDP (ex. « 6x9 »).
    pub trim_size: String,
    /// Police du corps de texte (sert aussi de police de couverture).
    pub body_font: String,
    /// Taille du corps (points).
    pub body_size: f64,
    /// Interligne (multiplicateur, ex. 1.15).
    pub line_spacing: f64,
    /// Alignement : `justify` ou `left`.
    pub text_alignment: String,
    /// Lettrine (initiale de chapitre mise en exergue).
    pub drop_cap: bool,
    /// Police du titre de chapitre (`"body"` = identique au corps).
    pub chapter_title_font: String,
    /// Taille du titre de chapitre (points).
    pub chapter_title_size: f64,
    /// Police des sous-titres (`"body"` = identique au corps).
    pub subtitle_font: String,
    /// Taille des sous-titres (points).
    pub subtitle_size: f64,
}

impl LayoutConfig {
    /// Résout une police « héritée » (`"body"` ou vide) vers la police du corps.
    ///
    /// Repli robuste : si la valeur est vide, on retombe sur la police du corps
    /// (jamais de police vide, qui ferait échouer la compilation Typst).
    pub fn resolve_font<'a>(&'a self, font: &'a str) -> &'a str {
        let font = font.trim();
        if font.is_empty() || font == "body" {
            self.body_font.as_str()
        } else {
            font
        }
    }
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            trim_size: "6x9".to_string(),
            body_font: "Garamond".to_string(),
            body_size: 11.0,
            line_spacing: 1.15,
            text_alignment: "justify".to_string(),
            drop_cap: true,
            chapter_title_font: "body".to_string(),
            chapter_title_size: 16.0,
            subtitle_font: "body".to_string(),
            subtitle_size: 14.0,
        }
    }
}

/// Options de projet (sous-ensemble utile à l'export).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectOptions {
    /// Estimation optionnelle du nombre de pages (épaisseur → gouttière).
    pub estimated_page_count: Option<u32>,
    /// Traitement colorimétrique des images : `color` | `grayscale`.
    pub image_color_mode: String,
    /// Inclure la **couverture** dans le `.docx` (rendu « livre complet », contrôle
    /// visuel). Par défaut `false` : l'intérieur KDP Print n'embarque **jamais** la
    /// couverture (fournie via un PDF séparé). Rendre ce drapeau paramétrable depuis
    /// le frontend supprime le chemin mort `read_cover`/`cover_spec`.
    pub include_cover: Option<bool>,
}

impl ProjectOptions {
    /// Les images doivent-elles être converties en **niveaux de gris** ?
    ///
    /// Repli sûr : en l'absence de configuration (chaîne vide), les images sont
    /// conservées telles quelles (couleur).
    pub fn grayscale_images(&self) -> bool {
        matches!(
            self.image_color_mode.trim().to_ascii_lowercase().as_str(),
            "grayscale" | "grey" | "gray"
        )
    }

    /// La **couverture** doit-elle être incluse dans le `.docx` généré ?
    ///
    /// Repli sûr : `false` par défaut (intérieur KDP Print sans couverture).
    pub fn include_cover(&self) -> bool {
        self.include_cover.unwrap_or(false)
    }
}

/// Dossiers de travail du projet.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectDirectories {
    /// Dossier unique « Mes sources » (peut contenir une surcharge `couverture.*`).
    pub sources: Option<String>,
}

/// Élément de l'arborescence (« chemin de fer »), forme hiérarchique sérialisée.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrganizationNode {
    pub id: String,
    /// `act` | `chapter` | `image` | `special`.
    #[serde(rename = "type")]
    pub kind: String,
    pub display_name: Option<String>,
    pub source_file_name: Option<String>,
    /// Rôle éditorial des pages spéciales (`dedication`, `glossary`, …).
    pub role: Option<String>,
    pub children: Vec<OrganizationNode>,
}

/// Payload complet du projet (sous-ensemble ; les champs inconnus sont ignorés).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectPayload {
    pub metadata: ProjectMetadata,
    pub layout_config: LayoutConfig,
    pub options: ProjectOptions,
    pub directories: ProjectDirectories,
    /// Arborescence ordonnée (actes → chapitres / images / pages spéciales).
    pub organization: Vec<OrganizationNode>,
}

/// Estime le nombre de pages du manuscrit.
///
/// Si le frontend fournit une estimation explicite, elle est utilisée telle
/// quelle (bornée par les limites de pagination KDP). Sinon, repli déterministe
/// `MIN_PAGES + chapitres × 20`, borné `[MIN_PAGES, MAX_PAGES]`.
///
/// Les bornes proviennent du module central [`crate::kdp`] : aucune constante
/// physique n'est dupliquée ici.
pub fn estimate_page_count(payload: &ProjectPayload) -> u32 {
    use crate::kdp::{MAX_PAGES, MIN_PAGES};
    if let Some(count) = payload.options.estimated_page_count {
        return count.clamp(MIN_PAGES, MAX_PAGES);
    }
    let chapters = payload.organization.iter().map(count_chapters).sum::<u32>();
    (MIN_PAGES + chapters * 20).clamp(MIN_PAGES, MAX_PAGES)
}

/// Compte récursivement les nœuds de type `chapter`.
fn count_chapters(node: &OrganizationNode) -> u32 {
    let own = if node.kind == "chapter" { 1 } else { 0 };
    own + node.children.iter().map(count_chapters).sum::<u32>()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Construit un payload depuis un JSON partiel (les champs absents prennent
    /// leur valeur par défaut, grâce à `#[serde(default)]`).
    fn payload(json: serde_json::Value) -> ProjectPayload {
        serde_json::from_value(json).expect("payload valide")
    }

    // NOTE : les règles physiques KDP (formats de coupe, gouttière) sont
    // testées dans le module central `crate::kdp` (`src/kdp.rs`), et non ici :
    // `project.rs` ne contient plus aucune logique métier d'impression.

    #[test]
    fn estimate_uses_provided_count_and_clamps_to_kdp_bounds() {
        // Valeur explicite respectée dans les bornes.
        assert_eq!(
            estimate_page_count(&payload(serde_json::json!({
                "options": { "estimatedPageCount": 300 }
            }))),
            300
        );
        // Borne basse : jamais moins de 24 pages.
        assert_eq!(
            estimate_page_count(&payload(serde_json::json!({
                "options": { "estimatedPageCount": 5 }
            }))),
            24
        );
        // Borne haute : jamais plus de 828 pages.
        assert_eq!(
            estimate_page_count(&payload(serde_json::json!({
                "options": { "estimatedPageCount": 5000 }
            }))),
            828
        );
    }

    #[test]
    fn estimate_counts_chapters_recursively_when_absent() {
        // Sans estimation : 24 + chapitres × 20 → 24 + 2 × 20 = 64.
        let value = payload(serde_json::json!({
            "organization": [
                { "type": "act", "children": [ { "type": "chapter" }, { "type": "chapter" } ] }
            ]
        }));
        assert_eq!(estimate_page_count(&value), 64);
    }

    #[test]
    fn estimate_clamps_minimum_without_any_chapter() {
        let value = payload(serde_json::json!({ "organization": [] }));
        assert_eq!(estimate_page_count(&value), 24);
    }

    #[test]
    fn estimate_clamps_maximum_with_many_chapters() {
        // 60 chapitres → 24 + 1200 = 1224, borné à 828.
        let chapters: Vec<serde_json::Value> = (0..60)
            .map(|_| serde_json::json!({ "type": "chapter" }))
            .collect();
        let value = payload(serde_json::json!({
            "organization": [ { "type": "act", "children": chapters } ]
        }));
        assert_eq!(estimate_page_count(&value), 828);
    }

    #[test]
    fn count_chapters_is_recursive_and_ignores_other_kinds() {
        let node: OrganizationNode = serde_json::from_value(serde_json::json!({
            "type": "act",
            "children": [
                { "type": "chapter" },
                { "type": "act", "children": [ { "type": "chapter" }, { "type": "image" } ] }
            ]
        }))
        .unwrap();
        assert_eq!(count_chapters(&node), 2);
    }

    #[test]
    fn grayscale_detection_is_case_insensitive_and_trimmed() {
        let options = |mode: &str| ProjectOptions {
            estimated_page_count: None,
            image_color_mode: mode.to_string(),
            include_cover: None,
        };
        assert!(options("grayscale").grayscale_images());
        assert!(options("  Grayscale ").grayscale_images());
        assert!(options("Grey").grayscale_images());
        assert!(options("GRAY").grayscale_images());
        assert!(!options("color").grayscale_images());
        assert!(!options("").grayscale_images());
    }

    #[test]
    fn layout_config_default_is_6x9_garamond_justified() {
        let layout = LayoutConfig::default();
        assert_eq!(layout.trim_size, "6x9");
        assert_eq!(layout.body_font, "Garamond");
        assert_eq!(layout.body_size, 11.0);
        assert_eq!(layout.line_spacing, 1.15);
        assert_eq!(layout.text_alignment, "justify");
        assert!(layout.drop_cap);
    }

    #[test]
    fn deserialization_applies_defaults_for_missing_fields() {
        let value = payload(serde_json::json!({}));
        assert_eq!(value.layout_config.trim_size, "6x9");
        assert_eq!(value.metadata.book_title, "");
        assert!(value.organization.is_empty());
        assert!(value.directories.sources.is_none());
    }
}
