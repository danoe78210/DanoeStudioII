use super::front_matter::*;
use super::helpers::*;
use super::post_process::*;
use super::*;

// Sous-modules de tests unitaires, éclatés par responsabilité (SRP) :
// - `helpers`      : utilitaires transverses (XML, dimensions, résolution de chemins) ;
// - `post_process` : manipulation OOXML/ZIP (en-têtes, folio, sauts de section) ;
// - `front_matter` : pages liminaires (hauteurs de ligne, espaceurs, couverture).
// Les tests d'intégration bout-en-bout (`build_docx`) restent dans ce module.
mod front_matter;
mod helpers;
mod post_process;

/// PNG 1×1 valide (transparent) servant d'image de couverture de test.
const TINY_PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4, 0,
    0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 100, 96, 248, 95, 15, 0, 2,
    135, 1, 128, 235, 71, 186, 146, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

fn sample_payload() -> ProjectPayload {
    serde_json::from_value(serde_json::json!({
        "metadata": {
            "bookTitle": "Nunael",
            "subtitle": "Chroniques du Nord",
            "sagaTitle": "Les Schattenjägers",
            "authorName": "Danoë",
            "publisher": "Éditions Test",
            "year": "2026",
            "isbn": "978-2-000000-00-0",
            "printLocation": "France"
        },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "organization": [
            { "type": "act", "children": [ { "type": "chapter" }, { "type": "chapter" } ] }
        ]
    }))
    .expect("payload valide")
}

fn read_entry(bytes: &[u8], name: &str) -> String {
    use std::io::Read;
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).expect("zip lisible");
    let mut file = archive.by_name(name).expect("entrée présente");
    let mut content = String::new();
    file.read_to_string(&mut content).expect("lecture utf-8");
    content
}

fn entry_names(bytes: &[u8]) -> Vec<String> {
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).expect("zip lisible");
    (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
        .collect()
}

/// Génère un PNG **valide** (couleur) via la crate `image`, pour tester la
/// conversion Noir & Blanc (`TINY_PNG` sert uniquement à la couverture).
fn sample_png(width: u32, height: u32) -> Vec<u8> {
    let mut img = image::RgbImage::new(width, height);
    for pixel in img.pixels_mut() {
        *pixel = image::Rgb([180, 40, 60]);
    }
    let mut png = std::io::Cursor::new(Vec::new());
    img.write_to(&mut png, image::ImageFormat::Png)
        .expect("encodage PNG");
    png.into_inner()
}

fn read_entry_bytes(bytes: &[u8], name: &str) -> Vec<u8> {
    use std::io::Read;
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).expect("zip lisible");
    let mut file = archive.by_name(name).expect("entrée présente");
    let mut content = Vec::new();
    file.read_to_end(&mut content).expect("lecture binaire");
    content
}

#[test]
fn build_docx_reports_progress_steps() {
    let payload = sample_payload();
    let steps = std::cell::RefCell::new(Vec::<(String, u32)>::new());
    let _ = build_docx(&payload, None, false, &|step, percent| {
        steps.borrow_mut().push((step.to_string(), percent));
    });
    let percents: Vec<u32> = steps.into_inner().into_iter().map(|(_, p)| p).collect();
    for expected in [10, 30, 60, 80] {
        assert!(
            percents.contains(&expected),
            "étape {expected}% manquante : {percents:?}"
        );
    }
}

#[test]
fn build_docx_declares_heading_styles_for_toc() {
    let payload = sample_payload();
    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");

    // Styles déclarés GLOBALEMENT (styles.xml) : nom intégré Word (« heading N ») +
    // niveau hiérarchique (`outlineLvl`) — sans quoi Word ignore l'assignation locale
    // et la table des matières reste vide.
    let styles = read_entry(&bytes, "word/styles.xml");
    assert!(
        styles.contains("w:styleId=\"Heading1\""),
        "style Heading1 déclaré"
    );
    assert!(
        styles.contains("w:styleId=\"Heading2\""),
        "style Heading2 déclaré"
    );
    assert!(
        styles.contains("<w:name w:val=\"heading 1\""),
        "nom intégré « heading 1 »"
    );
    assert!(
        styles.contains("<w:name w:val=\"heading 2\""),
        "nom intégré « heading 2 »"
    );
    assert!(
        styles.contains("<w:outlineLvl w:val=\"0\""),
        "outlineLvl 0 (Heading1)"
    );
    assert!(
        styles.contains("<w:outlineLvl w:val=\"1\""),
        "outlineLvl 1 (Heading2)"
    );

    // Titres de section/acte : attribut local `w:pStyle` (Titre 1) pour la TOC.
    let document = read_entry(&bytes, "word/document.xml");
    assert!(
        document.contains("<w:pStyle w:val=\"Heading1\""),
        "titre de section en style Titre 1"
    );

    // Saga (métadonnée) présente sur les pages liminaires.
    assert!(
        document.contains("LES SCHATTENJÄGERS"),
        "titre de saga sur les liminaires"
    );
}

#[test]
fn build_docx_applies_cover_and_mirror_margins() {
    let payload = sample_payload();
    // `include_cover = true` → rendu « livre complet » (couverture incluse).
    let bytes = build_docx(&payload, Some(TINY_PNG), true, &|_, _| {}).expect("génération");

    let document = read_entry(&bytes, "word/document.xml");
    // Texte superposé de couverture.
    assert!(
        document.contains("NUNAEL"),
        "titre de couverture en majuscules"
    );
    assert!(document.contains("Chroniques du Nord"), "sous-titre");
    assert!(document.contains("Danoë"), "auteur");
    // Image « derrière le texte », habillage sans.
    assert!(
        document.contains("behindDoc=\"1\""),
        "image derrière le texte"
    );
    assert!(
        document.contains("<wp:wrapNone"),
        "habillage sans (wrapNone)"
    );
    // Section de couverture : marges nulles.
    assert!(document.contains("w:top=\"0\""), "couverture plein bord");
    // Page de copyright alimentée par les métadonnées.
    assert!(document.contains("ISBN : 978-2-000000-00-0"));
    assert!(document.contains("Imprimé en France"));

    let settings = read_entry(&bytes, "word/settings.xml");
    let mirror = settings.find("<w:mirrorMargins").expect("marges miroir");
    assert!(
        settings.contains("evenAndOddHeaders"),
        "pages paires/impaires différentes"
    );
    // `w:mirrorMargins` doit **précéder** `w:evenAndOddHeaders` (séquence CT_Settings),
    // sinon Word peut ignorer le miroir (et donc la gouttière de reliure).
    if let Some(eooh) = settings.find("<w:evenAndOddHeaders") {
        assert!(mirror < eooh, "mirrorMargins placé avant evenAndOddHeaders");
    }

    // Foliotation & première page différente.
    assert!(document.contains("pgNumType"), "type de numérotation");
    assert!(
        document.contains("w:start=\"1\""),
        "numérotation à partir de 1"
    );
    assert!(document.contains("titlePg"), "première page différente");

    // En-têtes (running heads) et pieds de page.
    let headers: String = entry_names(&bytes)
        .iter()
        .filter(|name| name.starts_with("word/header"))
        .map(|name| read_entry(&bytes, name))
        .collect();
    assert!(
        headers.contains("Chapitre"),
        "en-tête = nom du chapitre courant"
    );
    assert!(
        headers.contains("w:pBdr") && headers.contains("w:bottom"),
        "trait de séparation sous l'en-tête"
    );
    let footers: String = entry_names(&bytes)
        .iter()
        .filter(|name| name.starts_with("word/footer"))
        .map(|name| read_entry(&bytes, name))
        .collect();
    assert!(footers.contains("PAGE"), "champ de numérotation de page");
}

#[test]
fn build_docx_excludes_cover_from_interior() {
    let payload = sample_payload();
    // Une image de couverture est fournie MAIS `include_cover = false` :
    // le fichier intérieur KDP Print doit démarrer sur les liminaires.
    let bytes = build_docx(&payload, Some(TINY_PNG), false, &|_, _| {}).expect("génération");
    let document = read_entry(&bytes, "word/document.xml");

    // Aucune couverture : pas d'image « derrière le texte », pas de section plein bord.
    assert!(
        !document.contains("behindDoc"),
        "aucune image de couverture"
    );
    assert!(
        !document.contains("w:top=\"0\""),
        "aucune section de couverture à marges nulles"
    );
    // Le document commence bien par les pages liminaires (page de copyright).
    assert!(
        document.contains("ISBN : 978-2-000000-00-0"),
        "liminaires présents"
    );

    // Gouttière de reliure injectée dans les propriétés de section (`<w:pgMar ... w:gutter=…>`).
    let gutter = crate::kdp::gutter_twips(crate::project::estimate_page_count(&payload));
    assert!(gutter > 0, "gouttière non nulle");
    assert!(
        document.contains(&format!("w:gutter=\"{gutter}\"")),
        "gouttière {gutter} twips injectée dans w:pgMar"
    );
}

