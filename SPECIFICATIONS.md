# Danoë Studio — Spécifications

> Machine à romans : environnement d'écriture littéraire rétro produisant des livres
> prêts à publier (Amazon KDP broché & ebook).

---

## 1. Vision du projet

Danoë Studio est un logiciel d'édition littéraire immersif : un **grand registre ancien
ouvert** posé sur un **bureau sombre rétro années 50**, encadré d'instruments de contrôle
(barre de progression « rétro-industrielle », journal/console).

- **Univers visuel** : parchemin, encre, cuir, cuivre, ors — typographie sérif littéraire.
- **Périmètre** : transformer des chapitres (Markdown / texte / Word) et des illustrations
  en un livre conforme aux normes d'impression **KDP** (format de coupe, gouttière, marges).
- **Principe fondateur** : les réglages du studio décrivent **uniquement le manuscrit
  exporté** (Word / PDF / EPUB) et ses métadonnées ; ils ne modifient **jamais** l'interface.
- **Source unique de vérité KDP** : `src-tauri/src/kdp.rs` (formats, gouttière, marges, fond perdu).

---

## 2. Architecture technique

**Stack** : Tauri **v2** (Rust) + React 19 + TypeScript + Tailwind CSS 4 + Lucide Icons.

### 2.1 Backend — `src-tauri/` (Rust)

| Module | Rôle |
|---|---|
| `kdp.rs` | Spécifications physiques KDP (source unique de vérité) |
| `project.rs` | Désérialisation du payload projet + estimation paginée |
| `markdown.rs` | Parsing AST Markdown, notes, glossaire, frontmatter |
| `export.rs` / `export/*` | Moteur **Word** (`docx-rs`) : sections, images, glossaire |
| `pdf/*` | Moteur **PDF Typst** : `generator.rs` (AST → balisage), `compiler.rs` (`typst::World`) |
| `epub.rs` | Moteur **EPUB 3** (`epub-builder`) |
| `commands.rs` | Commandes Tauri : exports, `render_pdf`, lecture/écriture de chapitres, `finalize_exit` |
| `corrector/*` | Correcteur linguistique (client, options, dictionnaires) |

**Principes transverses :**

- **Découpage sécurisé** : jamais de coupe au milieu d'un mot ni d'un caractère UTF-8.
- **Persistance atomique** : écriture `<fichier>.tmp` dans le même dossier puis `rename`
  (projet, dictionnaires, options) → aucune corruption en cas d'arrêt brutal.
- **Tolérance Zéro** : toute erreur (I/O, source introuvable, syntaxe Typst, HTTP) est
  propagée en `Result::Err` jusqu'au frontend — aucun échec silencieux.
- **Chemins maîtrisés** : les sources sont résolues via le **projet persistant**
  (`app_data_dir/project.danoe`) ; aucun chemin arbitraire n'est exposé.

### 2.2 Frontend — `src/`

| Élément | Rôle |
|---|---|
| `App.tsx` | État global, orchestration des onglets, export, fermeture animée |
| `components/Layout.tsx` | Registre double page, cinématique 3D de fermeture, glow |
| `components/MenuFlipBook.tsx` | Livre virtuel des onglets (`react-pageflip`) |
| `components/SettingsNavigator.tsx` + `settings/*` | Onglet Réglages (niveaux + pages) |
| `components/PreviewFlipbook.tsx` | Aperçu interactif du PDF Typst (`pdf.js` + `react-pageflip`) |
| `components/CorrectorView.tsx` / `CorrectorSidebar.tsx` / `ChapterEditor.tsx` | Module Correcteur |
| `utils/*` | Ponts Tauri, projet, sources, correcteur, arrêt propre |

### 2.3 Pont Tauri

`src/utils/tauri.ts` détecte `window.__TAURI__` (`withGlobalTauri: true`) :
`invokeCommand` (commandes) et `listenEvent` (événements `export-progress`,
`app-close-requested`). Hors Tauri (SPA Vite), repli explicite (export simulé, `window.close`).

---

## 3. Moteurs d'export KDP (parité tri-format)

| Format | Moteur | Spécificités |
|---|---|---|
| **Word `.docx`** | `docx-rs` | Styles natifs, sections isolées (`oddPage`), images EMU, TOC, glossaire |
| **PDF prêt-à-imprimer** | `typst` + `typst-pdf` | Marges miroir + gouttière (`kdp.rs`), en-tête courant, folio, `#outline` |
| **EPUB 3** | `epub-builder` | CSS fluide, `break-before`, notes de fin, illustrations pleine page |

**Règles d'aération communes** : titre de niveau 1 = page entière centrée H+V ; niveau 2 =
saut de page systématique ; illustration = page dédiée centrée. Vérifié par les tests de
parité (`test_layout_parity_page_breaks_across_formats`).

### 3.1 Aperçu interactif (Flipbook)

1. `commands::render_pdf(payload)` → octets **en mémoire** via `tauri::ipc::Response`
   (aucune écriture disque) ; `src/utils/pdfPreview.ts` normalise en `Uint8Array`.
2. `pdf.js` rasterise chaque page sur un `<canvas>` ; `react-pageflip` feuillette.
3. **Lazy rendering** : 4 pages initiales, buffer ±2 pages autour de `current`, canvas
   créés en 1×1 (mémoire minimale) ; rouleau de navigation (`turnToPage`).

---

## 4. Module Correcteur linguistique

### 4.1 Moteur LanguageTool

- Client `reqwest` asynchrone → `https://api.languagetool.org/v2/check` (`language`, `text`,
  `level=picky`, `disabledRules`).
