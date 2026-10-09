//! Tests unitaires des utilitaires transverses (`export/helpers.rs`) :
//! échappement XML, conversions de dimensions, détection/résolution de fichiers
//! d'image et traitement colorimétrique.

use super::*;

#[test]
fn xml_escape_encodes_ampersand_then_angle_brackets() {
    assert_eq!(xml_escape("a & b <c>"), "a &amp; b &lt;c&gt;");
    // L'esperluette est échappée en premier : les entités produites ne sont pas
    // re-échappées.
    assert_eq!(xml_escape("<&>"), "&lt;&amp;&gt;");
    assert_eq!(xml_escape("texte simple"), "texte simple");
}

#[test]
fn point_and_line_conversions_enforce_lower_bounds() {
    assert_eq!(points_to_half(11.0), 22);
    assert_eq!(points_to_half(5.5), 11);
    // Bornes basses : jamais 0 (un `w:sz` nul serait invalide).
    assert_eq!(points_to_half(0.0), 2);
    assert_eq!(points_to_half(-4.0), 2);

    assert_eq!(line_value(1.15), 276);
    assert_eq!(line_value(1.0), 240);
    assert_eq!(line_value(0.0), 240);
    assert_eq!(line_value(-2.0), 240);
}

#[test]
fn current_year_is_plausible() {
    let year = current_year();
    assert!((2024..=2100).contains(&year), "année plausible : {year}");
}

#[test]
fn image_is_rasterizable_accepts_png_and_rejects_garbage() {
    // PNG valide (signature + dimensions lisibles) → accepté.
    assert!(image_is_rasterizable(TINY_PNG));
    assert!(image_is_rasterizable(&sample_png(3, 3)));
    // Octets arbitraires → refusés (aucun panic).
    assert!(!image_is_rasterizable(b"not an image"));
    assert!(!image_is_rasterizable(&[]));
}

#[test]
fn image_is_rasterizable_accepts_decodable_non_png() {
    // Sans signature PNG, la validité repose sur le décodage complet (JPEG ici).
    let mut img = image::RgbImage::new(2, 2);
    img.put_pixel(0, 0, image::Rgb([10, 20, 30]));
    let mut jpeg = std::io::Cursor::new(Vec::new());
    img.write_to(&mut jpeg, image::ImageFormat::Jpeg)
        .expect("encodage JPEG");
    assert!(image_is_rasterizable(&jpeg.into_inner()));
}

#[test]
fn find_file_ci_matches_name_insensitively_in_subfolder() {
    use std::fs;

    let dir = std::env::temp_dir().join(format!("danoe-findci-{}", std::process::id()));
    let sub = dir.join("images");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("Planche.PNG"), b"x").unwrap();

    // Cible avec chemin relatif : la comparaison porte sur le **nom de fichier**.
    let found = find_file_ci(&dir, "images/planche.png").expect("fichier trouvé");
    assert_eq!(found.file_name().unwrap().to_string_lossy(), "Planche.PNG");
    // Nom réellement absent → None.
    assert!(find_file_ci(&dir, "absent.png").is_none());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn collect_image_files_is_recursive_and_filters_extensions() {
    use std::fs;

    let dir = std::env::temp_dir().join(format!("danoe-collect-{}", std::process::id()));
    let sub = dir.join("sub");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&sub).unwrap();
    fs::write(dir.join("a.png"), b"x").unwrap();
    fs::write(sub.join("b.JPG"), b"x").unwrap();
    fs::write(dir.join("notes.txt"), b"x").unwrap();

    let mut out = Vec::new();
    collect_image_files(&dir, &mut out);
    out.sort();
    assert_eq!(out, vec!["a.png".to_string(), "b.JPG".to_string()]);

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

/// La résolution du chemin d'une illustration est **robuste** : elle trouve le
/// fichier qu'il soit fourni en **absolu**, en **relatif** (y compris dans un
/// sous-dossier) ou par **nom seul** (repli récursif insensible à la casse), et
/// renvoie `None` quand il est réellement absent (l'appelant produit alors une
/// erreur explicite).
#[test]
fn resolve_source_path_handles_absolute_relative_and_case() {
    use std::fs;

    let dir = std::env::temp_dir().join(format!("danoe-resolve-helpers-{}", std::process::id()));
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
    // Fichier réellement absent → None (erreur explicite produite par l'appelant).
    assert!(
        resolve_source_path(base, "introuvable.png").is_none(),
        "fichier absent → None"
    );

    let _ = fs::remove_dir_all(&dir);
}