// NOTE : les règles de format de coupe et de gouttière KDP sont validées
// exclusivement dans le module central `crate::kdp` (`src/kdp.rs`), par des
// tests métier purs indépendants de tout fichier généré. Le test d'intégration
// ci-dessus se limite à vérifier leur **injection** dans le `.docx`.

#[test]
fn build_docx_renders_chapters_notes_and_glossary() {
    use std::fs;
    use std::io::Write;

    // Dossier temporaire servant de « Mes sources ».
    let dir = std::env::temp_dir().join(format!("danoe-studio-test-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    fs::write(dir.join("carte.png"), TINY_PNG).expect("création carte.png");
    let mut chapter = fs::File::create(dir.join("ch1.md")).expect("création ch1.md");
    writeln!(chapter, "# Le Dernier Souffle").unwrap();
    writeln!(chapter).unwrap();
    writeln!(chapter, "Premier paragraphe avec une note[^neume] et du **gras**, prolongé par une longue phrase narrative destinée à occuper plusieurs lignes de texte afin que la lettrine s'y intègre sans déborder du cadre qui la contient.").unwrap();
    writeln!(chapter).unwrap();
    writeln!(chapter, "## Une sous-partie").unwrap();
    writeln!(chapter).unwrap();
    writeln!(chapter, "Second paragraphe en *italique*.").unwrap();
    writeln!(chapter).unwrap();
    writeln!(chapter, "---").unwrap();
    writeln!(chapter).unwrap();
    writeln!(chapter, "> Une citation mémorable.").unwrap();
    writeln!(chapter).unwrap();
    writeln!(chapter, "- Premier point").unwrap();
    writeln!(chapter).unwrap();
    writeln!(chapter, "1. Étape un").unwrap();
    writeln!(chapter).unwrap();
    writeln!(chapter, "![Carte](carte.png)").unwrap();
    writeln!(chapter).unwrap();
    writeln!(chapter, "[^neume]: Neumes : premiers symboles.").unwrap();

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "Nunael", "authorName": "Danoë", "year": "2026" },
        "layoutConfig": {
            "trimSize": "6x9", "bodyFont": "Garamond",
            "lineSpacing": 1.15, "textAlignment": "justify"
        },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "act", "displayName": "Acte 1", "children": [
                { "type": "chapter", "displayName": "Chapitre 1", "sourceFileName": "ch1.md" },
                { "type": "special", "role": "glossary", "displayName": "Glossaire" }
            ] }
        ]
    }))
    .expect("payload valide");

    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");
    let document = read_entry(&bytes, "word/document.xml");

    // Titres (`#` → Titre 1, `##` → Titre 2) + corps.
    assert!(document.contains("Le Dernier Souffle"), "titre # capturé");
    assert!(document.contains("Heading1"), "style Titre 1");
    assert!(document.contains("Une sous-partie"), "titre ##");
    // Lettrine : la 1ʳᵉ lettre du 1ᵉʳ paragraphe (juste après le Titre 1) est détachée
    // (« P » + « remier paragraphe… ») via un cadre `w:framePr` + `w:dropCap`.
    assert!(
        document.contains("w:dropCap=\"drop\""),
        "lettrine (w:dropCap)"
    );
    assert!(document.contains("w:lines=\"2\""), "lettrine sur 2 lignes");
    assert!(document.contains("w:framePr"), "cadre de lettrine");
    assert!(
        document.contains("w:wrap=\"around\""),
        "habillage autour de la lettrine"
    );
    assert_eq!(
        document.matches("w:dropCap=\"drop\"").count(),
        1,
        "une seule lettrine (1 chapitre)"
    );
    assert!(
        document.contains("remier paragraphe"),
        "corps du texte après lettrine"
    );
    assert!(document.contains("gras"), "fragment en gras");
    assert!(document.contains("italique"), "fragment en italique");
    // Séparateur de scène + citation.
    assert!(document.contains("* * *"), "séparateur de scène");
    assert!(document.contains("Une citation mémorable"), "citation");
    // Listes natives (puces + numérotée).
    assert!(document.contains("w:numId"), "liste native");
    // Image de corps (partie média embarquée).
    assert!(
        entry_names(&bytes)
            .iter()
            .any(|name| name.starts_with("word/media/")),
        "image embarquée"
    );
    // Note : appel en exposant.
    assert!(
        document.contains("superscript"),
        "appel de note en exposant"
    );
    // Glossaire : plus de syntaxe littérale, définition + regroupement + Titre 2.
    assert!(!document.contains("[^"), "pas de syntaxe littérale [^n]");
    assert!(document.contains("Neumes"), "définition du glossaire");
    assert!(
        document.contains("Acte 1, chapitre 1"),
        "regroupement par chapitre"
    );
    assert!(document.contains("Heading2"), "sous-titre en style Titre 2");
    // Sauts de section `oddPage` des chapitres.
    assert!(document.contains("w:val=\"oddPage\""), "sauts oddPage");

    // Table des matières native + achevé d'imprimer.
    assert!(document.contains("Table des matières"), "titre du sommaire");
    assert!(document.contains("TOC"), "champ TOC natif");
    assert!(
        document.contains("par France"),
        "achevé d'imprimer (colophon)"
    );
    assert!(document.contains("Dépôt légal"), "dépôt légal");
    let settings = read_entry(&bytes, "word/settings.xml");
    assert!(settings.contains("updateFields"), "mise à jour des champs");

    // En-tête dynamique = **nom du chapitre saisi dans « Organisation »**
    // (et non le titre `#` du fichier source).
    let headers: String = entry_names(&bytes)
        .iter()
        .filter(|name| name.starts_with("word/header"))
        .map(|name| read_entry(&bytes, name))
        .collect();
    assert!(
        headers.contains("Chapitre 1"),
        "en-tête dynamique (nom du chapitre / Organisation)"
    );
    assert!(
        !headers.contains("Le Dernier Souffle"),
        "le titre # du fichier source ne sert plus d'en-tête"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn build_docx_renders_transition_image_and_strips_frontmatter() {
    use std::fs;
    use std::io::Write;

    let dir = std::env::temp_dir().join(format!("danoe-studio-test-img-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("dossier temporaire");
    fs::write(dir.join("transition.png"), sample_png(4, 4)).expect("création transition.png");

    // Chapitre précédé d'un frontmatter YAML (Obsidian) à supprimer.
    let mut chapter = fs::File::create(dir.join("ch1.md")).expect("création ch1.md");
    writeln!(chapter, "---").unwrap();
    writeln!(chapter, "title: Hildegarde").unwrap();
    writeln!(chapter, "tome: 7").unwrap();
    writeln!(chapter, "pov: Ada").unwrap();
    writeln!(chapter, "statut: brouillon").unwrap();
    writeln!(chapter, "---").unwrap();
    writeln!(chapter, "# Le Réveil").unwrap();
    writeln!(chapter).unwrap();
    writeln!(chapter, "Corps du chapitre.").unwrap();

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "Hildegarde", "authorName": "Danoë", "year": "2026" },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "options": { "imageColorMode": "grayscale" },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "image", "displayName": "Transition", "sourceFileName": "transition.png" },
            { "type": "act", "displayName": "Acte 1", "children": [
                { "type": "chapter", "displayName": "Chapitre 1", "sourceFileName": "ch1.md" }
            ] }
        ]
    }))
    .expect("payload valide");

    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");
    let document = read_entry(&bytes, "word/document.xml");

    // Frontmatter Obsidian intégralement supprimé du manuscrit.
    assert!(!document.contains("tome:"), "métadonnée tome: supprimée");
    assert!(!document.contains("pov:"), "métadonnée pov: supprimée");
    assert!(
        !document.contains("statut:"),
        "métadonnée statut: supprimée"
    );
    assert!(
        !document.contains("brouillon"),
        "valeur de métadonnée supprimée"
    );
    // Corps du chapitre conservé.
    assert!(document.contains("Le Réveil"), "titre du chapitre conservé");
    // La lettrine détache le « C » initial (« C » + « orps du chapitre. »).
    assert!(document.contains("orps du chapitre"), "corps conservé");
    // Illustration de transition embarquée dans le paquet OOXML (on ignore
    // l'entrée de dossier `word/media/`).
    let media = entry_names(&bytes)
        .into_iter()
        .find(|name| name.starts_with("word/media/") && !name.ends_with('/'))
        .expect("illustration de transition embarquée");
    // Mode `grayscale` configuré → l'image embarquée est en niveaux de gris (L8).
    let embedded = read_entry_bytes(&bytes, &media);
    let decoded = image::load_from_memory(&embedded).expect("image embarquée décodable");
    assert_eq!(
        decoded.color(),
        image::ColorType::L8,
        "illustration convertie en niveaux de gris"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn apply_image_color_mode_converts_to_grayscale() {
    let original = sample_png(2, 2);

    // Mode couleur : octets strictement inchangés.
    assert_eq!(apply_image_color_mode(original.clone(), false), original);

    // Mode Noir & Blanc : l'image ré-encodée est en niveaux de gris (L8).
    let gray = apply_image_color_mode(original, true);
    let decoded = image::load_from_memory(&gray).expect("décodage N&B");
    assert_eq!(
        decoded.color(),
        image::ColorType::L8,
        "image convertie en niveaux de gris"
    );
}

#[test]
fn build_docx_resolves_image_case_insensitively() {
    use std::fs;

    let dir = std::env::temp_dir().join(format!("danoe-studio-test-ci-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("dossier temporaire");
    // Fichier présent en minuscules ; référencé en majuscules dans le projet.
    fs::write(dir.join("transition.png"), sample_png(4, 4)).expect("création transition.png");

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "Hugo", "authorName": "Danoë" },
        "layoutConfig": { "trimSize": "6x9" },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "image", "sourceFileName": "Transition.PNG" },
            { "type": "act", "displayName": "Acte 1", "children": [] }
        ]
    }))
    .expect("payload valide");

    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");
    assert!(
        entry_names(&bytes)
            .iter()
            .any(|name| name.starts_with("word/media/") && !name.ends_with('/')),
        "illustration résolue malgré une différence de casse"
    );

    let _ = fs::remove_dir_all(&dir);
}

