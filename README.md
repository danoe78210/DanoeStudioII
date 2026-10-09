# Danoë Studio — Machine à romans

Atelier d'édition littéraire « livre ouvert » : construction de la structure d'un roman
(Actes / Chapitres / Images / Pages spéciales), paramétrage typographique KDP, et export
**Word (`.docx`)** / PDF / EPUB prêts pour Amazon KDP.

## Stack

- **Frontend** : React 19 + TypeScript + Vite, Tailwind CSS v4, Framer Motion, `@dnd-kit`.
- **Backend de bureau** : **Tauri v2** + **Rust** (`src-tauri/`) — moteur d'export (`.docx`
  via `docx-rs`, `serde`/`serde_json` pour le payload de projet).

L'interface fonctionne **dans le navigateur** (SPA, export simulé) **et** en **application
de bureau** (Tauri, export réel). Les utilitaires détectent `window.__TAURI__` et retombent
proprement — aucun code spécifique au navigateur n'est requis.

## Prérequis

- **Node.js** ≥ 20 et **npm**.
- **Rust** (stable) + **cargo** — pour compiler le backend de bureau.

## Développement

```bash
npm install            # dépendances frontend (inclut @tauri-apps/cli)

# SPA dans le navigateur (Vite)
npm run dev            # http://localhost:5173  — export simulé

# Application de bureau (Vite + fenêtre Tauri)
npm run tauri dev
```

## Build

```bash
npm run build          # build du frontend dans dist/
npm run tauri build    # installeur / exécutable de bureau (dist + binaire Rust)
```

## Structure

```
DanoeStudioII/
├─ src/                 # Frontend (voir SPECIFICATIONS_DANOE_STUDIO.md)
└─ src-tauri/           # Backend Tauri v2 + Rust
   ├─ Cargo.toml        # tauri, serde, serde_json, docx-rs
   ├─ tauri.conf.json   # withGlobalTauri, bundle.resources = ["assets/**/*"]
   ├─ .taurignore       # fichiers exclus du watcher `tauri dev` (target/, assets/)
   ├─ capabilities/     # permissions de la fenêtre
   ├─ src/lib.rs        # commandes `generate_docx` / `generate_epub`
   ├─ src/epub.rs       # scaffold d'export EPUB 3 (epub-builder)
   ├─ icons/            # icônes d'application (générées par `tauri icon`)
   └─ assets/           # ressources empaquetées — voir assets/README.md
```

### Ressource « couverture cuir »

Déposer `Couverture_Cuir_01.jpg` dans `src-tauri/assets/` (non versionné) — voir
`src-tauri/assets/README.md`.

## Statut de l'export

- **Word (`.docx`)** — **fonctionnel (phases 1–5)** : fichier **intérieur KDP** (couverture **exclue** —
  produite via un PDF séparé), pages liminaires (faux-titre, copyright, page de titre, avec **saga**),
  **table des matières** native, **corps du texte** (Markdown étendu : Titres 1/2, séparateurs de scène,
  citations, listes natives, **images**, gras/italique), **lettrines** (drop caps), **glossaire** et
  notes en exposant, **achevé d'imprimer**, en-têtes dynamiques par chapitre, marges **miroir** KDP
  (`<w:mirrorMargins/>`) et **gouttière** calculée (`w:gutter`). La commande Tauri `generate_docx` ouvre
  une boîte de dialogue d'enregistrement et émet un événement **`export-progress`** (étapes +
  pourcentage) ; l'UI affiche une **jauge de progression** et une notification de fin.
  > Déposer `Couverture_Cuir_01.jpg` dans `src-tauri/assets/` (ou un `couverture.*` dans « Mes sources »).
- **Ebook (`.epub`)** — **scaffold fonctionnel** : page de titre + un XHTML par chapitre/page
  (Markdown → HTML), métadonnées et sommaire, via `epub-builder` (images/notes à venir).
- **PDF** — à venir (le bouton renvoie un message explicite).
- Hors application empaquetée (navigateur), l'export reste **simulé**.

## Documentation

- `SPECIFICATIONS_DANOE_STUDIO.md` — spécification d'implémentation (état du dépôt).
- `specifications_roman_kdp.md` — cahier des charges fonctionnel KDP.