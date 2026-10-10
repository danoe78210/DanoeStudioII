# Danoë Studio — Cover Studio v1.1.0

Spécification du sous-module **Atelier de Couverture** (« full wrap » KDP Print : plat 4 | tranche | plat 1).
Complète `SPECIFICATIONS_DANOE_STUDIO.md` (§24).

---

## 1. Vue d'ensemble

Le module produit le **fichier de couverture complet** (300 DPI) d'un roman broché KDP à partir de
deux images (plat 1 / Recto et plat 4 / Verso) et de la pagination du bloc intérieur.

| Couche | Technologie | Fichiers |
|---|---|---|
| Moteur géométrique | Rust | `src-tauri/src/cover/geometry.rs` |
| Inspecteur d'images | Rust (`image`) | `src-tauri/src/cover/inspector.rs` |
| Extraction couleur de tranche | Rust (`image`) | `src-tauri/src/cover/color_extractor.rs` |
| Balisage + compilation PDF | Rust (`typst`, `typst-pdf`) | `src-tauri/src/cover/typst_generator.rs` |
| Interface | React + Tailwind v4 | `src/components/cover/*` |

---

## 2. Onglet & navigation

- Onglet latéral **`couverture`**, libellé « Couverture », icône `BookMarked`, teinte `#a35829`.
- Placé **entre `organisation` et `correcteur`** (`LeftPage.tsx`, `MENU_ORDER` dans `App.tsx`).
- Vue rendue par `MenuPage` → `CoverStudioView` (page de droite), état lu via `StudioContext`.

---

## 3. Géométrie KDP (`geometry.rs`)

### 3.1 Règle de parité
Le nombre de pages retenu pour la tranche est **pair** (arrondi supérieur si impair) :
`pages_used = pages + (pages % 2)`.

### 3.2 Coefficients papier (mm/page)
| Papier | id | Coefficient |
|---|---|---|
| Blanc | `white` | `0.05720` |
| Crème | `cream` | `0.06350` |
| Couleur | `color` | `0.05960` |

`spine_width_mm = pages_used × coefficient`. Les identifiants français (`blanc`, `crème`, `couleur`)
sont acceptés (insensibles à la casse/accents).

### 3.3 Seuil de texte sur tranche
Le texte de tranche n'est **éligible qu'à partir de `pages >= 80`**. Dans le cas contraire, la tranche
reste unie (ou dégradée) sans texte.

### 3.4 Dimensions & réserve code-barres
- `total_width_mm  = 3.2 + trim_width_mm + spine_width_mm + trim_width_mm + 3.2`
- `total_height_mm = 3.2 + trim_height_mm + 3.2`
- Fond perdu : **3,2 mm** sur chacun des 4 bords.
- Réserve code-barres (**50,8 × 30,5 mm**), coin inférieur droit du plat 4 :
  - `x = 3.2 + trim_width_mm − 6.4 − 50.8`
  - `y = total_height_mm − 3.2 − 6.4 − 30.5`

Les formats de coupe proviennent de `kdp::trim_size` (repli `6x9`). Sorties arrondies à 3 décimales.

### 3.5 Tests
`cover::geometry::tests` : parité, formats **5.5×8.5** et **6×9 po** pour **120** et **300** pages
(papier crème), seuil de texte, papier inconnu rejeté.

---

## 4. Inspecteur d'images (`inspector.rs`)

Lecture **uniquement de l'en-tête** (`ImageReader::into_decoder()`, aucun décodage de pixels) :
- dimensions (`width_px`, `height_px`) ;
- canal alpha (`ColorType::has_alpha()` — `Rgba8`, `Rgba16`, `La8`, `La16`) ;
- DPI effectif : `dpi_horizontal = width_px / trim_width_in`, `dpi_vertical = height_px / trim_height_in`,
  `dpi = min(des deux)` ;
- statut : `valid` (**≥ 300**), `warning` (**250–299**), `invalid` (**< 250**).

`inspect_cover_images` renvoie un `ImageInspectionReport { trimSize, requiredDpi, front, back }`.

---

## 5. Pipeline Typst & export PDF (`typst_generator.rs`)

### 5.1 Pré-traitement des images
Chaque plat est préparé dans un dossier temporaire (`std::env::temp_dir()/danoe_cover_<pid>_<nano>`) :
- image **sans** canal alpha → copiée telle quelle (extension préservée) ;
- image **avec** canal alpha → **aplatie sur fond blanc `#ffffff`** et ré-encodée en PNG.

### 5.2 Balisage (4 calques)
Conversion `pt = mm × 72 / 25.4`. Page `#set page(width, height, margin: 0pt, fill: white)`.
1. **Calque 1** — image **plat 4** (`fit: "cover"`), largeur `bleed + trim`.
2. **Calque 2** — image **plat 1** (`fit: "cover"`), largeur `trim + bleed`, à `x = bleed + trim + spine`.
3. **Calque 3** — **tranche vectorielle** (`rect`) : fond uni (`rgb("#rrggbb")`) ou dégradé
   (`gradient.linear`) ; si `pages >= 80`, texte centré tourné à **−90°**.