/// Vérifie qu'**aucune** partie XML du paquet n'est corrompue : chaque partie doit
/// contenir exactement **une** déclaration XML, placée en tête (une déclaration
/// dupliquée ou précédée d'un blanc rend la partie invalide → Word refuse le fichier).
#[test]
fn all_xml_parts_have_single_declaration() {
    let payload = sample_payload();
    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");
    for name in entry_names(&bytes) {
        if !(name.ends_with(".xml") || name.ends_with(".rels")) {
            continue;
        }
        let content = read_entry(&bytes, &name);
        assert!(
            content.matches("<?xml").count() <= 1,
            "au plus une déclaration XML dans {name}"
        );
        if content.contains("<?xml") {
            assert!(
                content.starts_with("<?xml"),
                "{name} : la déclaration XML doit être le premier nœud"
            );
        }
    }
}

#[test]
fn footer_page_field_is_well_formed() {
    let payload = sample_payload();
    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");

    // Le folio est injecté dans une partie dédiée, référencée par les sections
    // du corps (numéro de page).
    let footer = read_entry(&bytes, FOLIO_FOOTER_PART);
    // **Régression** : la partie ne doit contenir qu'**une seule** déclaration
    // XML (une déclaration dupliquée rend la partie invalide et Word refuse
    // d'ouvrir le fichier).
    assert_eq!(
        footer.matches("<?xml").count(),
        1,
        "déclaration XML unique dans la partie pied de page"
    );
    assert!(footer.starts_with("<?xml"), "déclaration XML en tête");
    assert!(
        footer.contains("<w:instrText>PAGE</w:instrText>"),
        "instruction de champ PAGE"
    );
    // Le champ `PAGE` doit être éclaté en **runs séparés** : la balise
    // `fldChar begin` est refermée (`</w:r>`) avant l'`instrText`. Sinon certains
    // lecteurs affichent le mot « PAGE » au lieu du numéro.
    assert!(
        footer.contains("w:fldCharType=\"begin\" w:dirty=\"true\" /></w:r>"),
        "fldChar de début dans son propre run"
    );
    assert!(
        !footer.contains("w:fldCharType=\"begin\" w:dirty=\"false\" /><w:instrText>"),
        "champ PAGE non monolithique"
    );

    // Référence de pied de page présente dans `document.xml` (sections du corps).
    let document = read_entry(&bytes, "word/document.xml");
    assert!(
        document.contains(&format!("r:id=\"{FOLIO_FOOTER_RID}\"")),
        "référence de pied de page injectée"
    );
    // Type de contenu déclaré pour la nouvelle partie.
    let content_types = read_entry(&bytes, "[Content_Types].xml");
    assert!(
        content_types.contains(FOLIO_FOOTER_PART),
        "type de contenu du folio déclaré"
    );
}

#[test]
fn front_matter_anchors_bottom_blocks() {
    // `sample_payload` : 6×9, avec ISBN + lieu d'impression + saga/sous-titre/auteur.
    // Hauteur utile = 12960 − 1440 − 1080 = 10440 twips.
    let payload = sample_payload();
    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");
    let document = read_entry(&bytes, "word/document.xml");

    // Page 2 (copyright) : hauteur du bloc = 240 + (180+240) + 180 + 240 = 1080 ;
    // espaceur de tête = 10440 − 60 − 1080 = 9300 (hauteur de ligne **exacte**,
    // jamais supprimée en tête de page).
    assert!(
        document.contains("w:line=\"9300\""),
        "bloc de copyright ancré en bas de la page 2"
    );
    // Page 3 (mention d'édition) : bloc de titre = 2880 + 6048 ;
    // espaceur = 10440 − 60 − 6048 − 288 = 4044.
    assert!(
        document.contains("w:line=\"4044\""),
        "mention d'édition ancrée en bas de la page 3"
    );
    // Hauteurs de ligne figées (`lineRule="exact"`) pour un ancrage déterministe.
    assert!(
        document.contains("w:lineRule=\"exact\""),
        "hauteur de ligne figée pour l'ancrage bas de page"
    );
}

/// Chunk de `document.xml` délimité par `</w:p>` contenant `needle` (permet
/// d'isoler le paragraphe d'un titre et d'inspecter ses propriétés).
fn paragraph_chunk<'a>(xml: &'a str, needle: &str) -> &'a str {
    xml.split("</w:p>")
        .find(|chunk| chunk.contains(needle))
        .unwrap_or_else(|| panic!("paragraphe « {needle} » introuvable"))
}

#[test]
fn subtitle_always_page_breaks() {
    use std::fs;
    use std::io::Write;

    let dir = std::env::temp_dir().join(format!("danoe-studio-test-h2-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);

    // Chapitre A : `##` isolé (précédé d'un paragraphe) → saut de page attendu.
    let mut a = fs::File::create(dir.join("a.md")).expect("création a.md");
    writeln!(a, "# Chapitre Alpha").unwrap();
    writeln!(a).unwrap();
    writeln!(a, "Un paragraphe d'ouverture.").unwrap();
    writeln!(a).unwrap();
    writeln!(a, "## Sous-titre Tardif").unwrap();
    writeln!(a).unwrap();
    writeln!(a, "Suite du texte.").unwrap();

    // Chapitre B : `##` immédiatement après le `#` → saut de page **aussi**.
    let mut b = fs::File::create(dir.join("b.md")).expect("création b.md");
    writeln!(b, "# Chapitre Beta").unwrap();
    writeln!(b, "## Sous-titre Immediat").unwrap();
    writeln!(b).unwrap();
    writeln!(b, "Corps du chapitre.").unwrap();

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "Test", "authorName": "Auteur", "year": "2026" },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "chapter", "displayName": "Chapitre Alpha", "sourceFileName": "a.md" },
            { "type": "chapter", "displayName": "Chapitre Beta", "sourceFileName": "b.md" }
        ]
    }))
    .expect("payload valide");

    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");
    let document = read_entry(&bytes, "word/document.xml");

    // **Règle structurelle** : tout sous-titre `##` saut de page, sans exception.
    assert!(
        document.contains("<w:pageBreakBefore"),
        "saut de page présent dans le document"
    );
    assert!(
        paragraph_chunk(&document, "Sous-titre Tardif").contains("<w:pageBreakBefore"),
        "saut de page avant le sous-titre isolé"
    );
    assert!(
        paragraph_chunk(&document, "Sous-titre Immediat").contains("<w:pageBreakBefore"),
        "saut de page systématique, même après le titre de chapitre"
    );

    let _ = fs::remove_dir_all(&dir);
}
/// Le titre de chapitre concatène le **numéro** (Organisation) et le **nom** (`#`),
/// et un **saut de page obligatoire** suit le titre (corps sur la page suivante).
#[test]
fn chapter_title_combines_number_and_name_and_breaks_after() {
    use std::fs;
    use std::io::Write;

    let dir = std::env::temp_dir().join(format!("danoe-chap-title-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let mut c = fs::File::create(dir.join("c1.md")).unwrap();
    writeln!(c, "# Le Dernier Soir").unwrap();
    writeln!(c).unwrap();
    writeln!(c, "La flamme se courbe.").unwrap();

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "Nunael", "authorName": "Dano", "year": "2026" },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "chapter", "displayName": "Chapitre 1", "sourceFileName": "c1.md" }
        ]
    }))
    .unwrap();

    let bytes = build_docx(&payload, None, false, &|_, _| {}).unwrap();
    let document = read_entry(&bytes, "word/document.xml");

    // Numérotation + nom réunis.
    assert!(
        document.contains("Chapitre 1 - Le Dernier Soir"),
        "titre concaténé :\n{document}"
    );
    // Saut de page **après** le titre (isolement strict de la page de chapitre).
    let title = paragraph_chunk(&document, "Chapitre 1 - Le Dernier Soir");
    assert!(
        title.contains("<w:br w:type=\"page\""),
        "saut de page obligatoire après le titre :\n{title}"
    );
    // Le premier paragraphe du corps suit le titre.
    assert!(document.contains("La flamme se courbe."), "corps présent");

    let _ = fs::remove_dir_all(&dir);
}