- **Découpage parallèle** : tronçons ≤ **8 000 octets**, jamais coupés au milieu d'un mot
  (priorité `\n\n` → `\n` → espace → frontière UTF-8). Requêtes lancées via `futures::join_all`.
- **Réindexation en caractères** : `chunk_start += chunk.chars().count()`, puis
  `entry.offset += chunk_start` → offsets strictement alignés sur le texte source.
- **Tolérance aux erreurs partielles** : un tronçon en échec (HTTP 500 / timeout) est
  signalé en console sans interrompre les autres ; échec **total** → `Result::Err`.
- `disabledRules` nettoyés (`trim`, jeu de caractères `[A-Za-z0-9_.-]`, vides éliminés).

### 4.2 Masquage non destructif du Markdown

Avant envoi, la syntaxe est remplacée par des **espaces de même longueur en caractères**
(offsets préservés) : frontmatter **YAML Obsidian** de tête, appels de notes `[^label]`,
code inline `` `…` ``, texte barré `~~…~~`.

### 4.3 Double dictionnaire (persistance atomique)

| Fichier (`app_data_dir`) | Contenu | Commandes |
|---|---|---|
| `ignored_words.json` | Mots ignorés | `list_ignored_words`, `update_ignored_words` |
| `places.json` | Toponymes / noms propres | `list_places`, `add_place` |
| `corrector_options.json` | Options d'analyse | `get_corrector_options`, `set_corrector_options` |

`analyze_chapter` écarte automatiquement : les mots ignorés, les alertes **orthographiques**
visant un toponyme connu, et les règles désactivées.

### 4.4 Interface

- **Volet latéral épuré** : compteur de suggestions + « Relancer » ; cartes d'erreur avec
  message humain, contexte (mot fautif surligné), suggestions lisibles
  (`formatReplacement` : `Supprimer`, `⎵ 1 espace`, `⎵ Espace insécable`), actions
  **Localiser**, **Lieu**, **Règle**, **Ignorer**.
- **Journal des opérations** (accordéon sous l'en-tête) : corrections validées de la session
  (`ancien terme` barré → terme corrigé, horodatage `HH:MM`) ; mention italique si vide.
- **Animation rétro** : la carte validée s'évanouit (`opacity-0`, `-translate-x-2`, `scale-95`,
  repli `max-h-0`, teinte `bg-copper/10`, 300–380 ms) avant retrait.
- **Éditeur in-situ** (`ChapterEditor`) : soulignements ondulés (rouge = orthographe,
  ambré = style, bleu = grammaire) calés sur `offset`/`length` ; clic sur une faute →
  défilement de la sidebar vers la carte ; survol d'une carte → surbrillance dans le texte.
- **« Localiser »** : défilement du textarea (`scrollTo`, centrage vertical), synchronisation
  du calque miroir, `setSelectionRange(offset, offset + length)`, surlignage rouge vif
  (`bg-red-500/35 ring-2 ring-red-500/70`).
- **Sauvegarde synchronisée** : chaque remplacement écrit le chapitre via
  `write_chapter_file` (atomique) ; indicateur discret « Sauvegardé » (~2 s).

### 4.5 Page « Réglages » — Correcteur linguistique

- **Région / dialecte** : `fr`, `fr-FR`, `fr-BE`, `fr-CA`, `fr-CH` (pastilles).
- **Mode Pointilleux** : interrupteur (style, typographie, sémantique avancée).
- **Règles désactivées** : liste humanisée + bouton **Réactiver**.
- **Infobulles pédagogiques** (icône `?`) : langage clair, orienté relecture littéraire,
  sans jargon technique.

---

## 5. Cinématique de fermeture (3D + Glow)

1. **Interception native** : `WindowEvent::CloseRequested` → `api.prevent_close()` +
   émission `app-close-requested` (aucune fermeture brutale).
2. **Frontend** : `isClosing = true` (bouton « Quitter » ou croix native).
3. **Backdrop glow** : calque radial cuivré `rgba(198, 134, 66, 0.4)` + `blur-3xl`
   (opacité 0 → 1).
4. **Seam glow** : bande lumineuse sur l'axe central (`0 0 25px 6px rgba(217, 119, 6, 0.6)`).
5. **Rabattement 3D** : perspective **1400 px** ; volet droit `transform-origin: left`,
   `rotateY(-180deg)` en **1000 ms** `cubic-bezier(0.22, 1, 0.36, 1)` ; `backface-visibility`
   révèle la couverture cuir + logo doré.
6. **Clôture** (~1150 ms) : `finalize_exit` → `window.destroy()` (libération ordonnée des
   threads WebView2 → évite l'erreur Win32 1412) puis `app.exit(0)`.
7. **Purge garantie** : cache volatil nettoyé sur `RunEvent::Exit` (jamais les données
   utilisateur ni le profil WebView).

---

## 6. Persistance & sécurité

- `app_data_dir` (roaming) : `project.danoe`, `ignored_words.json`, `places.json`,
  `corrector_options.json` — **jamais** purgés.
- `app_cache_dir/volatile` : purge synchrone à la fermeture.
- Écritures **atomiques** systématiques (`.tmp` + `rename`).

---

## 7. Qualité & validation

```bash
cargo check                        # backend
cargo clippy --all-targets         # 0 warning attendu
cargo test --lib                   # parité tri-format, pipeline PDF, correcteur
npx tsc --noEmit -p tsconfig.app.json
npx eslint .
npm run build
```

**Politique** : aucune régression tolérée (0 erreur TypeScript, 0 warning Clippy,
tests de parité verts).