4. **Calque 4** — **réserve blanche de code-barres** (50,8 × 30,5 mm) au coin inférieur droit du plat 4.

### 5.3 Compilation 300 DPI
Compilation via `pdf::compile_to_pdf_with_root(source, work_dir)` (moteur **Typst embarqué**), puis
écriture du PDF à `output_pdf_path`. Le rapport (`ExportReport`) inclut `dpi` (300), `bytes`,
`pageCountUsed`, dimensions, `flattenedImages`.

### 5.4 Tests
`cover::typst_generator::tests` : génération complète vers un PDF temporaire (signature `%PDF` +
`%%EOF`), présence des 4 calques, aplatissement conditionnel, `spine_fill` (uni/dégradé/défaut).

---

## 6. Interface (`src/components/cover/`)

| Composant | Rôle |
|---|---|
| `CoverStudioView.tsx` | Conteneur d'état (persistance, dérive, modal pré-export) |
| `CoverSettingsPanel.tsx` | Page de gauche : images, paramètres KDP, typographie de tranche |
| `CoverCanvas2D.tsx` | Page de droite : planche 2D / bascule 3D + export |
| `Cover3DPreview.tsx` | Livre fermé CSS 3D orbital (plat 1, tranche, tranche des pages) |
| `CoverPreExportModal.tsx` | Checklist de conformité avant export |

- **Badges DPI** : 🟢 ≥ 300 · 🟡 250–299 · 🔴 < 250 · 🔵 alpha aplati.
- **Bascule** `[ Planche 2D ]` / `[ Modèle 3D ]` dans la barre d'outils.
- **Repères 2D** : fond perdu (rouge pointillé), pliures (bleu pointillé), zone de sécurité (vert),
  réserve code-barres (blanc).

---

## 7. Commandes Tauri (`src-tauri/src/cover/mod.rs`)

| Commande | Signature |
|---|---|
| `calculate_cover_geometry` | `(page_count, paper_type, trim_size) -> CoverGeometry` |
| `inspect_cover_images` | `(front_path, back_path, trim_size) -> ImageInspectionReport` |
| `extract_spine_color` | `(front_path, back_path) -> SpineColorSuggestion` |
| `export_kdp_cover_pdf` | `(params: CoverRenderParams, output_pdf_path) -> ExportReport` |
| `pick_cover_image` | `() -> Option<String>` (sélecteur natif) |
| `pick_cover_output_path` | `(default_name) -> Option<String>` (sélecteur natif) |

Enregistrées dans `lib.rs` (`generate_handler!`). En **Tauri v2**, les plugins `dialog`/`fs` ne sont pas
exposés sur `window.__TAURI__` : les sélecteurs natifs passent par ces commandes.

---

## 8. Persistance (`CoverStudioState`)

Champ optionnel **`cover?: CoverStudioState`** du fichier projet `.danoe` (JSON v1.0) :

```ts
interface CoverStudioState {
  frontPath: string | null;
  backPath: string | null;
  pageCount: number;
  paperType: 'white' | 'cream' | 'color';
  spineText: string;
  spineColorFront: string | null;
  spineColorBack: string | null;
  gradient: boolean;
  pageCountSnapshot: number;
  structureSignature: string;
}
```

Sérialisé par `buildProjectFile` (état `App`) ; hydraté au chargement. La géométrie et les rapports
d'inspection sont **recalculés à la volée** et ne sont jamais persistés.

---

## 9. Dialogue de pré-export (`CoverPreExportModal`)

Contrôles **automatiques** (blocage si un échec) :
- Résolution ≥ 300 DPI (Recto & Verso) ;
- Fond perdu **3,2 mm** ;
- Épaisseur de tranche (calculée) ;
- Réserve code-barres (**50,8 × 30,5 mm**) ;
- Texte de tranche (éligibilité ≥ 80 pages).

Validations **manuelles** de l'auteur : textes hors zones de massicotage, lisibilité de la tranche.

Boutons `[ Annuler ]` / `[ Confirmer & Enregistrer le PDF › ]` (→ sélecteur natif + export Typst).

---

## 10. Détection de dérive

La pagination et la **signature de structure** courantes sont comparées au snapshot enregistré lors
de la validation de la tranche (`pageCountSnapshot`, `structureSignature`). En cas d'écart, une
bannière propose **[ Recalculer la tranche ]** (met à jour le snapshot).

---

## 11. Limites & roadmap v1.2

| Sujet | Statut |
|---|---|
| Couverture **brochée** (full wrap) PDF 300 DPI | ✅ Livré |
| Couverture **rigide** (« hardcover ») | ❌ Reporté en **v1.2** |
| Formats de coupe personnalisés (saisie libre) | ❌ Non pris en charge (7 presets KDP) |

---

*Fin du document. Toute évolution doit préserver la parité des règles KDP (§3), la compatibilité du
champ `cover?` (§8) et la **Tolérance Zéro** (aucun échec silencieux).*