/// Les pages structurelles (titres de section) portent « première page
/// différente » avec un en-tête **`first` vide** : l'en-tête courant n'apparaît
/// pas sur la page de titre, uniquement sur les pages suivantes contenant le corps.
#[test]
fn structural_sections_blank_header_on_first_page() {
    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "Nunael", "authorName": "Danoe", "year": "2026" },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "organization": [
            { "type": "act", "displayName": "Acte 1", "children": [
                { "type": "chapter", "displayName": "Chapitre 1" }
            ] }
        ]
    }))
    .unwrap();
    let bytes = build_docx(&payload, None, false, &|_, _| {}).unwrap();
    let document = read_entry(&bytes, "word/document.xml");

    let sections = document.matches("<w:sectPr").count();
    assert!(sections >= 3, "sections structurelles présentes : {sections}");
    assert_eq!(
        document.matches("<w:titlePg").count(),
        sections,
        "« première page différente » sur chaque section"
    );
    assert_eq!(
        document
            .matches("<w:headerReference w:type=\"first\" r:id=\"rIdRunningHeaderEmpty\"")
            .count(),
        sections,
        "en-tête `first` VIDE sur chaque section (page de titre sans en-tête)"
    );
}


#[test]
fn running_header_uses_eight_points_with_bottom_rule() {
    let payload = sample_payload();
    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");

    let headers: String = entry_names(&bytes)
        .iter()
        .filter(|name| name.starts_with("word/header"))
        .map(|name| read_entry(&bytes, name))
        .collect();
    // Taille de police de l'en-tête : 8 pt = 16 demi-points (`w:sz w:val="16"`).
    assert!(
        headers.contains("<w:sz w:val=\"16\""),
        "en-tête de chapitre à 8 pt"
    );
    // Trait de séparation horizontal discret sous le texte.
    assert!(
        headers.contains("<w:pBdr>") && headers.contains("<w:bottom"),
        "bordure inférieure de l'en-tête"
    );
    // **Aucun encadrement** : seuls le bas est bordé (ni haut, ni gauche, ni droite).
    assert!(!headers.contains("<w:top"), "pas de bordure supérieure");
    assert!(!headers.contains("<w:left"), "pas de bordure gauche");
    assert!(!headers.contains("<w:right"), "pas de bordure droite");
}

#[test]
fn running_header_uses_organisation_chapter_name() {
    use std::fs;
    use std::io::Write;

    let dir = std::env::temp_dir().join(format!("danoe-studio-test-hdr-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("dossier temporaire");

    // Le titre `#` de la source diffère du nom saisi dans « Organisation ».
    let mut file = fs::File::create(dir.join("c1.md")).expect("création c1.md");
    writeln!(file, "# Titre issu de la source").unwrap();
    writeln!(file).unwrap();
    writeln!(file, "Corps du chapitre.").unwrap();

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "Test", "authorName": "Auteur", "year": "2026" },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "act", "displayName": "Acte 1", "children": [
                { "type": "chapter", "displayName": "Chapitre Un", "sourceFileName": "c1.md" }
            ] }
        ]
    }))
    .expect("payload valide");

    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");
    let headers: String = entry_names(&bytes)
        .iter()
        .filter(|name| name.starts_with("word/header"))
        .map(|name| read_entry(&bytes, name))
        .collect();

    // L'en-tête reprend le **nom du chapitre saisi dans Organisation**…
    assert!(
        headers.contains("Chapitre Un"),
        "en-tête = nom du chapitre (Organisation)"
    );
    // …et **pas** le titre `#` du fichier source.
    assert!(
        !headers.contains("Titre issu de la source"),
        "le titre du fichier source ne sert pas d'en-tête"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn build_docx_finds_image_in_subfolder() {
    use std::fs;

    let dir = std::env::temp_dir().join(format!("danoe-studio-test-subimg-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let sub = dir.join("illustrations");
    fs::create_dir_all(&sub).expect("sous-dossier");
    // Image rangée dans un sous-dossier, référencée par son seul nom de fichier.
    fs::write(sub.join("planche.png"), sample_png(4, 4)).expect("écriture planche.png");

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "Test", "authorName": "Auteur", "year": "2026" },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "options": { "imageColorMode": "color" },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "image", "displayName": "Planche", "sourceFileName": "planche.png" }
        ]
    }))
    .expect("payload valide");

    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");
    let media = entry_names(&bytes)
        .into_iter()
        .find(|name| name.starts_with("word/media/") && !name.ends_with('/'));
    assert!(
        media.is_some(),
        "image résolue depuis un sous-dossier et embarquée dans le .docx"
    );

    let _ = fs::remove_dir_all(&dir);
}

/// Reproduction du scénario utilisateur : tome sur la page de titre, image
/// placée après l'acte, en-tête = nom du chapitre (et non de l'acte).
/// Texte visible d'une partie XML (concaténation des `<w:t>`).
fn header_text(xml: &str) -> String {
    let mut out = String::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<w:t") {
        let after = &rest[start..];
        let Some(gt) = after.find('>') else { break };
        let content = &after[gt + 1..];
        let Some(end) = content.find("</w:t>") else {
            break;
        };
        out.push_str(&content[..end]);
        rest = &content[end + "</w:t>".len()..];
    }
    out
}

/// En-têtes **réellement affichés** (pages impaires) par section, dans l'ordre
/// du document : résout `headerReference[default]` → partie XML → texte.
fn displayed_headers(bytes: &[u8]) -> Vec<String> {
    let document = read_entry(bytes, "word/document.xml");
    let rels = read_entry(bytes, "word/_rels/document.xml.rels");
    let mut rid_target: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for chunk in rels.split("<Relationship ").skip(1) {
        let grab = |key: &str| -> Option<String> {
            let i = chunk.find(key)? + key.len();
            let s = &chunk[i..];
            Some(s[..s.find('"')?].to_string())
        };
        if let (Some(id), Some(target)) = (grab("Id=\""), grab("Target=\"")) {
            rid_target.insert(id, target);
        }
    }
    let mut out = Vec::new();
    let mut rest = document.as_str();
    while let Some(start) = rest.find("<w:sectPr") {
        let after = &rest[start..];
        let end = after
            .find("</w:sectPr>")
            .map(|e| e + "</w:sectPr>".len())
            .unwrap_or(after.len());
        let sect = &after[..end];
        let mut text = String::new();
        for piece in sect.split("<w:headerReference ").skip(1) {
            let stop = piece.find("/>").unwrap_or(piece.len());
            let tag = &piece[..stop];
            let grab = |key: &str| -> String {
                match tag.find(key) {
                    Some(i) => {
                        let s = &tag[i + key.len()..];
                        s[..s.find('"').unwrap_or(0)].to_string()
                    }
                    None => String::new(),
                }
            };
            if grab("w:type=\"") != "default" {
                continue;
            }
            if let Some(target) = rid_target.get(&grab("r:id=\"")) {
                text = header_text(&read_entry(bytes, &format!("word/{target}")));
            }
        }
        out.push(text);
        rest = &after[end..];
    }
    out
}

/// Pour **chaque section** (dans l'ordre du document), renvoie une paire
/// `(texte du contenu, en-tête par défaut réellement affiché)`.
///
/// Le contenu d'une section = tout le texte (`<w:t>`) situé **avant** son
/// `<w:sectPr>` (c'est-à-dire les pages de la section). L'en-tête est résolu en
/// suivant la référence `w:type="default"` → relation → partie XML. Ce helper
/// vérifie donc l'association **contenu ↔ en-tête** (et non le seul ordre des
/// libellés) : c'est ce qui permet de détecter tout décalage d'un cran.
fn sections_content_and_headers(bytes: &[u8]) -> Vec<(String, String)> {
    let document = read_entry(bytes, "word/document.xml");
    let rels = read_entry(bytes, "word/_rels/document.xml.rels");
    let mut rid_target: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for chunk in rels.split("<Relationship ").skip(1) {
        let grab = |key: &str| -> Option<String> {
            let i = chunk.find(key)? + key.len();
            let s = &chunk[i..];
            Some(s[..s.find('"')?].to_string())
        };
        if let (Some(id), Some(target)) = (grab("Id=\""), grab("Target=\"")) {
            rid_target.insert(id, target);
        }
    }

    let mut out = Vec::new();
    let mut content = String::new();
    let mut rest = document.as_str();
    while let Some(start) = rest.find("<w:sectPr") {
        // Texte de la section courante (avant le saut de section).
        content.push_str(&header_text(&rest[..start]));
        let after = &rest[start..];
        let end = after
            .find("</w:sectPr>")
            .map(|e| e + "</w:sectPr>".len())
            .unwrap_or(after.len());
        let sect = &after[..end];
        let mut header = String::new();
        for piece in sect.split("<w:headerReference ").skip(1) {
            let stop = piece.find("/>").unwrap_or(piece.len());
            let tag = &piece[..stop];
            let grab = |key: &str| -> String {
                match tag.find(key) {
                    Some(i) => {
                        let s = &tag[i + key.len()..];
                        s[..s.find('"').unwrap_or(0)].to_string()
                    }
                    None => String::new(),
                }
            };
            if grab("w:type=\"") != "default" {
                continue;
            }
            if let Some(target) = rid_target.get(&grab("r:id=\"")) {
                header = header_text(&read_entry(bytes, &format!("word/{target}")));
            }
        }
        out.push((std::mem::take(&mut content), header));
        rest = &after[end..];
    }
    // Texte éventuel après le dernier secteur (aucun : le dernier `sectPr` est
    // le tout dernier élément du corps).
    out
}

/// Chaque section du corps doit afficher **son propre** en-tête (le nom du
/// chapitre saisi dans Organisation), le tome doit apparaître sur la page de
/// titre, et l'image doit être intercalée dans le flux.
#[test]
fn build_docx_sections_display_their_own_header_and_place_image() {
    use std::fs;
    use std::io::Write;

    let dir = std::env::temp_dir().join(format!("danoe-dbg-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("Acte I.png"), sample_png(8, 6)).unwrap();
    let mut ch = fs::File::create(dir.join("ch1.md")).unwrap();
    writeln!(ch, "# Titre de la source").unwrap();
    writeln!(ch).unwrap();
    writeln!(ch, "Texte du chapitre.").unwrap();

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "HILDEGARDE", "authorName": "Danoe", "year": "2026", "volumeNumber": "2" },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "options": { "imageColorMode": "color" },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "act", "displayName": "Acte 1", "children": [
                { "type": "image", "displayName": "Acte I", "sourceFileName": "Acte I.png" },
                { "type": "chapter", "displayName": "Chapitre premier", "sourceFileName": "ch1.md" }
            ] }
        ]
    }))
    .unwrap();

    let bytes = build_docx(&payload, None, false, &|_, _| {}).unwrap();
    let out = std::env::temp_dir().join("danoe-debug.docx");
    fs::write(&out, &bytes).unwrap();
    println!("DOCX écrit : {}", out.display());

    let document = read_entry(&bytes, "word/document.xml");
    let rels = read_entry(&bytes, "word/_rels/document.xml.rels");
    let mut rid_target: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for chunk in rels.split("<Relationship ").skip(1) {
        let grab = |key: &str| -> Option<String> {
            let i = chunk.find(key)? + key.len();
            let s = &chunk[i..];
            Some(s[..s.find('"')?].to_string())
        };
        if let (Some(id), Some(target)) = (grab("Id=\""), grab("Target=\"")) {
            rid_target.insert(id, target);
        }
    }
    let mut displayed: Vec<String> = Vec::new();
    let mut rest = document.as_str();
    while let Some(start) = rest.find("<w:sectPr") {
        let after = &rest[start..];
        let end = after
            .find("</w:sectPr>")
            .map(|e| e + "</w:sectPr>".len())
            .unwrap_or(after.len());
        let sect = &after[..end];
        for piece in sect.split("<w:headerReference ").skip(1) {
            let stop = piece.find("/>").unwrap_or(piece.len());
            let tag = &piece[..stop];
            let grab = |key: &str| -> String {
                match tag.find(key) {
                    Some(i) => {
                        let s = &tag[i + key.len()..];
                        s[..s.find('"').unwrap_or(0)].to_string()
                    }
                    None => String::new(),
                }
            };
            // L'en-tête des pages impaires = référence `default`.
            if grab("w:type=\"") != "default" {
                continue;
            }
            if let Some(target) = rid_target.get(&grab("r:id=\"")) {
                displayed.push(header_text(&read_entry(&bytes, &format!("word/{target}"))));
            }
        }
        rest = &after[end..];
    }

    // 3) **Chaque section affiche SON propre en-tête** (et non celui de la
    //    section précédente). Sections : [liminaires, sommaire, acte, image,
    //    chapitre, colophon].
    assert_eq!(
        displayed.iter().filter(|h| h.as_str() == "Acte 1").count(),
        1,
        "le nom de l'acte n'apparaît que dans l'en-tête de l'acte"
    );
    let non_empty: Vec<&str> = displayed
        .iter()
        .map(String::as_str)
        .filter(|header| !header.is_empty())
        .collect();
    assert_eq!(
        non_empty,
        vec!["Acte 1", "Chapitre premier"],
        "chaque section affiche SON en-tête (acte puis chapitre)"
    );

    // 4) Image intercalée **entre l'acte et le chapitre** dans le flux.
    let act_at = document.find("ACTE 1").expect("titre d'acte");
    let img_at = document.find("w:drawing").expect("image dans le corps");
    let ch_at = document
        .find("Titre de la source")
        .expect("titre de chapitre");
    assert!(
        act_at < img_at && img_at < ch_at,
        "image placée entre l'acte et le chapitre"
    );
    assert!(
        document.contains("a:blip"),
        "image référencée dans le corps"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn build_docx_title_tome_image_and_chapter_header() {
    use std::fs;
    use std::io::Write;

    let dir = std::env::temp_dir().join(format!("danoe-studio-test-full-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("dossier temporaire");
    // Image nommée avec un espace, placée **après** l'acte dans l'organisation.
    fs::write(dir.join("Acte I.png"), sample_png(8, 6)).expect("écriture Acte I.png");

    let mut chapter = fs::File::create(dir.join("ch1.md")).expect("création ch1.md");
    writeln!(chapter, "# Titre de la source").unwrap();
    writeln!(chapter).unwrap();
    writeln!(chapter, "Texte du chapitre.").unwrap();

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": {
            "bookTitle": "HILDEGARDE", "authorName": "Danoë", "year": "2026",
            "volumeNumber": "2"
        },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "options": { "imageColorMode": "color" },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "act", "displayName": "Acte 1", "children": [
                { "type": "image", "displayName": "Acte I", "sourceFileName": "Acte I.png" },
                { "type": "chapter", "displayName": "Chapitre premier", "sourceFileName": "ch1.md" }
            ] }
        ]
    }))
    .expect("payload valide");

    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");
    let document = read_entry(&bytes, "word/document.xml");

    // 1) Tome sous le titre du livre.
    assert!(
        document.contains("Tome 2"),
        "tome affiché sur la page de titre"
    );

    // 2) Image intégrée **et réellement référencée** dans le corps.
    let media = entry_names(&bytes)
        .into_iter()
        .find(|name| name.starts_with("word/media/") && !name.ends_with('/'));
    assert!(media.is_some(), "illustration intégrée au .docx");
    assert!(
        document.contains("w:drawing") && document.contains("a:blip"),
        "illustration référencée dans le corps du document"
    );
    // Relation image déclarée (sinon Word n'affiche rien).
    let rels = read_entry(&bytes, "word/_rels/document.xml.rels");
    assert!(
        rels.contains("relationships/image"),
        "relation de type image déclarée"
    );

    // 3) En-tête = nom du **chapitre** (Organisation), pas le nom de l'acte.
    let headers: Vec<String> = entry_names(&bytes)
        .into_iter()
        .filter(|name| name.starts_with("word/header"))
        .map(|name| read_entry(&bytes, &name))
        .collect();
    let chapter_header = headers
        .iter()
        .find(|header| header.contains("Chapitre premier"))
        .expect("un en-tête porte le nom du chapitre");
    assert!(
        !chapter_header.contains("Acte 1"),
        "l'en-tête du chapitre ne reprend pas le nom de l'acte"
    );

    let _ = fs::remove_dir_all(&dir);
}

/// **Test de non-régression strict** — scénario exact
/// « Acte → Image → Chapitre 1 → Chapitre 2 ». Vérifie de façon purement
/// programmatique :
///
/// 1. la section qui **contient le corps du Chapitre 1** porte l'en-tête
///    « Chapitre 1 » (ni « Chapitre 2 », ni le nom de l'acte) : cela interdit
///    tout décalage d'un cran (off-by-one) dans l'association
///    chapitre ↔ section ;
/// 2. l'illustration produit un **dessin OOXML valide** (`w:drawing` +
///    `a:blip` + media + relation), intercalé entre l'acte et le chapitre 1.
#[test]
fn build_docx_strict_chapter_header_and_image_placement() {
    use std::fs;

    let dir = std::env::temp_dir().join(format!("danoe-strict-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("planche.png"), sample_png(16, 12)).unwrap();
    fs::write(
        dir.join("c1.md"),
        "# Titre du chapitre 1\n\nCorps UNIQUE du chapitre 1.\n",
    )
    .unwrap();
    fs::write(
        dir.join("c2.md"),
        "# Titre du chapitre 2\n\nCorps UNIQUE du chapitre 2.\n",
    )
    .unwrap();

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "HILDEGARDE", "authorName": "Danoë", "year": "2026" },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "act", "displayName": "Acte 1", "children": [
                { "type": "image", "displayName": "Planche", "sourceFileName": "planche.png" },
                { "type": "chapter", "displayName": "Chapitre 1", "sourceFileName": "c1.md" },
                { "type": "chapter", "displayName": "Chapitre 2", "sourceFileName": "c2.md" }
            ] }
        ]
    }))
    .expect("payload valide");

    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");

    // Artefact de diagnostic : écrit un vrai `.docx` inspectable.
    let _ = std::fs::write(std::env::temp_dir().join("danoe-strict.docx"), &bytes);

    // --- 1) Association STRICTE contenu ↔ en-tête, section par section. ---
    let sections = sections_content_and_headers(&bytes);
    let find = |needle: &str| -> String {
        sections
            .iter()
            .find(|(content, _)| content.contains(needle))
            .map(|(_, header)| header.clone())
            .unwrap_or_else(|| panic!("section contenant « {needle} » introuvable"))
    };

    // La section qui porte le corps du Chapitre 1 affiche SON PROPRE en-tête.
    assert_eq!(
        find("Corps UNIQUE du chapitre 1"),
        "Chapitre 1",
        "l'en-tête de la section du Chapitre 1 doit être « Chapitre 1 »"
    );
    // …et surtout PAS celui du chapitre suivant (décalage d'un cran)…
    assert_ne!(
        find("Corps UNIQUE du chapitre 1"),
        "Chapitre 2",
        "aucun décalage : le Chapitre 1 ne doit pas afficher « Chapitre 2 »"
    );
    // …ni le nom de l'acte.
    assert_ne!(
        find("Corps UNIQUE du chapitre 1"),
        "Acte 1",
        "le chapitre ne doit pas reprendre l'en-tête de l'acte"
    );
    assert_eq!(
        find("Corps UNIQUE du chapitre 2"),
        "Chapitre 2",
        "l'en-tête du Chapitre 2 doit être « Chapitre 2 »"
    );
    assert_eq!(find("ACTE 1"), "Acte 1", "l'acte porte son propre en-tête");

    // --- 2) L'illustration produit un dessin OOXML valide et embarqué. ---
    let document = read_entry(&bytes, "word/document.xml");
    assert!(
        document.contains("<w:drawing>"),
        "balise de dessin <w:drawing> présente dans le flux"
    );
    assert!(
        document.contains("<a:blip"),
        "référence d'image <a:blip> présente dans le flux"
    );
    let media = entry_names(&bytes)
        .into_iter()
        .find(|name| name.starts_with("word/media/") && !name.ends_with('/'));
    assert!(media.is_some(), "image embarquée dans word/media/");
    let rels = read_entry(&bytes, "word/_rels/document.xml.rels");
    assert!(
        rels.contains("relationships/image"),
        "relation de type image déclarée"
    );

    // --- 3) Page d'illustration intercalée entre l'acte et le Chapitre 1. ---
    let act_at = document.find("ACTE 1").expect("titre d'acte");
    let img_at = document.find("w:drawing").expect("dessin dans le corps");
    let ch_at = document
        .find("Titre du chapitre 1")
        .expect("titre du Chapitre 1");
    assert!(
        act_at < img_at && img_at < ch_at,
        "l'illustration est intercalée entre l'acte et le chapitre 1"
    );

    let _ = fs::remove_dir_all(&dir);
}

/// La partie d'en-tête courante doit être un **XML bien formé** : en particulier
/// un **espace sépare** `w:rFonts` de son premier attribut (`w:ascii`) — sans lui,
/// `<w:rFontsw:ascii=…>` est du XML invalide et Word ignore l'en-tête (retour à
/// un héritage, ex. le nom de l'Acte).
#[test]
fn running_header_xml_is_well_formed_and_carries_chapter_name() {
    let xml = running_header_xml("Chapitre 1", "Garamond");
    assert!(
        xml.contains("<w:rFonts w:ascii=\"Garamond\""),
        "espace présent entre « w:rFonts » et « w:ascii » : {xml}"
    );
    assert!(
        !xml.contains("<w:rFontsw:ascii"),
        "attribut collé (XML invalide) détecté : {xml}"
    );
    assert!(xml.contains("standalone=\"yes\"?>"), "déclaration XML");
    assert!(xml.contains("<w:hdr "), "racine w:hdr");
    assert!(xml.contains("Chapitre 1"), "texte de l'en-tête présent");
}

/// **Validation bas niveau du binaire final** : chaque partie XML du `.docx`
/// produit doit être bien formée. On vérifie en particulier qu'aucun **nom
/// d'élément** ne contient plus d'un `:` (un attribut collé au nom — ex.
/// `<w:rFontsw:ascii=…>` — rendrait le XML invalide et Word ignorerait la
/// partie concernée, en-tête compris).
#[test]
fn generated_docx_xml_parts_have_no_glued_attributes() {
    let payload = sample_payload();
    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");

    let mut checked = 0;
    for name in entry_names(&bytes) {
        if !(name.ends_with(".xml") || name.ends_with(".rels")) {
            continue;
        }
        let content = read_entry(&bytes, &name);
        for tag in content.split('<').skip(1) {
            // Le nom d'élément s'arrête au premier blanc, '>' ou '/'.
            let end = tag
                .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
                .unwrap_or(tag.len());
            let element = &tag[..end];
            assert!(
                element.matches(':').count() <= 1,
                "nom d'élément malformé « {element} » (attribut collé ?) dans {name}"
            );
            // Un attribut doit toujours être précédé d'un blanc : on rejette
            // tout « /> » ou « = » immédiatement collé après le nom d'élément.
            if element.len() < tag.len() {
                let next = tag.as_bytes()[element.len()] as char;
                assert!(
                    next.is_whitespace() || next == '>' || next == '/',
                    "caractère « {next} » collé au nom d'élément « {element} » dans {name}"
                );
            }
        }
        checked += 1;
    }
    assert!(checked >= 5, "plusieurs parties XML analysées ({checked})");
}

/// Un nœud `image` **sans fichier lié** ne doit plus être ignoré en silence : le
/// mandat impose une **erreur explicite** (jamais de « page ignorée »). Le message
/// doit être actionnable pour permettre la correction dans l'onglet Organisation.
#[test]
fn build_docx_reports_unlinked_image_node_as_explicit_error() {
    use std::fs;

    let dir = std::env::temp_dir().join(format!("danoe-unlinked-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("c1.md"), "Corps.\n").unwrap();

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "Livre", "authorName": "A", "year": "2026" },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "act", "displayName": "Acte 1", "children": [
                { "type": "image", "sourceFileName": "" },
                { "type": "chapter", "displayName": "Chapitre 1", "sourceFileName": "c1.md" }
            ] }
        ]
    }))
    .unwrap();

    let error = build_docx(&payload, None, false, &|_, _| {})
        .expect_err("un nœud image sans source doit produire une erreur explicite");
    assert!(
        error.contains("sans fichier source lié"),
        "message actionnable attendu, obtenu : {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

/// **Preuve bout-en-bout** de l'intégration d'une illustration : depuis un chemin
/// complexe (sous-dossier, casse différente) jusqu'à la **validation binaire** des
/// octets (identiques au fichier source) dans l'archive `.docx`, en passant par la
/// balise `<w:drawing>`, l'ancre `<a:blip>`/`r:embed` et la copie `word/media/`.
#[test]
fn build_docx_embeds_linked_image_end_to_end() {
    use std::fs;

    let dir = std::env::temp_dir().join(format!("danoe-embed-{}", std::process::id()));
    let sub = dir.join("Images");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&sub).unwrap();
    // Illustration rangée dans un sous-dossier, casse différente de la cible
    // déclarée (`Images/planche.png` → `Images/Planche.PNG`).
    let source_png = sample_png(40, 30);
    fs::write(sub.join("Planche.PNG"), &source_png).unwrap();
    fs::write(dir.join("c1.md"), "# Chapitre\n\nCorps.\n").unwrap();

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "Livre", "authorName": "A", "year": "2026" },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "image", "displayName": "Planche", "sourceFileName": "Images/planche.png" },
            { "type": "chapter", "displayName": "Chapitre 1", "sourceFileName": "c1.md" }
        ]
    }))
    .unwrap();

    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("export avec illustration");

    // 1) Dessin OOXML présent, avec ancre de bitmap (`a:blip`) et relation d'image.
    let document = read_entry(&bytes, "word/document.xml");
    assert!(
        document.contains("<w:drawing"),
        "dessin présent dans le corps"
    );
    assert!(
        document.contains("<a:blip"),
        "référence de bitmap (`a:blip`) présente"
    );
    assert!(document.contains("r:embed="), "relation d'image référencée");

    // 2) Copie binaire dans `word/media/` et **validation** stricte des octets.
    let media: Vec<String> = entry_names(&bytes)
        .into_iter()
        .filter(|n| n.starts_with("word/media/") && !n.ends_with('/'))
        .collect();
    assert_eq!(media.len(), 1, "exactement une image embarquée : {media:?}");
    let embedded = read_entry_bytes(&bytes, &media[0]);
    // …les octets embarqués sont un **PNG** valide (signature)…
    assert_eq!(
        &embedded[0..8],
        &[137, 80, 78, 71, 13, 10, 26, 10],
        "signature PNG des octets embarqués"
    );
    // …et **identiques** au fichier source (aucune altération du binaire).
    assert_eq!(embedded, source_png, "octets embarqués == fichier source");

    let _ = fs::remove_dir_all(&dir);
}

/// Vérifie la correspondance section → en-tête sur une structure **réaliste** :
/// deux actes et leurs chapitres.
#[test]
fn build_docx_headers_follow_chapters_across_acts() {
    use std::fs;
    use std::io::Write;

    let dir = std::env::temp_dir().join(format!("danoe-hdr-multi-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    for name in ["a.md", "b.md", "c.md"] {
        let mut file = fs::File::create(dir.join(name)).unwrap();
        writeln!(file, "Texte du {name}.").unwrap();
    }

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "Livre", "authorName": "A", "year": "2026" },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "act", "displayName": "Acte 1", "children": [
                { "type": "chapter", "displayName": "Chapitre 1", "sourceFileName": "a.md" },
                { "type": "chapter", "displayName": "Chapitre 2", "sourceFileName": "b.md" }
            ] },
            { "type": "act", "displayName": "Acte 2", "children": [
                { "type": "chapter", "displayName": "Chapitre 3", "sourceFileName": "c.md" }
            ] }
        ]
    }))
    .unwrap();

    let bytes = build_docx(&payload, None, false, &|_, _| {}).unwrap();
    let headers = displayed_headers(&bytes);
    let non_empty: Vec<&str> = headers
        .iter()
        .map(String::as_str)
        .filter(|header| !header.is_empty())
        .collect();
    assert_eq!(
        non_empty,
        vec!["Acte 1", "Chapitre 1", "Chapitre 2", "Acte 2", "Chapitre 3",],
        "chaque section affiche son propre en-tête, dans l'ordre du livre"
    );

    // **Vérification STRICTE contenu ↔ en-tête** (et non le seul ordre des
    // libellés, qui masque un éventuel décalage) : la section qui contient le
    // corps d'un chapitre doit afficher SON propre en-tête.
    let sections = sections_content_and_headers(&bytes);
    let header_for = |needle: &str| -> String {
        sections
            .iter()
            .find(|(content, _)| content.contains(needle))
            .map(|(_, header)| header.clone())
            .unwrap_or_else(|| panic!("section contenant « {needle} » introuvable"))
    };
    assert_eq!(header_for("Texte du a.md"), "Chapitre 1");
    assert_eq!(header_for("Texte du b.md"), "Chapitre 2");
    assert_eq!(header_for("Texte du c.md"), "Chapitre 3");
    assert_eq!(header_for("ACTE 1"), "Acte 1");
    assert_eq!(header_for("ACTE 2"), "Acte 2");

    let _ = fs::remove_dir_all(&dir);
}

/// La résolution du chemin d'une illustration est **robuste** : elle trouve le
/// fichier qu'il soit fourni en **absolu**, en **relatif** (y compris dans un
/// sous-dossier) ou par **nom seul** (repli récursif insensible à la casse), et
/// renvoie `None` (donc une erreur journalisée) quand il est réellement absent.
#[test]
fn resolve_source_path_handles_absolute_relative_and_case() {
    use std::fs;

    let dir = std::env::temp_dir().join(format!("danoe-resolve-top-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let sub = dir.join("Images");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("Planche.PNG"), sample_png(4, 4)).unwrap();

    let base = std::path::Path::new(&dir);
    // 2) Chemin relatif avec sous-dossier.
    let rel = resolve_source_path(base, "Images/Planche.PNG").expect("chemin relatif");
    assert!(rel.is_file(), "résolu en relatif");
    // 3) Repli récursif insensible à la casse (nom seul).
    let ci = resolve_source_path(base, "planche.png").expect("repli casse/sous-dossier");
    assert!(ci.is_file(), "résolu par nom seul");
    // 1) Chemin absolu injecté directement.
    let absolute = sub.join("Planche.PNG");
    let abs = resolve_source_path(base, &absolute.to_string_lossy()).expect("chemin absolu");
    assert!(abs.is_file(), "résolu en absolu");
    // Fichier réellement absent → None (cause journalisée par `load_scaled_pic`).
    assert!(
        resolve_source_path(base, "introuvable.png").is_none(),
        "fichier absent → None"
    );

    let _ = fs::remove_dir_all(&dir);
}

/// Une illustration **présente mais non décodable** ne doit plus être ignorée en
/// silence : l'export échoue avec une **erreur explicite** (et **jamais** via un
/// panic interne de `docx-rs`).
#[test]
fn build_docx_reports_undecodable_image_as_explicit_error() {
    use std::fs;

    let dir = std::env::temp_dir().join(format!("danoe-badimg-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("corrompu.png"), b"ceci n'est pas une image").unwrap();

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "Livre", "authorName": "A", "year": "2026" },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "image", "displayName": "Corrompue", "sourceFileName": "corrompu.png" }
        ]
    }))
    .unwrap();

    // Aucun panic : une erreur explicite et localisée (fichier + cause) est renvoyée.
    let error = build_docx(&payload, None, false, &|_, _| {})
        .expect_err("image non décodable → erreur explicite");
    assert!(
        error.contains("non décodable") && error.contains("corrompu.png"),
        "message explicite (fichier + cause) attendu, obtenu : {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Racine export.rs : nom de fichier et surcharge de couverture
// ---------------------------------------------------------------------------

#[test]
fn default_file_name_slugs_and_keeps_alphanumeric() {
    let payload = sample_payload();
    assert_eq!(default_file_name(&payload), "Nunael.docx");
}

#[test]
fn default_file_name_sanitises_non_alphanumeric() {
    let mut payload = sample_payload();
    payload.metadata.book_title = "Le Souffle".to_string();
    // Chaque caractère non alphanumérique devient `_` (ici l'espace).
    assert_eq!(default_file_name(&payload), "Le_Souffle.docx");
    // Un titre entièrement composé de séparateurs retombe sur le nom par défaut.
    payload.metadata.book_title = "   ".to_string();
    assert_eq!(default_file_name(&payload), "manuscrit.docx");
}

#[test]
fn read_custom_cover_matches_supported_extensions_only() {
    use std::fs;

    let dir = std::env::temp_dir().join(format!("danoe-cover-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    // Aucun fichier → None.
    assert!(read_custom_cover(&dir).is_none());

    // Surcharge reconnue, insensible à la casse.
    fs::write(dir.join("Couverture.PNG"), TINY_PNG).unwrap();
    assert_eq!(read_custom_cover(&dir).as_deref(), Some(TINY_PNG));

    // Extension non supportée → ignorée (repli silencieux voulu, jamais d'erreur).
    fs::remove_file(dir.join("Couverture.PNG")).unwrap();
    fs::write(dir.join("couverture.xyz"), b"ceci n'est pas une couverture").unwrap();
    assert!(read_custom_cover(&dir).is_none());

    let _ = fs::remove_dir_all(&dir);
}

/// **Régression — lettrine** : après un Titre 1, la lettrine (`w:dropCap`) doit être
/// portée par le **tout premier** paragraphe de corps, et **jamais** par le second.
/// Le premier paragraphe est ici assez long pour couvrir la lettrine (≥ 2 lignes) :
/// c'est une condition structurelle pour qu'elle ne déborde pas (voir
/// `drop_cap_skipped_when_first_paragraph_shorter_than_the_cap`).
#[test]
fn drop_cap_lands_on_first_paragraph_after_heading() {
    use std::fs;

    let dir = std::env::temp_dir().join(format!("danoe-dropcap-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("c1.md"),
        "# Chapitre\n\nElle ne s'éteint pas, pas encore, mais la flamme vacille déjà sous le souffle froid de la nuit qui descend sur le vieux scriptorium et fait danser les ombres.\n\nLa flamme se courbe.\n",
    )
    .unwrap();

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "Livre", "authorName": "A", "year": "2026" },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "chapter", "displayName": "Chapitre", "sourceFileName": "c1.md" }
        ]
    }))
    .unwrap();

    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");
    let document = read_entry(&bytes, "word/document.xml");

    // « scriptorium » est unique au 1er paragraphe, « courbe » au 2e (la 1re lettre
    // du premier paragraphe est détachée en run séparé).
    let first = paragraph_chunk(&document, "scriptorium");
    let second = paragraph_chunk(&document, "courbe");
    assert!(
        first.contains("w:dropCap=\"drop\""),
        "lettrine attendue sur le 1er paragraphe : {first}"
    );
    assert!(
        !second.contains("w:dropCap=\"drop\""),
        "aucune lettrine sur le 2e paragraphe : {second}"
    );

    let _ = fs::remove_dir_all(&dir);
}

/// **Régression — lettrine vs sous-titre (cause racine)** : un sous-titre `##`
/// placé entre le titre `#` et le corps ne doit **pas** consommer ni supprimer la
/// lettrine. Elle reste portée par le **premier** paragraphe (assez long ici).
#[test]
fn drop_cap_survives_subtitle_between_heading_and_body() {
    use std::fs;

    let dir = std::env::temp_dir().join(format!("danoe-dropcap-sub-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("c1.md"),
        "# Chapitre\n\n## Sous-titre\n\nElle ne s'éteint pas, pas encore, mais la flamme vacille déjà sous le souffle froid de la nuit qui descend sur le vieux scriptorium et fait danser les ombres.\n\nLa flamme se courbe.\n",
    )
    .unwrap();

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "Livre", "authorName": "A", "year": "2026" },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "chapter", "displayName": "Chapitre", "sourceFileName": "c1.md" }
        ]
    }))
    .unwrap();

    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");
    let document = read_entry(&bytes, "word/document.xml");

    let first = paragraph_chunk(&document, "scriptorium");
    let second = paragraph_chunk(&document, "courbe");
    assert!(
        first.contains("w:dropCap=\"drop\""),
        "la lettrine doit survivre au sous-titre et rester sur le 1er paragraphe : {first}"
    );
    assert!(
        !second.contains("w:dropCap=\"drop\""),
        "aucune lettrine sur le 2e paragraphe : {second}"
    );

    let _ = fs::remove_dir_all(&dir);
}

/// **Garde structurelle anti-débordement** : si le premier paragraphe est **plus
/// court que la hauteur de la lettrine** (une seule ligne), le cadre déborderait
/// sous le texte et Word ferait remonter le paragraphe suivant autour de lui. On
/// vérifie donc qu'**aucune** lettrine n'est émise dans ce cas — même si un
/// paragraphe ultérieur est long : la mise en page ne peut pas être cassée.
#[test]
fn drop_cap_skipped_when_first_paragraph_shorter_than_the_cap() {
    use std::fs;

    let dir = std::env::temp_dir().join(format!("danoe-dropcap-short-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("c1.md"),
        "# Chapitre\n\nLa flamme.\n\nElle ne s'éteint pas, pas encore, mais la flamme vacille déjà sous le souffle froid de la nuit qui descend sur le vieux scriptorium et fait danser les ombres.\n",
    )
    .unwrap();

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "Livre", "authorName": "A", "year": "2026" },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "chapter", "displayName": "Chapitre", "sourceFileName": "c1.md" }
        ]
    }))
    .unwrap();

    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");
    let document = read_entry(&bytes, "word/document.xml");

    // Aucun cadre de lettrine (donc aucun débordement possible), même sur le
    // paragraphe long qui suit.
    assert!(
        !document.contains("w:dropCap"),
        "aucune lettrine ne doit être émise quand le 1er paragraphe est trop court"
    );
    assert!(
        !document.contains("<w:framePr"),
        "aucun cadre de lettrine ne doit être émis (pas de débordement possible)"
    );

    let _ = fs::remove_dir_all(&dir);
}

/// **Glossaire** : le terme défini doit apparaître **en gras** au début de chaque
/// entrée (dérivé du libellé de note quand la source n'utilise pas « Terme : »),
/// et un **séparateur de scène** (`* * *`) doit aérer les groupes de chapitres —
/// **sans** apparaître avant le premier groupe.
#[test]
fn glossary_shows_bold_term_and_separates_chapters() {
    use std::fs;

    let dir = std::env::temp_dir().join(format!("danoe-gloss-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    // Définitions « nues » (sans préfixe « Terme : ») : le terme doit être dérivé
    // du libellé `[^libellé]`.
    fs::write(
        dir.join("c1.md"),
        "Dehors, la Nahe[^nahe] gronde sous la pluie.\n\n[^nahe]: Rivière allemande qui rejoint le Rhin.\n",
    )
    .unwrap();
    fs::write(
        dir.join("c2.md"),
        "La muraille[^mur] tient bon.\n\n[^mur]: Enceinte de pierre élevée.\n",
    )
    .unwrap();

    let payload: ProjectPayload = serde_json::from_value(serde_json::json!({
        "metadata": { "bookTitle": "Livre", "authorName": "A", "year": "2026" },
        "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond" },
        "directories": { "sources": dir.to_string_lossy() },
        "organization": [
            { "type": "chapter", "displayName": "Chapitre 1", "sourceFileName": "c1.md" },
            { "type": "chapter", "displayName": "Chapitre 2", "sourceFileName": "c2.md" },
            { "type": "special", "role": "glossary", "displayName": "Glossaire" }
        ]
    }))
    .unwrap();

    let bytes = build_docx(&payload, None, false, &|_, _| {}).expect("génération");
    let document = read_entry(&bytes, "word/document.xml");

    // 1) Terme en gras + « : » en début d'entrée (n° en exposant, puis « Nahe : … »).
    assert!(document.contains("Nahe :"), "terme « Nahe : » présent");
    assert!(document.contains("Mur :"), "terme « Mur : » présent");
    let nahe = paragraph_chunk(&document, "Nahe :");
    assert!(nahe.contains("<w:b"), "le terme est en gras : {nahe}");
    // La définition suit le terme (elle reste rendue).
    assert!(document.contains("Rivière allemande"), "définition rendue");

    // 2) Un seul séparateur de scène (2 groupes) et il est **entre** les groupes.
    assert_eq!(
        document.matches("* * *").count(),
        1,
        "un seul séparateur pour deux groupes"
    );
    let separator = document.find("* * *").expect("séparateur présent");
    let first_definition = document.find("Rivière allemande").expect("1re définition");
    let second_term = document.find("Mur :").expect("2e terme");
    assert!(
        first_definition < separator && separator < second_term,
        "le séparateur est inséré entre les groupes, jamais avant le premier"
    );

    let _ = fs::remove_dir_all(&dir);
}
