# Danoë Studio — « Machine à romans »

> **Document de spécifications complètes (fonctionnelles & techniques)**
> Objectif : permettre à n'importe quel LLM (ou développeur) de comprendre
> exactement le projet et de le poursuivre sans ambiguïté. Ce document décrit
> **l'état réellement implémenté** du dépôt `DanoeStudioII`.

---

## 0. Sommaire

1. [Vision & périmètre](#1-vision--périmètre)
2. [Glossaire & conventions](#2-glossaire--conventions)
3. [Stack technique & commandes](#3-stack-technique--commandes)
4. [Arborescence & rôle des fichiers](#4-arborescence--rôle-des-fichiers)
5. [Conventions de code (impératives)](#5-conventions-de-code-impératives)
6. [Direction artistique (design system)](#6-direction-artistique-design-system)
7. [Architecture de l'interface](#7-architecture-de-linterface)
8. [Navigation séquentielle « tour de page »](#8-navigation-séquentielle-tour-de-page)
9. [Écrans — Onglet Réglages](#9-écrans--onglet-réglages)
10. [Écrans — Onglet Informations](#10-écrans--onglet-informations)
11. [Écrans — Onglets Organisation & Export](#11-écrans--onglets-organisation--export)
12. [Modèle de données (TypeScript)](#12-modèle-de-données-typescript)
13. [Journalisation (journal des opérations)](#13-journalisation-journal-des-opérations)
14. [Intégrations OS (Tauri) & replis navigateur](#14-intégrations-os-tauri--replis-navigateur)
15. [Guide de continuation pour un LLM](#15-guide-de-continuation-pour-un-llm)
16. [État actuel, validation & dette technique](#16-état-actuel-validation--dette-technique)
17. [Sauvegarde & persistance (fichier de projet)](#17-sauvegarde--persistance-fichier-de-projet)
18. [Bac à sable & importation de fichiers](#18-bac-à-sable--importation-de-fichiers)
19. [Organisation — pagination & D&D multi-pages](#19-organisation--pagination--dd-multi-pages)
21. [Module Correcteur linguistique (LanguageTool)](#21-module-correcteur-linguistique-languagetool)
22. [Cinématique 3D de fermeture & effet Glow](#22-cinématique-3d-de-fermeture--effet-glow)
23. [Matrice de conformité et limites connues](#23-matrice-de-conformité-et-limites-connues)
24. [Atelier de Couverture (Cover Studio)](#24-atelier-de-couverture-cover-studio)

---

## 1. Vision & périmètre

**Danoë Studio** est un logiciel d'édition littéraire (« Machine à romans ») destiné à
transformer des chapitres (Markdown / texte / Word) et des illustrations en un livre
prêt à publier (**Amazon KDP** : broché et ebook). L'expérience est volontairement
immersive : **un grand registre ancien ouvert** posé sur un **bureau sombre rétro
années 50**, encadré par des instruments de contrôle (barre de progression
« rétro-industrielle » au-dessus, journal/console en dessous).

### Règle d'architecture fondamentale (à respecter absolument)

Les réglages configurés dans l'application (format de coupe, corps de texte, polices,
dossiers, métadonnées…) décrivent **uniquement le manuscrit EXPORTÉ** (Word / PDF / EPUB)
et les **métadonnées du projet**. Ils constituent un **objet de configuration global**
qui alimente les moteurs d'export **Word / PDF / EPUB**. **Ils ne modifient JAMAIS l'interface
graphique** du studio (les styles rétro, polices d'époque et la mise en page double-page
restent figés).

> Exception unique et volontaire : l'**aperçu visuel** d'une police dans la liste de
> sélection (`FontCard` / `FontRadioRow`) applique la police à **une seule balise de
> démonstration** (`style={{ fontFamily: stack }}`), pour aider au choix. Ce n'est pas
> une modification de l'UI.

---

## 2. Glossaire & conventions

| Terme | Signification |
|---|---|
| **Le Registre** | Le livre ouvert en double page (conteneur principal). |
| **Page de gauche** | Navigation (onglets/marque-pages) + page de garde (identité). |
| **Page de droite** | Zone de travail dynamique (« workspace ») selon l'onglet actif. |
| **Onglet / Marque-page** | Bouton de navigation débordant à gauche du registre. |
| **Tour de page** | Transition animée (rotateY) simulant le froissement d'une page. |
| **Sous-page** | Écran atteint par un tour de page (formulaires de réglages/infos). |
| **Journal des opérations** | La console sous le livre (logs). |
| **KDP** | Amazon Kindle Direct Publishing (normes d'impression). |
| **Trim size** | Dimensions physiques de coupe du livre. |

Conventions : langue de l'UI = **français** ; code et identifiants = **anglais** ;
nombres affichés à la française via `numberFr` (virgule décimale).

---

## 3. Stack technique & commandes

| Élément | Valeur |
|---|---|
| Framework UI | **React 19** (`react`, `react-dom` ^19.2.8) |
| Build / Dev server | **Vite 8** (`@vitejs/plugin-react`) |
| Langage | **TypeScript ~6.0** |
| Styles | **Tailwind CSS v4** (`@tailwindcss/postcss`, config CSS-first via `@theme`) |
| Animations | **framer-motion ^14** (`AnimatePresence`, `motion`) |
| Glisser-déposer | **@dnd-kit/core**, **@dnd-kit/sortable**, **@dnd-kit/utilities** |
| Icônes | **lucide-react** |
| Markdown | **react-markdown ^10** (aperçu d'écriture) |
| Point d'entrée | `index.html` → `src/main.tsx` → `App.tsx` |

**Scripts npm**
```bash
npm run dev       # serveur de développement Vite (http://localhost:5173/)
npm run build     # tsc -b && vite build
npm run lint      # eslint .
npm run preview   # prévisualisation du build
```

**Découpage du build (`vite.config.ts`) :** `chunkSizeWarningLimit: 1200` +
`rollupOptions.output.manualChunks` fragmentent les dépendances lourdes en chunks **vendor** :
`vendor-pdfjs` (`pdfjs-dist`), `vendor-motion` (`framer-motion`), `vendor-dnd` (`@dnd-kit`),
`vendor` (reste de `node_modules`). Supprime l'avertissement « chunks > 500 kB ».

**Contraintes de compilation** (`tsconfig.app.json`) — elles conditionnent le code :
- `verbatimModuleSyntax: true` → **les imports de TYPES doivent utiliser `import type`**
  (ex. `import type { LogEntry } from "../types"`, ou `import React, { type ReactNode } from "react"`).
- `noUnusedLocals` / `noUnusedParameters: true` → **aucune variable/paramètre inutilisé**
  (un import non utilisé ou un setter non lu fait échouer le build).
- `noFallthroughCasesInSwitch: true` → chaque `case` d'un `switch` doit `return`/`break`.
- `jsx: react-jsx` ; `target: es2023`, `module: esnext`, `moduleResolution: bundler`.

**Tailwind v4 :** il n'y a **pas** de `tailwind.config.js` utilisé. Le thème est défini
en CSS-first dans `src/index.css` via un bloc `@theme { … }`. Les couleurs personnalisées
génèrent des utilitaires (`bg-desk`, `text-copper`, `border-brass`, …). Les dégradés
utilisent la syntaxe v4 canonique **`bg-linear-to-r` / `bg-linear-to-b`**.

---

## 4. Arborescence & rôle des fichiers

```
DanoeStudioII/
├─ index.html                     # Shell HTML, #root, charge /src/main.tsx
├─ package.json                   # Dépendances & scripts
├─ vite.config.ts                 # Plugin React
├─ postcss.config.cjs             # @tailwindcss/postcss + autoprefixer
├─ tsconfig.app.json              # Options TS (voir §3)
├─ specifications_roman_kdp.md    # Cahier des charges KDP d'origine (référence)
├─ SPECIFICATIONS_DANOE_STUDIO.md # CE document
├─ src-tauri/                     # Backend de bureau (Tauri v2 + Rust)
│  ├─ Cargo.toml                  # Dépendances : tauri, serde, docx-rs, epub-builder, typst, typst-pdf
│  ├─ build.rs                    # Script de build `tauri-build`
│  ├─ tauri.conf.json             # Config Tauri v2 (withGlobalTauri, resources: assets/**)
│  ├─ .taurignore                 # Fichiers exclus du watcher `tauri dev` (target/, assets/)
│  ├─ capabilities/default.json   # Permissions de la fenêtre (core:default)
│  ├─ src/
│  │  ├─ main.rs                  # Point d'entrée → `danoe_studio_lib::run()`
│  │  ├─ lib.rs                   # Commandes Tauri + `run()` + progression
│  │  ├─ commands.rs              # `export_pdf`, `render_pdf`, `read/write_chapter_file`, `finalize_exit`
│  │  ├─ kdp.rs                   # Spécifications KDP (source unique de vérité)
│  │  ├─ corrector/               # Module Correcteur linguistique (LanguageTool)
│  │  │  ├─ mod.rs                # Commandes : analyze_chapter, dictionnaires, options
│  │  │  ├─ client.rs             # Client HTTP + découpage parallèle (< 8 Ko) + offsets
│  │  │  ├─ dictionary.rs         # ignored_words.json + places.json (écriture atomique)
│  │  │  └─ options.rs            # corrector_options.json (langue, picky, règles désactivées)
│  │  └─ pdf/                     # Moteur PDF Typst (generator / compiler / mod)
│  ├─ icons/                      # Icônes d'application (générées par `tauri icon`)
│  └─ assets/                     # Ressources empaquetées (Couverture_Cuir_01.jpg…)
└─ src/
   ├─ main.tsx                    # createRoot + <StrictMode><App/></StrictMode>
   ├─ index.css                   # @theme (palette, ombres, polices) + fond texturé
   ├─ types.ts                    # TOUS les types & interfaces partagés
   ├─ App.tsx                     # État global + orchestration + journalisation
   ├─ data/
   │  ├─ trimSizes.ts             # Presets KDP + helpers (label/cm)
   │  ├─ specialPages.ts          # Rôles de page spéciale (liste stricte, rôles sans titre)
   │  └─ fonts.ts                 # Polices, tailles, interlignes + helpers
   ├─ utils/
   │  ├─ directory.ts             # Dossier + import de fichiers (Tauri / repli navigateur)
   │  ├─ projectFile.ts           # Fichier projet .danoe (flat↔tree, save/load, v1.0)
   │  ├─ sources.ts               # Tri des sources par extension (chapitres / images)
   │  ├─ tauri.ts                 # Pont invoke/events Tauri + replis navigateur
   │  ├─ pdfPreview.ts            # PDF Typst en mémoire → Uint8Array (aperçu)
   │  ├─ projectApi.ts            # readChapterFile / writeChapterFile
   │  ├─ correctorApi.ts          # analyze_chapter, dictionnaires, options (types LtOptions)
   │  ├─ shutdown.ts              # Caches volatils + finalize_exit + app-close-requested
   │  └─ shell.ts                 # Ouverture de fichier par l'OS (Tauri shell)
   └─ components/
      ├─ Layout.tsx               # Registre + cinématique 3D de fermeture (glow)
      ├─ LeftPage.tsx             # Onglets + action « Quitter l'atelier » + page de garde
      ├─ ProgressBar.tsx          # Barre de progression rétro-industrielle
      ├─ LogPanel.tsx             # Journal des opérations (console)
      ├─ MenuFlipBook.tsx         # Livre virtuel des onglets (react-pageflip)
      ├─ PreviewFlipbook.tsx      # Aperçu interactif du PDF (pdf.js + canvas)
      ├─ CorrectorView.tsx        # Onglet Correcteur (orchestration + journal de session)
      ├─ CorrectorSidebar.tsx     # Volet latéral des suggestions
      ├─ ChapterEditor.tsx        # Éditeur décoré in-situ (soulignements + « Localiser »)
      ├─ SettingsNavigator.tsx    # Navigation séquentielle des Réglages (niveaux 1–9)
      ├─ SettingsView.tsx         # Niveau 1 : sommaire des réglages
      ├─ InfoView.tsx             # Onglet Informations (formulaire en sections)
      ├─ OrganizationView.tsx     # Onglet Organisation (structure DnD + pagination)
      ├─ ExportView.tsx           # Onglet Export (DOCX / PDF / EPUB + Aperçu)
      ├─ settings/
      │  ├─ controls.tsx          # PillButton, RadioRow, FontRadioRow, FontCard
      │  ├─ SettingsGeneralPage.tsx  # Niveau 2
      │  ├─ FormatPage.tsx           # Niveau 3 (formats KDP)
      │  ├─ BodyTextPage.tsx         # Niveau 4 (corps du texte)
      │  ├─ ChapterTitlePage.tsx     # Niveau 5
      │  ├─ SubtitlePage.tsx         # Niveau 6
      │  ├─ SourcesPage.tsx          # Niveau 7 (dossier unique « Mes sources »)
      │  ├─ ErrorLogPage.tsx         # Niveau 8 (journal des erreurs)
      │  └─ CorrectorPage.tsx        # Niveau 9 (correcteur linguistique)
```

> Fichiers hérités/à ignorer : `src/App.css`, `src/styles/global.css` (non importés),
> `src/components/SettingsView.tmp` (brouillon), `src/assets/*`.

---

## 5. Conventions de code (impératives)

1. **Typage strict & commentaires JSDoc** : chaque prop/composant exporté a un commentaire
   `/** … */`. Style : `export const X: React.FC<XProps> = ({ … }) => (…)`.
2. **Composants fonctionnels uniquement**, `React.FC<Props>`.
3. **Imports de types** via `import type`. **Pas d'import ni de variable inutilisés.**
4. **Palette** : utiliser en priorité les couleurs du thème (`copper`, `brass`, `gold`,
   `parchment`, `desk`, `atelier`, `verdigris`, `retro-violet`) et les tons `stone-*`.
5. **Effets rétro/bakélite** (boutons d'action) : dégradé cuivre
   `bg-linear-to-b from-copper to-[#8a5426]`, bord `border-brass/50`, reflet interne
   `shadow-[inset_0_1px_0_rgba(255,255,255,0.3),0_2px_4px_rgba(0,0,0,0.5)]`, enfoncement
   `active:translate-y-px active:shadow-[inset_0_2px_4px_rgba(0,0,0,0.6)]`.
6. **Sélection** : élément actif → `border-copper bg-copper/10` (+ pastille cuivre) ou
   `bg-copper text-amber-50` pour les pastilles.
7. **Navigation** : **jamais de modale ni de pop-up**. La navigation entre sous-pages se
   fait par `goTo(level, direction)` dans les `*Navigator`, avec animation `framer-motion`.
8. **Journalisation** : toute modification d'un réglage écrit une ligne via `addLog(...)`
   (voir §13). Niveaux : `info`, `success`, `warning`, `error`.
9. **États** : immutabilité (`{ ...previous, champ: valeur }`) ; `useCallback` pour les
   handlers exposés aux enfants.
10. **Dépendances UI** limitées à `lucide-react`, `framer-motion`, `react-markdown`.

---

## 6. Direction artistique (design system)

Thème défini en CSS-first dans `src/index.css` (bloc `@theme`).

### Couleurs du thème
| Token | Valeur | Usage |
|---|---|---|
| `--color-desk` | `#12161f` | Fond du bureau (arrière-plan global) |
| `--color-desk-light` | `#1b212e` | Variante claire du bureau |
| `--color-paper` | `#f5f5f0` | Papier neutre |
| `--color-parchment` | `#f8f3e6` | **Pages du registre** |
| `--color-parchment-dark` | `#efe6d0` | Parchemin assombri |
| `--color-gold` | `#c5a059` | Lettrine, compteur |
| `--color-gold-light` | `#e6c878` | Reflets dorés |
| `--color-brass` | `#b08d57` | **Laiton** (bordures châssis) |
| `--color-ink` | `#2b2620` | Encre (texte) |
| `--color-copper` | `#b87333` | **Cuivre** — Réglages / sélection active |
| `--color-atelier` | `#3e5d78` | Bleu atelier — Informations |
| `--color-verdigris` | `#5c8d7b` | Vert-de-gris — Écriture |
| `--color-retro-violet` | `#6b5b95` | Violet — Export |

Utilitaires générés : `bg-desk`, `bg-parchment`, `text-copper`, `border-brass`, …,
avec modificateurs d'opacité (`border-brass/40`, `bg-copper/10`).

### Ombres signature
```css
--shadow-book: 0 20px 50px rgba(0,0,0,0.6);                                   /* ombre portée du livre */
--shadow-binding-left:  inset -25px 0 25px -20px rgba(0,0,0,0.15);           /* reliure (page gauche) */
--shadow-binding-right: inset  25px 0 25px -20px rgba(0,0,0,0.15);           /* reliure (page droite) */
```
Ces ombres sont appliquées en classes arbitraires :
`shadow-[0_20px_50px_rgba(0,0,0,0.6)]` (livre),
`shadow-[inset_-25px_0_25px_-20px_rgba(0,0,0,0.15)]` (gauche),
`shadow-[inset_25px_0_25px_-20px_rgba(0,0,0,0.15)]` (droite).

### Typographie
- Serif littéraire : `Georgia, "Times New Roman", serif` (corps, titres).
- Mono « machine à écrire » : `"SFMono-Regular", Consolas, "Liberation Mono", monospace`
  (labels de registre, chemins, console, compteurs).
- Sans : `"Helvetica Neue", Arial, sans-serif` (rare).

### Fond texturé du bureau
`body` combine : un halo lumineux (radial), un vignettage sombre (radial) et de fines
stries diagonales (`repeating-linear-gradient 45deg`), en `background-attachment: fixed`.

### Motifs décoratifs récurrents
- **Châssis rétro-industriel** (ProgressBar/LogPanel) : fond `bg-linear-to-b from-[#232a38] to-[#161b26]`,
  bordure `border-brass/40`, ombre interne + externe, **4 vis** aux coins
  (`h-1.5 w-1.5 rounded-full bg-brass/70`).
- **Bouton d'action cuivre** : voir §5 point 5.
- **Puce/indicateur actif** : pastille cuivre (RadioRow) ou « œil » vert pulsant (journal).
- **Titres de section** : `font-serif uppercase tracking-[0.25em] text-stone-700` avec
  filet inférieur `border-b border-stone-400/40`.
- **Labels de champ** : `font-mono text-[10px] uppercase tracking-widest text-stone-500`.

---

## 7. Architecture de l'interface

```
<App>                                   (état global + orchestration)
 └─ <Layout progress… logs…>            (bureau sombre)
     ├─ <ProgressBar/>                  (AU-DESSUS du livre)
     ├─ <div> Le Registre (max-w-6xl, shadow portée)
     │   ├─ <section> PAGE GAUCHE  ← leftContent  = <LeftPage/>
     │   │     • onglets marque-pages (débordent à -left-36)
     │   │     • page de garde (lettrine D, titre, sous-titre)
     │   └─ <section> PAGE DROITE  ← volet double-face 3D (recto = <MenuFlipBook>, verso = couverture)
     └─ <LogPanel logs/>                (SOUS le livre — console)
```

### `Layout.tsx`
- Conteneur : `flex min-h-screen w-full flex-col items-center justify-center gap-6 bg-desk p-6`.
- Livre : `relative flex w-full max-w-6xl rounded-md shadow-[0_20px_50px_rgba(0,0,0,0.6)]`.
- Chaque page : `relative min-h-[68vh] w-1/2 rounded-l-md|rounded-r-md bg-parchment p-10`
  + ombre de reliure interne (gauche/droite).
- Props : `leftContent`, `bookPages`, `activeIndex`, `progress`, `progressLabel`, `running`,
  `logs`, `saveState`, `projectFilePath`, `onPickProjectFile`, `closing`.
- **Double-face 3D** : la page de droite est un volet (`transform-origin: left center`,
  `transform-style: preserve-3d`) à **deux calques** `backface-visibility: hidden` — recto
  `MenuFlipBook`, verso **couverture cuir** (voir §22.2).

### `LeftPage.tsx` (page de gauche)
- **Onglets** : `<nav class="absolute top-16 -left-36 flex flex-col gap-1">`, boutons
  `w-36 rounded-l-md px-4 py-3`, couleur thématique, actif → `translate-x-1 ring-1 ring-white/30`.
  | Onglet | id | Couleur | Icône lucide |
  |---|---|---|---|
  | Réglages | `reglages` | `bg-copper` | `Settings` |
  | Informations | `infos` | `bg-atelier` | `Info` |
  | Organisation | `organisation` | `bg-verdigris` | `ListTree` |
  | Couverture | `couverture` | `bg-[#a35829]` | `BookMarked` |
  | Correcteur | `correcteur` | `bg-gold` | `SpellCheck` |
  | Export | `export` | `bg-retro-violet` | `Share2` |
  | **Quitter l'atelier** | *(action)* | `bg-[#1c222e]` | `LogOut` |
- **Action permanente « Quitter l'atelier »** : placée **après un séparateur discret**
  (`mt-4 pt-2 border-t border-stone-400/20`), bouton rétro
  `bg-[#1c222e] hover:bg-[#252d3d] text-stone-300 border-l-2 border-amber-600/60` (icône `LogOut` 16 px).
  N'altère **pas** `activeMenu` ; appelle directement la prop `onExitApp()`
  (`App.tsx` → `handleQuit` : `isClosing = true` → cinématique 3D → purge du cache → `finalize_exit`).
  Disponible depuis **tout écran**.
- **Page de garde** : carré doré contenant la **lettrine « D »** (serif, ombre interne),
  titre **« ANOË STUDIO »** (majuscules, `tracking-[0.4em]`), sous-titre **« Machine à romans »**
  (italique) encadré de deux filets.

### `ProgressBar.tsx`
- Châssis rétro-industriel + 4 vis.
- Libellé (`font-mono uppercase tracking-[0.3em]`) + **compteur mécanique** à 3 chiffres
  (`String(clamped).padStart(3,"0") %`, `tabular-nums`).
- **Cadran segmenté** : piste sombre + remplissage
  `bg-linear-to-r from-brass via-gold to-gold-light`, largeur = `progress%`, `animate-pulse`
  si `running`. Graduations verticales en surimpression.
- Échelle 0 / 25 / 50 / 75 / 100.

### `LogPanel.tsx` (journal)
- Châssis sombre + 4 vis, en-tête « Journal des opérations » + témoin « Enregistrement actif »
  (pastille verte `animate-pulse`).
- Zone `h-32 overflow-y-auto font-mono text-xs`, **auto-défilement** (`scrollTop = scrollHeight`)
  au changement de `logs`.
- Préfixes/coloris par niveau : `info` « › » (`text-amber-100/80`), `success` « ✓ »
  (`text-emerald-300`), `warning` « ! » (`text-amber-300`), `error` « ✗ » (`text-red-400`).

---

## 8. Navigation séquentielle « tour de page »

Chaque grande section (Réglages, Informations) possède un **`*Navigator`** qui gère un état
interne `level` (numérique) + une `direction` (+1 avant / -1 retour) et rend la sous-page
correspondante avec un **fil d'Ariane** et une **animation de tour de page**.

**Mécanisme d'animation** (commun aux deux navigateurs) :
```tsx
const pageVariants = {
  enter:  (d: number) => ({ rotateY: d > 0 ?  75 : -75, opacity: 0 }),
  center: { rotateY: 0, opacity: 1 },
  exit:   (d: number) => ({ rotateY: d > 0 ? -75 :  75, opacity: 0 }),
};
<div className="relative flex-1 [perspective:2000px]">
  <AnimatePresence mode="wait" custom={direction}>
    <motion.div
      key={level}
      custom={direction}
      variants={pageVariants}
      initial="enter" animate="center" exit="exit"
      transition={{ duration: 0.45, ease: "easeInOut" }}
      style={{ transformOrigin: "left center", transformStyle: "preserve-3d",
               backfaceVisibility: "hidden" }}
      className="h-full"
    > … sous-page … </motion.div>
  </AnimatePresence>
</div>
```
- **`transformOrigin: "left center"`** = rotation autour de la **reliure** (bord gauche de la
  page de droite) → effet « page de parchemin qui se tourne ».
- **Fil d'Ariane** : `<nav class="mb-5 flex flex-wrap items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-stone-500">`,
  éléments séparés par `›` (`&rsaquo;`), **dernier élément en `text-copper`**.
- **Bouton retour** (dans chaque sous-page) : `← Retour` (`ChevronLeft` + libellé), appelle `onBack`.

**Règle stricte : aucune modale, aucune pop-up.** Toute navigation est un tour de page.

---

## 9. Écrans — Onglet Réglages

Onglet par défaut (`activeMenu = 'reglages'`). Rendu par **`SettingsNavigator`** (niveaux `0 → 9` ; `0` = page de garde `WelcomeCover`).

**Liste ordonnée stricte des niveaux :**
1. **Sommaire** — `SettingsView.tsx`
2. **Paramètres du livre** (Format & styles) — `SettingsGeneralPage.tsx`
3. **Format de coupe KDP** — `FormatPage.tsx`
4. **Corps du texte** — `BodyTextPage.tsx`
5. **Titres** — `ChapterTitlePage.tsx`
6. **Sous-titres** — `SubtitlePage.tsx`
7. **Mes sources** — `SourcesPage.tsx`
8. **Journal des erreurs** — `ErrorLogPage.tsx`
9. **Correcteur linguistique** — `CorrectorPage.tsx`

### Niveau 1 — Sommaire des Réglages (`SettingsView.tsx`)
Titre centré « RÉGLAGES ». Liste de **cartes-lignes** (ordre strict) :

| # | Libellé | Icône | Particularité |
|---|---|---|---|
| 1 | Paramètres du livre | `Settings` | ligne **cliquable** + sous-titre `Format de coupe : <label>` + **bouton OUVRIR** (cuivre, `SlidersHorizontal`) |
| 2 | Mes sources | `FolderOpen` | ligne cliquable + chevron `›` |
| 3 | Journal des erreurs | `ShieldAlert` | ligne cliquable + chevron |
| 4 | Correcteur linguistique | `SpellCheck` | ligne cliquable + chevron |

> Le réglage « **Dossier des exports** » est **retiré** (l'export ouvre la boîte de dialogue native à
> chaque génération — §20).

Ligne : `flex items-center gap-4 border-b border-stone-400/25 bg-stone-500/5 px-4 py-4`
(+ `cursor-pointer` si navigable). Le bouton OUVRIR est en cuivre (§5.5) et stoppe la
propagation du clic. Chaque clic → `goTo(2|7|8|9, +1)`.

### Niveau 2 — Paramètres du livre (`SettingsGeneralPage.tsx`)
Fil d'Ariane : `Réglages › Paramètres du livre`. Bouton retour.
- **Carte « Format du livre »** (`BookOpen`) : sous-titre = `trimSubtitleFor(trimSize)`
  (ex. `6 × 9 po · 15,2 × 22,9 cm`, rendu majuscule) → `onOpenFormat` (niveau 3).
- **Section « Mise en page »** — 3 cartes de navigation :
  - **Corps du texte** (`PenLine`) — sous-titre `${bodyFont} ${bodySize} pt · interligne ${lineSpacing}`
  - **Titre du chapitre** (`Heading1`) — sous-titre `${fontLabel(chapterTitleFont)} ${chapterTitleSize} pt`
  - **Sous-titres** (`Heading2`) — sous-titre `${fontLabel(subtitleFont)} ${subtitleSize} pt`
  → niveaux 4 / 5 / 6.

### Niveau 3 — Format de coupe KDP (`FormatPage.tsx`)
Liste des **7 formats KDP** (`TRIM_PRESETS`) en lignes radio (`RadioRow`) :

| id | Libellé | cm |
|---|---|---|
| `5x8` | 5 × 8 po | environ 12,7 × 20,3 cm |
| `5.25x8` | 5,25 × 8 po | environ 13,3 × 20,3 cm |
| `5.5x8.5` | 5,50 × 8,50 po | environ 14 × 21,6 cm |
| `6x9` | 6 × 9 po | environ 15,2 × 22,9 cm *(standard romans)* |
| `7x10` | 7 × 10 po | environ 17,8 × 25,4 cm |
| `8x10` | 8 × 10 po | environ 20,3 × 25,4 cm |
| `a4` | A4 | 21,0 × 29,7 cm *(unité mm)* |

Clic → `onTrimChange(id)` (met à jour `manuscriptConfig.trimSize`, journalise).

### Niveau 4 — Corps du texte (`BodyTextPage.tsx`)
- **Taille** : pastilles `9 · 10 · 10,5 · 11 · 12 · 13 · 14 pt` (11 pt recommandé, infobulle).
- **Police** : grille **2 colonnes** de `FontCard` (9 polices, aperçu `Abc 123 — Danoë`).
- **Interligne** : pastilles `1 · 1,15 · 1,25 · 1,5 · 2`.
- **Justification + Lettrine sur une même rangée** (grille 2 colonnes) :
  - Justification : `Gauche` / `Justifié`
  - Lettrine : `Oui` / `Non`

### Niveau 5 — Titres (`ChapterTitlePage.tsx`)
- **Taille** : `12 · 14 · 16 · 18 · 20 pt`.
- **Police** : option **« Identique au corps du texte »** (ligne pleine largeur, `FontRadioRow`)
  + grille 2 colonnes de 9 `FontCard`.

### Niveau 6 — Sous-titres (`SubtitlePage.tsx`)
- **Taille** : `11 · 12 · 14 · 16 pt`.
- **Police** : « Identique au corps du texte » + 9 polices (grille 2 colonnes).

### Niveau 7 — Mes sources (`SourcesPage.tsx`)
Page unique centralisant les **ressources brutes** du roman (remplace les anciens
« Dossier des chapitres » et « Dossier des images »).
- Section **« EMPLACEMENT DU DOSSIER UNIQUE (WORKSPACE) »** : ligne `flex items-center gap-4`
  → bouton cuivre **« Parcourir… »** (`FolderOpen`, `shrink-0`) + champ `flex-1` grisé en
  `font-mono` (`truncate`, `title`) ; si vide : *« Aucun dossier sélectionné »* (italique, `text-stone-400`).
  Un **seul** dossier contient l'intégralité des fichiers du projet.
- `Parcourir…` appelle `pickDirectoryWithFiles()` (voir §14) ; le **moteur de tri** répartit
  ensuite les fichiers par extension (voir §18).
- **Note d'atelier** sous le champ (petite, italique, ocre foncé `text-[#8a5a2b]`) :
  *« Chapitres : .txt, .md, .docx · Images : .png, .jpg, .jpeg, .webp, .tiff »*.
- Section **« TRAITEMENT DES IMAGES (EXPORT PDF) »** : pastilles rétro
  `Conserver les couleurs` · `Convertir en Noir & Blanc` (défaut, infobulle « recommandée »).

> **Réglage retiré** — « Dossier des exports » : l'export utilise **exclusivement** la **boîte de
> dialogue native de l'OS** (emplacement choisi fichier par fichier, §20). Aucun chemin mémorisé,
> aucune organisation en sous-dossiers (`ExportsPage.tsx` n'est plus référencé). La numérotation des
> niveaux reste donc **stricte** (aucun niveau « fantôme »).

### Niveau 8 — Journal des erreurs (`ErrorLogPage.tsx`)
- Section **« Niveau de diagnostic »** : pastilles
  `Standard` (alertes + erreurs critiques ; infobulle « recommandé ») /
  `Mécanique / Diagnostic` (toutes les opérations). Défaut = `Standard`.
- Section **« Emplacement du fichier journal »** : bouton **« Parcourir… »** + champ `flex-1`
  (mono) ; si vide : *« Dossier par défaut du système (AppData / .config) »* (italique).
- Section **« Accès rapide »** : bouton secondaire discret **« Ouvrir le fichier de bord »**
  (`FileText`, bordure grise, hover cuivre) → `openPathExternal(...)` (voir §14).

### Niveau 9 — Correcteur linguistique (`CorrectorPage.tsx`)
- Section **« Région / dialecte »** : pastilles `Français standard (fr)` · `France (fr-FR)` ·
  `Belgique (fr-BE)` · `Canada (fr-CA)` · `Suisse (fr-CH)` → `set_corrector_options`.
- Section **« Niveau d'exigence »** : interrupteur **« Mode Pointilleux »** (style, typographie
  et sémantique avancée) → `level=picky`.
- Section **« Règles désactivées »** : liste **humanisée** (jamais l'ID technique brut) +
  bouton **« Réactiver »** (retrait de `disabled_rules`, écriture atomique).
- **Infobulles pédagogiques** (`HelpCircle` → `InfoTooltip`) : langage clair, non technique,
  orienté relecture littéraire (voir §21).

### Récapitulatif fil d'Ariane (Réglages)
| Niveau | Fil d'Ariane |
|---|---|
| 1 | Réglages |
| 2 | Réglages › Paramètres du livre |
| 3 | Réglages › Paramètres du livre › Format du livre |
| 4 | Réglages › Paramètres du livre › Corps du texte |
| 5 | Réglages › Paramètres du livre › Titre du chapitre |
| 6 | Réglages › Paramètres du livre › Sous-titres |
| 7 | Réglages › Mes sources |
| 8 | Réglages › Journal des erreurs |
| 9 | Réglages › Correcteur linguistique |

---

## 10. Écrans — Onglet Informations

Rendu par **`InfoView.tsx`** — **page unique** (plus de navigation multi-niveaux).
Métadonnées du roman (`bookInfo`) : formulaire fluide en sections, validé à la perte de focus.

Titre centré « **I N F O R M A T I O N S** » : `mb-4 border-b border-stone-400/40 pb-3 text-center
font-serif text-2xl uppercase tracking-[0.25em] text-stone-700` (style repris de la page Organisation).

**Mise en page** : conteneur `mx-auto w-full max-w-2xl overflow-y-auto` ; chaque section =
`<h3>` en petites capitales cuivre + grille **2 colonnes** (`grid grid-cols-2 gap-x-8 gap-y-4`).
Les champs longs occupent **les 2 colonnes** (`col-span-2`).

**Champs « registre »** : libellé `font-mono text-[10px] uppercase tracking-widest text-stone-500`
au-dessus ; saisie `w-full bg-transparent border-b border-copper/50 font-serif text-sm`
(soulignement cuivre seul, placeholder italique). `onBlur` → `onChange` (autosauvegarde `.danoe`).

| Section | Champ affiché | `field` | Largeur | Placeholder |
|---|---|---|---|---|
| **Identité de l'œuvre** | Titre du livre | `title` | pleine | `ex. Nunael` |
| | Sous-titre | `subtitle` | pleine | — |
| | Nom de la série / Saga | `sagaTitle` | demi | `ex. Les Schattenjägers` |
| | Numéro de tome | `volumeNumber` | demi | `ex. 1` (`type="number"`) |
| **Auteurs et contributeurs** | Nom de l'auteur principal | `author` | demi | `ex. Danoë` |
| | Traducteur ou illustrateur | `contributor` | demi | `Optionnel` |
| **Édition et mentions légales** | Nom de l'éditeur | `publisher` | demi | `ex. Auto-édition ou nom de la maison` |
| | Numéro ISBN | `isbn` | demi | `ex. 978-2-XXXXXX-XX-X` |
| | Date de parution | `year` | demi | `ex. Octobre 2026` |
| | Lieu d'impression | `printLocation` | demi | `ex. France` |
| **Éléments annexes** | Autres œuvres | `otherBooks` | pleine (`textarea`) | `ex. Titre 1, Titre 2, Titre 3` |

**Rôles de compilation** :
- Sections A/B → **Page de Titre** ; Section C → **Page de Copyright** (verso de la page de
  titre) & **Achevé d'imprimer** (le lieu d'impression est requis pour la conformité légale).
- Section D `otherBooks` : **si renseigné**, le moteur génère la page **« Du même auteur »**
  (page paire n°2) juste avant la page de titre ; si vide, la page est ignorée.

> Champs **hérités conservés pour la persistance** (non affichés) : `copyright`, `website`.

---

## 11. Écrans — Onglets Organisation & Export

### Onglet Organisation (`OrganizationView.tsx`)
Onglet `organisation` (**remplace** l'ancien onglet « Écriture »). Titre « ORGANISATION ».
Construit la structure du roman (« chemin de fer ») avec **glisser-déposer** (`@dnd-kit`).

**Barre d'outils** (boutons rétro cuivre, **grille 4 colonnes** sur une seule ligne) :
`Ajouter un Acte`, `Ajouter un Chapitre`, `Ajouter une Image`, `Page spéciale`.
Les trois derniers sont **désactivés** (et une note d'aide ocre s'affiche) si le dossier
unique « Mes sources » n'est pas configuré (§9 niveau 7).

**Liste de structure** (`DndContext` + `SortableContext` vertical) : lignes déplaçables via
une poignée `GripVertical` (à gauche) ; ombre portée pendant le déplacement
(`shadow-[0_6px_16px_rgba(0,0,0,0.25)]`). Indentation (`ml-8`) des chapitres, images et
pages spéciales appartenant au dernier Acte rencontré.

**Séparateurs filigranés** (repères très discrets `StructureSeparator`, `text-stone-400/60`) :
en tête de liste (page 1) *« --- Début de l'ouvrage (pages liminaires auto-générées) --- »* ;
en pied de liste (dernière page) *« --- Fin de l'ouvrage --- »*.

> **Placement du Sommaire / TOC** : la table des matières est générée **en tête du manuscrit**,
> juste après les liminaires et les dédicaces/épigraphes d'ouverture — donc **avant le premier
> chapitre** (`export/body.rs` : « Table des matières : après les liminaires et les dédicaces/
> épigraphes d'ouverture »). Elle n'est **jamais** placée en fin d'ouvrage.

| Type | Contenu de la ligne |
|---|---|
| `act` | badge **[ACTE]** + input du nom (ex. « Acte 1 ») |
| `chapter` | icône `FileText` + `<select>` (fichiers texte de « Mes sources ») + `Importer` + input « Nom dans le livre : » |
| `image` | icône `ImageIcon` + `<select>` (fichiers image de « Mes sources ») + `Importer` |
| `special` | **page spéciale** — icône `Bookmark` + `<select>` de **Rôle** + `<select>` de fichier texte + `Importer` ; Ligne 2 = « Titre dans le livre : » (voir ci-dessous) |

Chaque ligne possède un **bouton corbeille** (`Trash2`). Le renommage est validé à la perte
de focus. État : `structure: StructureItem[]` (tableau **plat ordonné**) dans `App`.

### Page spéciale — pages modulaires (`type: 'special'`)
Objet générique (bloc glisser-déposer **sur 2 lignes**) évitant de multiplier les boutons
d'ajout. Répartit les 15 éléments de structure standard : les pages **automatiques**
(faux-titre, titre, copyright, table des matières) sont générées par le moteur et n'apparaissent
**pas** ici ; les pages **modulaires** (rédigées par l'auteur) sont gérées via ce type.
- **Ligne 1** : identifiant visuel (`Bookmark`), **sélecteur de Rôle** (liste **stricte**,
  `src/data/specialPages.ts` → `SPECIAL_PAGE_ROLES`) : `Dédicace / Épigraphe / Prologue /
  Épilogue / Note de l'auteur / Remerciements / Glossaire` ; puis sélecteur de fichier (textes
  de « Mes sources ») + `Importer` + corbeille.
- **Condition « Glossaire »** : si le rôle est **`glossary`** (`SOURCELESS_SPECIAL_ROLES`), le
  **sélecteur de fichier ET le bouton `Importer` sont masqués** (le glossaire est généré par le
  moteur). Restent visibles : icône, sélecteur de rôle, `Titre dans le livre` et corbeille.
- **Ligne 2** : `Titre dans le livre :` + input. **Comportement intelligent** : si le rôle
  n'admet pas de titre (`TITLELESS_SPECIAL_ROLES` = `Dédicace`, `Épigraphe`), le champ est
  **grisé/désactivé** (_« Sans objet pour ce rôle »_) ; sinon il reste actif (ex. `Prologue`).
- Le `role` est porté par `StructureItem.role` (sérialisé dans le fichier projet). Le
  **moteur de compilation** (Rust) s'en sert pour le formatage : dédicace italique alignée à
  droite au tiers inférieur d'une page impaire, épigraphe centrée entre guillemets,
  prologue/chapitre/épilogue commençant sur une **page impaire** ; une image placée avant le
  prologue appartient au _front matter_.

### Glossaire & notes de bas de page (règles moteur — Rust)
Le moteur de compilation (hors de ce dépôt) **scanne la structure** pour détecter la présence
d'une **Page spéciale de rôle `glossary`**, puis traite les notes Markdown de type renvoi
`[^x]` et bloc de définition `[^x]: …` :

| Cas | Condition | Traitement |
|---|---|---|
| **A — pas de Glossaire** | aucune page `glossary` | **Suppression pure** de tous les renvois `[^x]` du corps ; les blocs `[^x]: …` sont **ignorés** (absents de l'export). |
| **B — Glossaire présent** | ≥ 1 page `glossary` | Renvois `[^x]` **conservés**, numérotation **strictement incrémentale et réinitialisée à chaque chapitre** (chap. 1 → 1…3 ; chap. 2 → 1…). Les définitions `[^x]: …` sont **extraites**. |

- **Génération dynamique** : les définitions extraites sont agrégées et insérées **à
  l'emplacement exact** de la Page spéciale « Glossaire » dans le livre (jamais imprimées en
  bas de page des chapitres).
- **Formatage cible** : définitions **regroupées par chapitre source**, sous un **sous-titre
  automatique** (ex. « Acte 2, chapitre 7 »), chaque entrée précédée de son renvoi :
  `[^1]: **Neumes :** Les neumes sont les premiers symboles…`.
- Le renvoi conservé agit comme un **appel de note** incitant le lecteur à consulter le
  glossaire (rendu visuel discret).
- **Parité Word / PDF** : ces règles sont implémentées **à l'identique** par l'export Word
  (`export/body.rs`) et l'export PDF (`commands.rs::build_blocks` + `pdf/generator.rs` : `#super[n]` +
  glossaire final). Séparateur `* * *` entre groupes de chapitres.

### Onglet Export (`ExportView.tsx`)

Onglet `export`. Titre « EXPORT ». 3 cartes-boutons — **désactivées pendant un export**
(`isExporting`) :

| id | Icône | Description |
|---|---|---|
| `Word (.docx)` | `FileText` | Document structuré avec styles natifs et sauts de section. |
| `PDF prêt-à-imprimer` | `FileType` | Verrouillé KDP Broché (Gutter & Bleed calculés). |
| `Ebook (.epub)` | `BookOpen` | EPUB 3 fluide optimisé pour les liseuses Kindle. |

**Indicateur de chargement** : tant que `isExporting` est vrai, une ligne
`<Loader2 class="animate-spin" />` *« Génération du manuscrit en cours… »* s'affiche et les boutons
sont désactivés (l'export est une tâche lourde).

Clic → `onExport(format)` → `App.runExport(format)` — **routage par format** (voir §20) :
- **Word (.docx)** → `invokeCommand('generate_docx', { format, payload })`.
- **PDF prêt-à-imprimer** → `invokeCommand('export_pdf', { payload })` (moteur **Typst** embarqué).
- **Ebook (.epub)** → `invokeCommand('generate_epub', { payload })`.
- Le backend génère le fichier puis ouvre la **boîte de dialogue native** d'enregistrement ; toute
  erreur est **remontée** à l'UI (Tolérance Zéro, §20).
- **Navigateur** : export **simulé** (intervalle 180 ms, +10 % par pas) + `warning` au journal.

Journalisation `Démarrage de l'export — <format>` puis le message de résultat. Une **notification**
discrète (`Toast`, `src/components/Toast.tsx`) confirme le succès (ou l'erreur) et se ferme
automatiquement (~6 s).

---

## 12. Modèle de données (TypeScript)

Tout est déclaré dans **`src/types.ts`**.

### Types transverses
```ts
export type ActiveMenu = 'reglages' | 'infos' | 'organisation' | 'correcteur' | 'export';

export type LogLevel = 'info' | 'success' | 'warning' | 'error';

export interface LogEntry { id: string; time: string; level: LogLevel; message: string; }

export type TrimUnit = 'in' | 'mm';
```

### `ManuscriptLayoutConfig` — gabarit du manuscrit exporté
```ts
export interface ManuscriptLayoutConfig {
  trimSize: string;                 // '5x8'|'5.25x8'|'5.5x8.5'|'6x9'|'7x10'|'8x10'|'a4'
  bodyFont: string;                 // ex. 'Garamond'
  bodySize: number;                 // 9|10|10.5|11|12|13|14
  lineSpacing: number;              // 1|1.15|1.25|1.5|2
  textAlignment: 'left' | 'justify';
  dropCap: boolean;                 // Lettrine
  chapterTitleFont: string;         // nom de police, ou 'body' (= corps)
  chapterTitleSize: number;         // 12|14|16|18|20
  subtitleFont: string;             // nom de police, ou 'body'
  subtitleSize: number;             // 11|12|14|16
}
```

- **Transmission** : sérialisé tel quel dans `ProjectFile.layoutConfig` → désérialisé côté Rust par
  `project::LayoutConfig` (`#[serde(default)]`, `camelCase`). Le **format de coupe** (`trimSize`) est
  consommé par `kdp.rs` ; les réglages **corps/titres/sous-titres** sont **câblés dans les moteurs**
  (Word : polices/types tailles ; PDF : `#set text`, `#show heading.where(level: 1|2)`).
- **Héritage** : les valeurs `'body'` (ou vides) sont résolues vers `bodyFont` via `LayoutConfig::resolve_font`. `dropCap` pilote la lettrine (Word `w:framePr`; PDF `#dropcap`).

### Autres configurations de projet
```ts
export type ImageColorMode = 'color' | 'grayscale';
/** Sources brutes : dossier unique (workspace) + traitement PDF des images. */
export interface SourcesConfig { directory: string | null; colorMode: ImageColorMode; }

// ⚠️ Supprimé : plus de « Dossier des exports ». L'export ouvre la boîte de dialogue native de l'OS
// à chaque génération (§20) — `organizeSubfolders` / `directory` ne sont plus utilisés.

export type LogDiagnosticLevel = 'standard' | 'diagnostic';
export interface ErrorLogConfig { level: LogDiagnosticLevel; directory: string | null; }

export interface BookInfoConfig {
  // A — Identité :  title, subtitle, sagaTitle, volumeNumber
  // B — Auteurs  :  author, contributor
  // C — Édition  :  publisher, isbn, year (date de parution), printLocation
  // D — Annexe   :  otherBooks
  title: string; subtitle: string; sagaTitle: string; volumeNumber: string;
  author: string; contributor: string;
  publisher: string; isbn: string; year: string; printLocation: string;
  otherBooks: string;
  copyright: string; website: string; // héritage (persistance)
}

export type StructureItemType = 'act' | 'chapter' | 'image' | 'special';
export type SpecialPageRole =
  'dedication' | 'epigraph' | 'prologue' | 'epilogue'
  | 'authorNote' | 'acknowledgements' | 'glossary';
export interface StructureItem {
  id: string;            // identifiant unique
  type: StructureItemType;
  displayName?: string;    // nom d'acte ou libellé renommé du chapitre
  sourceFileName?: string; // nom du fichier lié (chapitres, images, pages spéciales)
  role?: SpecialPageRole;  // rôle éditorial (pages spéciales uniquement)
}
```

> La **structure** est un tableau **plat ordonné** ; la hiérarchie (appartenance à un Acte)
> est déduite de l'ordre : les éléments suivant un `act` lui appartiennent (indentés)
> jusqu'au prochain `act`.


### Valeurs par défaut (dans `App.tsx`)
```ts
DEFAULT_MANUSCRIPT = { trimSize:"6x9", bodyFont:"Garamond", bodySize:11,
  lineSpacing:1.15, textAlignment:"justify", dropCap:true,
  chapterTitleFont:"body", chapterTitleSize:16, subtitleFont:"body", subtitleSize:14 };

DEFAULT_SOURCES    = { directory:null, colorMode:"grayscale" };
DEFAULT_ERROR_LOG  = { level:"standard",     directory:null };
DEFAULT_BOOK_INFO  = { sagaTitle:"", title:"Les Schattenjägers", subtitle:"",
  author:"Danoë", year:"2026", isbn:"", publisher:"", copyright:"", website:"", otherBooks:"" };
```

### État global de `App.tsx`
```ts
const [activeMenu, setActiveMenu]           = useState<ActiveMenu>('reglages');
const [logs, setLogs]                       = useState<LogEntry[]>([…2 entrées initiales…]);
const [progress, setProgress]               = useState(0);
const [progressLabel, setProgressLabel]     = useState('Aucune opération en cours');
const [running, setRunning]                 = useState(false);
const [manuscriptConfig, setManuscriptConfig] = useState<ManuscriptLayoutConfig>(DEFAULT_MANUSCRIPT);
const [sourceConfig, setSourceConfig]       = useState<SourcesConfig>(DEFAULT_SOURCES);
const [errorLogConfig, setErrorLogConfig]   = useState<ErrorLogConfig>(DEFAULT_ERROR_LOG);
const [bookInfo, setBookInfo]               = useState<BookInfoConfig>(DEFAULT_BOOK_INFO);
const timerRef = useRef<number | null>(null);   // interval de l'export simulé
```

### Helpers de données
**`src/data/trimSizes.ts`** : `TRIM_PRESETS` (7), `findTrimPreset(id)`,
`trimLabelFor(id)` → `"6 × 9 po (environ 15,2 × 22,9 cm)"`,
`trimSubtitleFor(id)` → `"6 × 9 po · 15,2 × 22,9 cm"` (retire le préfixe « environ »).

**`src/data/fonts.ts`** : `FONT_FAMILIES` (9 : Aptos, Cinzel, Garamond, Times New Roman,
Georgia, Calibri, Arial, Book Antiqua, Cambria) avec `stack` CSS d'aperçu ;
`BODY_FONT_SIZES`, `CHAPTER_TITLE_SIZES`, `SUBTITLE_SIZES`, `LINE_SPACINGS` ;
`INHERIT_FONT = "body"`, `INHERIT_LABEL`, `fontLabel`, `resolveFont`, `numberFr`, `SAMPLE_TEXT`.

---

## 13. Journalisation (journal des opérations)

Toutes les écritures passent par `addLog(level, message)` dans `App.tsx`.
`LogEntry.id = "${Date.now()}-${logSequence++}"`, `time` = heure locale `fr-FR`.

### Messages (source de vérité)

| Déclencheur | Niveau | Message |
|---|---|---|
| Initialisation | info | `Studio initialisé. Prêt à écrire.` |
| Initialisation | success | `Projet « Les Schattenjägers » chargé.` |
| Export lancé | info | `Démarrage de l'export — <format>` |
| Export terminé | success | `Export <format> généré avec succès.` |
| Format de coupe | success | `Format de coupe configuré sur <label> (<cm>)` |
| Mise en page (corps/titres) | info | `Mise en page manuscrit mise à jour : <détails>` |
| Traitement images | info | `Images PDF configurées sur : <Couleur\|Noir & Blanc>` |
| Dossier des sources | success | `Dossier des sources mis à jour : <chemin>` |
| Niveau de diagnostic | info | `Niveau de diagnostic défini sur : <Standard\|Mécanique>` |
| Dossier du journal | success | `Emplacement du journal mis à jour : <chemin>` |
| Ouverture du .log | success / warning | `Ouverture du fichier de bord : <chemin>` / *« … indisponible (nécessite l'application empaquetée). »* |
| Champ Informations | info | `[Informations - <section>] <label> mis à jour.` |
| Structure : ajouts | info | `Structure : acte <n> ajouté.` / `Structure : chapitre <n> ajouté.` / `Structure : illustration ajoutée.` / `Structure : page spéciale ajoutée.` |
| Structure : rôle page spéciale | info | `Structure : rôle de la page spéciale défini sur « <Rôle> ».` |
| Structure : suppression | info | `Structure : élément supprimé.` |
| Structure : renommage | info | `Structure : libellé mis à jour (« <valeur> »).` |
| Structure : source liée | info | `Structure : source liée « <fichier> ».` / `Structure : source retirée.` |
| Structure : réordonnancement | info | `Structure : ordre réorganisé.` |

### Détail de `describeLayoutChange(patch, next)`
Construit `Mise en page manuscrit mise à jour : ` + une liste de segments séparés par `, ` :
- `bodyFont`/`bodySize` → `${next.bodyFont} ${numberFr(next.bodySize)}pt`
- `lineSpacing` → `interligne ${numberFr(next.lineSpacing)}`
- `textAlignment` → `justifié` ou `aligné à gauche`
- `dropCap` → `lettrine active` ou `lettrine inactive`
- `chapterTitleFont`/`chapterTitleSize` → `titre de chapitre ${resolve(font)} ${size}pt`
- `subtitleFont`/`subtitleSize` → `sous-titres ${resolve(font)} ${size}pt`

où `resolve(font)` remplace `"body"` par `next.bodyFont` (police héritée).

---

## 14. Intégrations OS (Tauri) & replis navigateur

L'application **tourne en SPA Vite** dans un navigateur **et** en application de bureau
**Tauri v2** (`src-tauri/`, config `withGlobalTauri: true`). Les utilitaires **détectent**
l'API Tauri sur `window.__TAURI__` et retombent proprement si elle est absente — le frontend
reste identique dans les deux contextes.

> **Tauri v2 — accès fichiers/dialogues exclusivement via le backend Rust.** Avec Tauri v2,
> `withGlobalTauri` n'expose **pas** les API de plugins (`dialog`, `fs`, `shell`) sur
> `window.__TAURI__`. Toutes les interactions fichiers/dialogues passent donc par des **commandes
> Rust `invokeCommand`** (`pick_directory`, `list_directory_files`, `import_file_into_folder`,
> `open_path`, `finalize_exit`, `clear_cache_and_exit`, …) — **aucun** plugin n'est appelé
> directement depuis le frontend. Les utilitaires ne sondent que l'API **cœur**
> (`__TAURI__.core.invoke`, `__TAURI__.event.listen`) ; hors Tauri, repli navigateur.

### `src/utils/tauri.ts` — pont de commandes (`invoke`) & événements
```ts
isInvokeAvailable(): boolean                     // __TAURI__.core.invoke présent ? (v2)
invokeCommand<T>(command, args?): Promise<T>     // wrapper typé de `invoke`
isEventAvailable(): boolean                      // __TAURI__.event.listen présent ?
listenEvent<T>(event, handler): Promise<(() => void) | null>   // abonnement (unlisten)
```
- Résout `invoke` sur `window.__TAURI__.core.invoke` (v2), `__TAURI__.invoke` ou
  `__TAURI__.tauri.invoke` (v1), **lié** à son objet.
- Utilisé par `App.runExport` (routage : `generate_docx` / `export_pdf` / `generate_epub`).
  Hors Tauri, repli sur l'**export simulé** + `warning` au journal.
- `listenEvent` s'appuie sur `window.__TAURI__.event.listen` (API **cœur**, exposée avec
  `withGlobalTauri`) et renvoie la fonction de désabonnement ; hors Tauri, renvoie `null`.
  Utilisé par `ExportView` pour suivre l'événement **`export-progress`** émis par le backend.

### `src/utils/directory.ts`
```ts
isTauriAvailable(): boolean                    // __TAURI__.dialog.open présent ?
isFsAvailable(): boolean                       // __TAURI__.fs.readDir présent ?
pickDirectory(): Promise<string | null>
pickDirectoryWithFiles(): Promise<DirectoryPick>   // { path, files, handle }
listDirectoryFiles(path): Promise<string[]>
importFileIntoFolder(handle, directoryPath): Promise<ImportResult | null>
```
- Si Tauri : sélecteur de dossier **natif** → renvoie le **chemin absolu**. Tente d'abord
  `__TAURI__.dialog.open({ directory: true, multiple: false })` (Tauri v1) ; en **Tauri v2** — où
  `withGlobalTauri` n'expose **pas** les API de plugins sur `window.__TAURI__` — appelle la
  commande Rust **`pick_directory`** (`tauri-plugin-dialog`, `pick_folder`).
- Sinon (navigateur) : crée un `<input type="file" webkitdirectory>` invisible, l'ouvre, et
  renvoie le **nom du dossier racine** (les navigateurs n'exposent pas le chemin absolu).
- Renvoie `null` si l'utilisateur annule.

### `src/utils/shell.ts`
```ts
isShellAvailable(): boolean                    // __TAURI__.shell.open présent ?
openPathExternal(path: string): Promise<boolean>
```
- Si Tauri : `shell.open(path)` → ouvre le fichier avec l'éditeur par défaut de l'OS.
- Sinon : renvoie `false` (l'UI journalise alors un `warning`).

Types internes (`WindowWithTauri`, `TauriDialog`, `TauriShell`) évitent d'imposer la
dépendance `@tauri-apps/api`.

---

## 15. Guide de continuation pour un LLM

### Modèle mental
- **Un seul composant racine `App`** détient **tout l'état global** et **tous les handlers**
  qui journalisent. Les composants enfants sont **purement présentationnels** : ils reçoivent
  `value` + `onChange`/`onXxx` en props.
- **Un « routeur » interne** : `SettingsNavigator` (9 niveaux) gère `level` + `direction` et
  l'animation de tour de page. L'onglet **Informations** est une **page unique** (`InfoView`).
- **Aucune modale/pop-up** : toute nouvelle navigation = un nouveau niveau dans un navigateur.
- Les réglages == **configuration du manuscrit exporté** ; jamais l'UI.

### Ajouter une sous-page de Réglages (exemple : « Références bibliographiques »)
1. Créer `src/components/settings/MyPage.tsx` exposant
   `{ config, onChange|onFieldChange, onBack }` (voir `SourcesPage.tsx` comme modèle).
   Utiliser `PillButton`/`RadioRow`/`FontCard` (`settings/controls.tsx`) pour les sélecteurs,
   `ChevronLeft` + « Retour » pour le retour, un titre `font-serif uppercase`.
2. Dans `App.tsx` : ajouter l'état (`useState`) + un handler `useCallback` qui fait
   `setXxx(prev => ({ ...prev, champ: valeur }))` **puis** `addLog(...)`.
3. Dans `SettingsNavigator.tsx` : importer la page, ajouter le **niveau** (`type SettingsLevel`),
   sa **breadcrumb**, son **bloc `{level === N && <MyPage …/>}`**, et un **bouton/carte** qui
   appelle `goTo(N, 1)`. Ne pas oublier `onBack={() => goTo(parent, -1)}`.
4. Ajouter les props dans `SettingsNavigatorProps` et les transmettre depuis `App`
   (dans les **deux** branches `case 'reglages'` et `default`).

### Ajouter un champ à un formulaire Informations
1. Ajouter le champ dans `BookInfoConfig` (`types.ts`).
2. L'ajouter à `DEFAULT_BOOK_INFO` (`App.tsx`).
3. Dans la page concernée (`IdentityPage`/`AdminPage`/`OtherInfoPage`), ajouter un `<InfoField>`
   avec `label`, `value={config.champ}`, `onChange={(v) => onFieldChange("<Section>", "<Label>", "champ", v)}`.
   → la journalisation `[Informations - <Section>] <Label> mis à jour.` est automatique.

### Ajouter un nouvel onglet (marque-page)
1. Étendre `ActiveMenu` (`types.ts`) et le tableau `tabs` dans `LeftPage.tsx`
   (id, label, icône lucide, couleur).
2. Ajouter un `case` dans `renderRightPage()` (`App.tsx`).

> L'action permanente **« Quitter l'atelier »** (`LeftPage`) est **hors** du tableau `tabs`
> et n'a pas d'`id` dans `ActiveMenu` : c'est un bouton dédié appelant `onExitApp`.

### Règles à ne jamais enfreindre
- **`import type`** pour tout type importé (`verbatimModuleSyntax`).
- Pas de variable/import/paramètre inutilisé (`noUnusedLocals`/`noUnusedParameters`).
- Chaque `case` d'un `switch` retourne (`noFallthroughCasesInSwitch`).
- Tailwind v4 : dégradés en **`bg-linear-to-r` / `bg-linear-to-b`**.
- Couleurs du thème (`copper`, `brass`, `gold`, `parchment`, `desk`, `atelier`, `verdigris`,
  `retro-violet`) plutôt que des hexadécimaux ad hoc.
- Journaliser chaque modification d'un réglage via `addLog`.
- Ne jamais appliquer une police/format de `manuscriptConfig` à l'UI.

### Checklist de validation après toute modification
```bash
npx tsc --noEmit -p tsconfig.app.json     # doit retourner 0 erreur
npm run build                             # doit réussir (tsc -b && vite build)
```

---

## 16. État actuel, validation & dette technique

### Ce qui est implémenté (fonctionnel)
- Bureau rétro texturé + registre double-page + reliure (ombres internes).
- Barre de progression rétro-industrielle + journal des opérations.
- Page de gauche : **5 onglets** (marque-pages : Réglages, Informations, Organisation,
  Correcteur, Export) + l'action permanente **« Quitter l'atelier »** sur la tranche + page de
  garde (lettrine, titre, sous-titre).
- **Onglet Réglages** : sommaire + **9 sous-pages** (niveaux 2 à 10), navigation par tour de page.
- **Onglet Informations** : sommaire + **3 formulaires** (Identité, Administratif, Autres informations).
- **Onglet Organisation** : structure du roman (Actes / Chapitres / Images) en glisser-déposer,
  **paginée** (8 éléments par page, sans barre de défilement) avec D&D **multi-pages** (§19).
- **Onglet Export** : 3 formats **générés nativement** (Word via `docx-rs`, **PDF prêt-à-imprimer** via
  Typst embarqué, EPUB via `epub-builder`) + progression/journal réels (`export-progress`).
- **Persistance du projet** : fichier unique `.danoe` (JSON, v1.0), autosauvegarde différée
  (800 ms), Tauri `fs` + repli `localStorage` (§17).
- **Bac à sable** : import de sources **physiquement copiées** dans les dossiers officiels,
  renommage automatique en cas de conflit (§18).
- Journalisation de toutes les actions ; sélection de dossier (Tauri + repli) ; ouverture du
  fichier journal par l'OS (Tauri).

### Validation de référence
| Vérification | Résultat attendu |
|---|---|
| `npx tsc --noEmit -p tsconfig.app.json` | **0 erreur** |
| `npm run build` | Succès (`built in ~0,8 s`) |
| Dev server | `http://localhost:5173/` → HTTP 200 |
| `cargo test --lib` (backend) | **105 tests**, 0 échec |
| `cargo clippy --all-targets --all-features -- -D warnings` | **0 warning** |
| `npm run tauri build` | Succès (exe ~29 MiB + MSI + NSIS) |

> ~~Le build émet un **warning** « Some chunks are larger than 500 kB »~~ → **résolu** :
> `vite.config.ts` segmente les vendors (`manualChunks` : `vendor-pdfjs`, `vendor-motion`,
> `vendor-dnd`, `vendor`) et relève `chunkSizeWarningLimit` à **1200** (voir §3).

### Dette / points d'attention
- `src/App.css` et `src/styles/global.css` : **non importés** (héritage Vite) → supprimables.
- `src/components/SettingsView.tmp` : brouillon → à supprimer.
- `index.html` : `lang="en"` et `<title>danoestudioii</title>` → à franciser / personnaliser
  (`Danoë Studio`).
- ~~Aucun test unitaire~~ → **résolu** : **105 tests Rust** (`cargo test --lib`) couvrant KDP (`kdp.rs`),
  Word (`export.rs`), EPUB (`epub.rs`) et PDF (pipeline **AST → Typst → PDF**), dont les tests de
  **parité** `test_glossary_parity` et `test_layout_parity_page_breaks_across_formats`.
- ~~Aucune persistance~~ → **résolu** (§17) : l'état est sérialisé dans `danoe-studio.danoe`.
  Reste à faire : **mémoriser `projectFilePath`** (l'emplacement choisi est perdu entre deux
  sessions) et purger `fileCache` (fichiers détectés conservés pour les rechargements
  navigateur).
- Les **formats de coupe personnalisés** (saisie manuelle) ne sont **pas** implémentés
  (seuls les 7 presets KDP).
- ~~Moteurs d'export non implémentés~~ → **résolu** (§20) : **Word** (`docx-rs`), **PDF prêt-à-imprimer**
  (Typst embarqué, AST → balisage → PDF) et **EPUB** sont générés nativement ; `runExport` route par format.
  Le repli **navigateur** (hors Tauri) reste une **simulation**.

### Roadmap suggérée
1. ~~Persistance du projet~~ → **fait** (§17) ; reste la mémorisation de `projectFilePath`.
2. ~~Moteurs d'export~~ → **fait** (§20) : Word (`docx-rs`), **PDF prêt-à-imprimer** (Typst embarqué,
   AST → balisage → PDF, Gutter/Bleed via `kdp.rs`), EPUB 3 (`epub-builder` + `nav`). Reste la
   finalisation EPUB (images, notes).
3. ~~Import de sources~~ → **fait** (§18) ; reste la **détection automatique** des fichiers du
   dossier au démarrage (aujourd'hui : à la sélection / après import) et les **vignettes**
   d'images (chemins absolus requis).
4. ~~Calculs KDP avancés~~ → **fait** : centralisés dans **`kdp.rs`** (gouttière dynamique selon le
   nombre de pages, marges minimales, fond perdu) — cf. `specifications_roman_kdp.md`.
5. **Empaquetage Tauri** pour l'accès disque réel (les utils détectent déjà `window.__TAURI__`).
6. **Accessibilité** : passer `index.html` en `lang="fr"` (les onglets exposent déjà `aria-current`).

---

## 17. Sauvegarde & persistance (fichier de projet)

### Format & portabilité
Tout le store est sérialisé en **JSON texte** dans un fichier **`danoe-studio.danoe`**
(texte brut, lisible par un humain, versionnable sur Git, portable sur clé USB / cloud).
Aucune base de données.

### Emplacement
Le fichier est destiné à la **racine de l'espace de travail du roman** (dossier parent
contenant chapitres/images). Il est choisi via le bouton de la **barre de statut** (icône
dossier, libellé = nom du fichier ou « Dossier du projet »), qui appelle `pickDirectory()` :
le chemin cible devient `<dossier>/danoe-studio.danoe`.

### Auto-save (debounce, sans bouton)
Aucun bouton « Enregistrer ». Un `useEffect` (**debounce 800 ms**) observe tous les états
(métadonnées, réglages, dossiers, options, structure, chemin de projet) et écrit le fichier
de manière **asynchrone**. L'écriture ne démarre qu'après **hydratation** (`hydrated`) afin de
ne pas écraser un projet existant par les valeurs par défaut.

### Témoin visuel
À chaque écriture réussie, un témoin discret **« Consigné »** (icône plume `Feather`,
`text-emerald-300`) apparaît **~1,5 s** dans la barre de statut du journal (sous le livre).

### Backend
- **Tauri** : `__TAURI__.fs.writeTextFile` / `readTextFile` (asynchrone, thread UI non bloqué).
- **Repli navigateur** : `localStorage` (clé `danoe-studio-project`).
- Aucune dépendance `@tauri-apps/api` imposée : les API sont détectées sur `window.__TAURI__`.

### Schéma du fichier (`ProjectFile`, version 1.0)
```json
{
  "projectVersion": "1.0",
  "metadata": { "sagaTitle": "…", "bookTitle": "…", "subtitle": "…", "volumeNumber": "…",
                "authorName": "…", "contributor": "…", "year": "…", "isbn": "…", "publisher": "…",
                "printLocation": "…", "copyrightText": "…", "website": "…", "otherBooks": "…" },
  "layoutConfig": { "trimSize": "6x9", "bodyFont": "Garamond", "bodySize": 11,
                    "lineSpacing": 1.15, "textAlignment": "justify", "dropCap": true,
                    "chapterTitleFont": "body", "chapterTitleSize": 16,
                    "subtitleFont": "body", "subtitleSize": 14 },
  "directories": { "sources": "…", "logs": "…" },
  "options": { "imageColorMode": "grayscale",
               "logLevel": "standard" },
  "organization": [ { "id": "uuid-1", "type": "act", "displayName": "ACTE I",
                      "children": [ { "id": "uuid-2", "type": "chapter",
                                      "displayName": "Chapitre 1",
                                      "sourceFileName": "01_introduction.docx" },
                                    { "id": "uuid-3", "type": "special",
                                      "role": "glossary", "displayName": "Glossaire" } ] } ]
}
```
> `organization` est **hiérarchique** (actes avec `children`). En mémoire, la structure reste
> un **tableau plat** ; les conversions `flatToTree` / `treeToFlat` (`utils/projectFile.ts`)
> font le pont dans les deux sens (round-trip fidèle).

### API de `src/utils/projectFile.ts`
`PROJECT_VERSION`, `PROJECT_FILE_NAME`, `flatToTree`, `treeToFlat`, `buildProjectFile(state)`,
`parseProjectJson(text)`, `saveProject(json, filePath) → 'file' | 'storage' | 'error'`,
`loadProject(filePath) → string | null`.

### Log associé
`Fichier de projet défini dans : <dossier>` (à la sélection de l'emplacement).
Les autosauvegardes ne polluent pas le journal (témoin visuel uniquement).

---

## 18. Bac à sable & importation de fichiers

### Principe
Les sources (chapitres, images) sont **centralisées** dans un **dossier unique (Workspace)**
configuré dans Réglages (« Mes sources »). Le fichier projet ne stocke que des **noms de
fichiers (chemins relatifs)** → le dossier maître reste déplaçable (clé USB, cloud) sans
re-paramétrage.

### Tri automatique par extension (`utils/sources.ts`)
Le dossier unique est scanné ; les fichiers détectés sont répartis en **deux tableaux**
distincts (State) selon leur extension, de manière transparente pour l'utilisateur :
- **Chapitres** : `.txt`, `.md`, `.docx` → `chapterFiles`.
- **Images** : `.png`, `.jpg`, `.jpeg`, `.webp`, `.tiff` → `imageFiles`.

Les extensions inconnues sont ignorées. `splitSourceFiles(files)` effectue ce filtrage
+ un tri alphabétique. Le **watcher** (`listDirectoryFiles` sous Tauri) rafraîchit l'état ;
hors Tauri, les fichiers proviennent du sélecteur de dossier lui-même.

### Composants de la ligne source (Organisation)
- **A — Menu déroulant** : le menu « Chapitre » lit **exclusivement** `chapterFiles`
  (textes filtrés) ; le menu « Image » lit **exclusivement** `imageFiles` (illustrations
  filtrées). Rafraîchie automatiquement après import (tri par extension).
- **B — Bouton « Importer »** (cuivre) : ouvre l'explorateur, **copie physiquement** le
  fichier choisi à la **racine du dossier « Mes sources »**, rafraîchit la liste et
  **sélectionne automatiquement** le fichier importé sur la ligne concernée ; le fichier
  apparaît dans le bon menu déroulant selon son extension.

### Backend de copie (`utils/directory.ts → importFileIntoFolder`)
| Environnement | Sélection | Copie | Conflit |
|---|---|---|---|
| **Tauri** | `dialog.open({ directory:false })` | `fs.copyFile(source, dest)` (asynchrone) | renommage auto `_copie`, `_copie2`… (via `fs.exists`) |
| **Navigateur (File System Access)** | `<input type="file">` | écriture via la **poignée de dossier** (`getFileHandle` + `createWritable`) | renommage auto (`handleHasFile`) |
| **Dernier repli** | `<input type="file">` | **impossible** → `copied:false` + `warning` au journal | — |

### Poignée de dossier
`pickDirectoryWithFiles()` privilégie `window.showDirectoryPicker()` (Chrome/Edge) → renvoie
`{ path, files, handle }`. La poignée est conservée en `ref` dans `App`
(`sourceHandleRef`) et permet l'écriture. Repli : `webkitdirectory`
(lecture seule, `handle: null`).

> **Robustesse :** le sélecteur est appelé **lié à `window`** (un appel détaché peut lever
> « Illegal invocation ») et l'**énumération des fichiers est isolée** de la sélection :
> une erreur de listage ne doit **jamais** annuler le dossier choisi — le chemin reste
> injecté dans l'état (`sourceConfig.directory`) et le champ se met immédiatement à jour.
>
> **Chaîne de repli :** si `showDirectoryPicker()` **échoue** (ex. `SecurityError` dans une
> iframe / aperçu webview) ou est **indisponible**, on retombe automatiquement sur le
> sélecteur `<input type="file" webkitdirectory>` **au lieu de renvoyer `null`** — une
> sélection ne doit jamais échouer silencieusement. Seule une **annulation explicite**
> (`AbortError`) est traitée comme « aucun dossier ».

### Journal
- `Chapitre importé dans le projet : <fichier>` / `Illustration importé dans le projet : <fichier>` (success).
- `Import sans copie (application empaquetée requise) : <fichier>` (warning).

---

## 19. Organisation — pagination & D&D multi-pages

- **Limite d'affichage** : `ITEMS_PER_PAGE = 8` (aucune barre de défilement dans le livre).
- **Vue = tranche** : `items.slice((page-1)*8, page*8)` ; le store reste un **tableau plat
  unique** (sérialisé tel quel via `flatToTree`).
- **Navigation** (bas de page, centrée, typographie cuivrée discrète) : `← Page précédente`
  (grisé si page 1) · `Page X / N` · `Page suivante →` (grisé en dernière page).
- **D&D multi-pages (Option A)** : pendant un glisser, survoler `Page précédente/suivante`
  **> 800 ms** change la page (détection par coordonnées du pointeur dans le rectangle du
  bouton, via `onDragMove`). L'élément en cours de déplacement est **maintenu monté**
  (`visibleItems` l'inclut) pour que le dépôt fonctionne. Les index de réordonnancement sont
  calculés sur le **tableau global**.
- L'indentation (appartenance à un Acte) est calculée sur le tableau global (`indentById`).

**Ancrage du pied de page** : la page de droite du `Layout` est une colonne flexible
(`flex flex-col`), la racine de la vue est `flex h-full grow flex-col`, la zone de contenu
(outils + liste) est `flex-1`, et le bloc de pagination porte `mt-auto` — il est donc
mécaniquement plaqué au bas de la page.

---

## 20. Backend Tauri & moteurs d'export (KDP — Word / PDF / EPUB)

Le backend de bureau est **implémenté** dans **`src-tauri/`** (Tauri v2 + Rust) :
- `Cargo.toml` : `tauri`, **`tauri-plugin-dialog`**, `serde`, `serde_json`, **`docx-rs` 0.4**, `zip` 8,
  **`epub-builder` 0.8**, **`typst` 0.12**, **`typst-pdf` 0.12**, **`typst-assets` 0.12** (feature `fonts`).
- `tauri.conf.json` : `withGlobalTauri: true`, `bundle.resources = ["assets/**/*"]`.
- `.taurignore` : fichiers **exclus du watcher** `tauri dev` — `target/` (binaires réécrits par
  cargo → erreurs **`EBUSY`** sous Windows) et `assets/` (images manipulées). En **Tauri v2**, la
  clé `build.watch.ignore` (v1) n'existe plus : l'équivalent est ce fichier `.taurignore`.
- `capabilities/default.json` : `core:default` + **`dialog:default`**.
- `src/` :
  - `lib.rs` — enregistrement des commandes, plugin dialog, `run()` (cycle de vie : purge du cache
    volatile sur `RunEvent::Exit`, émission de progression `emit_progress`).
  - `commands.rs` — commande **`export_pdf(payload)`** : orchestration **AST post-traité → Typst → PDF**
    (voir ci-dessous) ; `build_blocks` aligne l'AST sur l'export Word (définitions, numérotation, glossaire).
  - `kdp.rs` — **module central KDP** (single source of truth) : formats de coupe, gouttière, marges, fond perdu.
  - `pdf/` — moteur PDF embarqué : `pdf/generator.rs` (AST → balisage Typst : typographie, en-tête
    conditionnel, sommaire `#outline`, glossaire, lettrines), `pdf/compiler.rs` (environnement
    `typst::World`, polices embarquées **+ système** avec repli), `pdf/mod.rs` (`compile_to_pdf_with_root`).
  - `export.rs` — génération **Word** (`docx-rs`) + assemblage des sections ; sous-modules `export/body.rs`
    (arborescence, Markdown, images, lettrines, glossaire, titres de chapitre), `export/front_matter.rs`
    (couverture + liminaires), `export/helpers.rs` (polices, chemins, images), `export/post_process.rs`
    (**post-traitement OOXML** via `zip` : en-têtes courants, folio, `oddPage`, `vAlign`, lettrines).
  - `epub.rs` — export **EPUB 3** (`epub-builder`) + CSS d'aération structurelle.
  - `project.rs` — désérialisation du payload (les règles KDP vivent dans `kdp.rs`).

**Pont** : `invokeCommand(...)` (`src/utils/tauri.ts`) route vers `generate_docx` (Word),
`export_pdf` (PDF Typst) ou `generate_epub` (EPUB). Le backend construit les octets, **ouvre une
boîte de dialogue système** (Tauri Dialog, `blocking_save_file`, hors thread async via
`spawn_blocking`) et écrit le fichier choisi — toute erreur est remontée en `Result::Err` (promesse
`invoke` **rejetée** côté React).

**Progression** : `generate_docx` émet l'événement **`export-progress`** (`app.emit`, payload
`{ step: string, progress: number }`) aux étapes clés — Lecture des sources (10 %), Analyse (30 %),
Structuration (60 %), Injection de la TOC/colophon (80 %), Enregistrement (90 %), Écriture disque
(100 %). `ExportView` s'y abonne (`listenEvent`) et affiche une **jauge** + l'étape en cours, avec
`unlisten` au démontage (nettoyage de l'écouteur).

### Module central `kdp.rs` — Single Source of Truth (KDP)
Toutes les règles physiques **Amazon KDP Print** sont centralisées dans **`src/kdp.rs`**, seule
source de vérité (aucune constante dupliquée ailleurs) :
- **Formats de coupe** : `TRIM_SIZES` (7 presets), `trim_size(id)`, `trim_size_twips(id)` (repli sûr 6×9).
- **Gouttière dynamique** : `gutter_twips(pages)` / `gutter_mm(pages)` — table KDP 24–150 → 9,6 mm … 701+ → 22,3 mm.
- **Marges** : minimums obligatoires (`MIN_MARGIN_*_MM`) + marges de service (`SIDE/TOP/BOTTOM/HEADER/FOOTER_MARGIN_TWIPS`).
- **Fond perdu (bleed)** : `BLEED_OUTER_IN` / `BLEED_VERTICAL_IN` (+0,125 / +0,25 po).
- **Agrégat prêt pour les moteurs** : `KdpPageSpec::resolve(trim_id, pages, bleed)`
  (+ `content_width_twips` / `content_height_twips`).
- **Unités** : `TWIPS_PER_INCH`, `TWIPS_PER_MM`, `EMU_PER_TWIP` + conversions.
`project.rs` se limite à la désérialisation du payload et à l'estimation paginée (bornée par
`kdp::{MIN_PAGES, MAX_PAGES}`). Word (`export.rs`) **et** PDF (`pdf/`) consomment **exclusivement** cette API.

### Pipeline d'export PDF natif (AST → Typst → PDF, 100 % embarqué)
Le PDF prêt-à-imprimer est produit **sans aucune dépendance externe** (ni binaire, ni service) :
1. `commands.rs::export_pdf` parse le `ProjectPayload` et **aplatit l'arborescence en AST Markdown**
   post-traité (`build_blocks`) — **même contrôle que le Word** : `extract_definitions`, puis
   `number_refs` (ou `strip_refs` hors glossaire) ; un titre `#` de tête de source est **supprimé**
   quand le nœud porte un `displayName` (pas de doublon) ; nœuds image → `Block::Image`.
2. `pdf::generate` traduit l'**AST → balisage Typst** (`pdf/generator.rs`) :
   - **mise en page** `#set page` (format, marges miroir `inside`/`outside` + gouttière via `KdpPageSpec`),
     folio en pied ;
   - **en-tête courant = titre du chapitre en cours**, centré au-dessus d'un **trait continu**
     (`line(length: 100%)`) ; **masqué** sur toute **page structurelle** (`query(heading.where(level: 1))`
     ou `query(figure)` renvoie un élément sur `here().page()` → `none`) ;
   - **typographie câblée depuis l'UI** : `#set text(font, size)`, `#set par(justify, leading)`,
     `#show heading.where(level: 1|2): set text(font, size)` (corps / titre de chapitre / sous-titres) —
     chaque **pile de polices** se termine par la police embarquée (repli garanti) ;
   - **règles d'aération** : niveau 1 = page entière centrée H+V (`pagebreak` + `block(height: 100%,
     align(center + horizon))`), niveau 2 = saut de page systématique, illustration isolée sur sa page
     (`figure(image(...))` centrée) ;
   - blocs `Heading / Paragraph / Quote / ListItem / SceneBreak / Image`, gras/italique, **notes**
     (`#super[n]`), **glossaire final unifié**, **lettrines** jointives (`#dropcap`), **sommaire**
     (`#outline`).
3. `pdf::compile_to_pdf_with_root` compile via l'environnement `typst::World` (`pdf/compiler.rs`) qui
   résout `source()` / `file()` (images depuis `directories.sources`) et charge les polices : **embarquées**
   (`typst-assets`, Libertinus) **puis** **polices système** (scan récursif `%WINDIR%\Fonts`,
   `%LOCALAPPDATA%\...\Fonts`, `/Library/Fonts`, `/usr/share/fonts` ; `.ttf/.otf/.ttc/.otc`, dédupliquées,
   plafonnées), enfin repli candidats explicites ; `typst` met en page, `typst-pdf` exporte.
- Les dimensions Typst dérivent de `kdp.rs` (twips → points : `1 twip = 1/20 pt`).
- **Tests Rust** : pipeline bout-en-bout (`KdpPageSpec → balisage → PDF`, `%PDF-` + `%%EOF`),
  câblage typo/notes/sommaire, `export_pdf` depuis un payload complet, et **parité tri-format**
  (`test_layout_parity_page_breaks_across_formats`) — cf. `pdf::tests` et `commands::tests`.

### Politique « Tolérance Zéro » (erreurs)
Aucun échec silencieux : **toute** erreur est propagée en `Result::Err` jusqu'à l'UI Tauri (la promesse
`invoke` est **rejetée** ; `App.runExport` journalise + `Toast` d'erreur) :
- **I/O & fichiers** : source introuvable (`Fichier source introuvable : …`), illustration sans fichier
  lié, écriture disque impossible (`std::fs::write` → `Err`).
- **Images** : `FileError::NotFound` remonté par `TypstWorld::file`.
- **Rendu** : diagnostics de syntaxe/mise en page Typst (`typst::compile`) et échec d'export (`typst-pdf`).

### Aperçu interactif — Flipbook hybride (Typst → octets → PDF.js → react-pageflip)
Aperçu **feuilletable fidèle à l'impression**, rendu **100 % en mémoire** (aucune écriture disque) :
1. **Backend** — `commands::render_pdf(payload)` réutilise `render_pdf_bytes` (même pipeline que
   `export_pdf` : `build_blocks → pdf::generate → pdf::compile_to_pdf_with_root`) et renvoie les octets
   **bruts** via `tauri::ipc::Response` (→ `ArrayBuffer` côté frontend, **sans** sérialisation JSON).
2. **Pont** — `src/utils/pdfPreview.ts::renderPdfBytes` normalise la réponse en `Uint8Array`.
3. **Frontend** — `src/components/PreviewFlipbook.tsx` : `pdf.js` (`pdfjs-dist`, worker servi par Vite)
   rasterise chaque page sur un **`<canvas>`**, injecté dans `react-pageflip` (`HTMLFlipBook`).
   - **Lazy rendering** : seules les **4 premières pages** sont rendues à l'ouverture ; un buffer
     **±2 pages** autour de `current` est pré-chargé (`ensureRendered`, idempotent via
     `renderedRef`/`inFlightRef`) ; canvas initialisés en **1×1** (mémoire minimale) puis redimensionnés.
   - **Navigation** : rouleau `<input type="range">` → `pageFlip().turnToPage()` ; **double page** sur
     grand écran (ombre de reliure centrale), **simple page** sous 720 px ; ratio KDP préservé.
   - **Indicateur** : spinner « Chargement… » sur chaque page non encore rasterisée.
   - Accès : onglet **Export → « Aperçu interactif »** (overlay plein écran piloté par `App`).

### Sauvegarde native (UX)
L'enregistrement des trois formats passe **exclusivement** par la **boîte de dialogue native de l'OS**
(`tauri-plugin-dialog`, `blocking_save_file`, hors thread async via `spawn_blocking`) : l'utilisateur
choisit l'emplacement, le backend écrit les octets. Le réglage « **Dossier des exports** » est
**supprimé** : plus aucun chemin d'export mémorisé, aucune organisation en sous-dossiers (chaque export
est un fichier choisi au cas par cas). La progression `export-progress` est émise jusqu'à `Terminé (100 %)`.

### Phase 1 — livrée : couverture (optionnelle) + pages liminaires
> **KDP Print** : le fichier **intérieur** ne contient **pas** la couverture (produite via un PDF
> séparé). Le document Word commence donc directement par les **pages liminaires** (faux-titre).
> La couverture n'est produite que si `build_docx(..., include_cover = true, ...)` est demandé
> (rendu « livre complet » — contrôle visuel uniquement ; **`false` par défaut**).
- **Couverture (si `include_cover`)** : image de fond **pleine page** (`Pic` flottante, ancrage page, offsets 0),
  texte superposé `#F5F5DC` — titre 42 pt majuscules (tiers supérieur), sous-titre 18 pt italique,
  auteur 24 pt (quart inférieur).
- **Surcharge** : si un fichier **`couverture.*`** (`.jpg/.jpeg/.png/.webp/.tiff`) est présent à la
  racine de `directories.sources`, il **remplace** la ressource embarquée.
- **Sections** : saut de section après la garde → marges **KDP** des pages liminaires rétablies.
- **Pages liminaires** (ordre KDP : ① faux-titre → ② copyright → ③ page de titre) : le **faux-titre**
  et la **page de titre** portent le **titre de saga** centré au-dessus du titre principal (capitales
  réduites) ; la **page de copyright** (mention générée ou saisie, ISBN, lieu d'impression) est
  **entièrement centrée** et ancrée **en bas** de page ; la **page de titre** porte titre /
  sous-titre / auteur + **mention d'édition** (`Éditeur · année`) ancrée **en bas** de page.
- **Gouttière KDP** calculée selon l'épaisseur estimée (`options.estimatedPageCount`, sinon
  `24 + chapitres × 20`) — table 24–150 → 9,6 mm … 701+ → 22,3 mm.

**Post-traitement OOXML** — deux limites de `docx-rs` 0.4.22 contournées :
1. `behindDoc` étant **codé en dur à `0`**, l'image de couverture est basculée **« derrière le
   texte »** (`behindDoc="1"`, `allowOverlap="1"`, habillage `wrapNone`).
2. `word/settings.xml` reçoit **`<w:mirrorMargins/>`** (marges en vis-à-vis / miroir), inséré **avant**
   `w:compat` / `w:evenAndOddHeaders` pour respecter la séquence du schéma (`CT_Settings`) — sans quoi
   Word peut ignorer le miroir et la gouttière de reliure. La gouttière est injectée dans la marge
   intérieure via `w:gutter` des `<w:pgMar>` (`crate::kdp::gutter_twips`).

### En-têtes, pieds de page & foliotation (Word)
- **Global** : `Settings::even_and_odd_headers()` → `<w:evenAndOddHeaders/>` (pages paires/impaires différentes).
- **En-tête courant = titre de l'élément en cours** (chapitre/acte, `displayName`), **centré**, 8 pt,
  souligné d'un **trait continu** (`<w:pBdr><w:bottom …/></w:pBdr>`), injecté **par post-traitement**
  (`inject_running_headers`) — chaque `<w:sectPr>` référence **sa** partie d'en-tête (corrige le décalage
  de `docx-rs`).
- **Première page sans en-tête** : chaque section porte `<w:titlePg/>` (« Première page différente ») et
  un `<w:headerReference w:type="first" …>` pointant vers la partie **vide** → l'en-tête courant démarre
  **sur la page suivante** (page de corps), la page de titre structurelle reste vierge.
- **Pieds de page** : **folio centré** (champ `PAGE`, `inject_folio_footers`) sur les sections du corps.
- **Centrage vertical absolu** (illustrations) : `<w:vAlign w:val="center"/>` injecté dans le `<w:sectPr>`
  des sections marquées `vertical_center` (`inject_vertical_centering`), dans l'ordre du schéma (`CT_SectPr`).
- **Foliotation** : `page_num_type(PageNumType::new().start(1))` → la numérotation **débute à 1 au
  faux-titre**, jamais sur la couverture (section de couverture sans en-tête ni pied).
- *(Dette)* les « pages fantômes » des sauts `oddPage` ne devraient pas être foliotées — dépend du rendu
  de mise en page (§4, §20).

**Tests Rust** (`cargo test`) : génération de bout en bout (couverture derrière-texte, marges miroir,
ISBN / lieu d'impression, en-tête = nom du chapitre + trait), repères KDP, et
`structural_sections_blank_header_on_first_page` (`#sectPr == #titlePg == headerReference "first"→vide`).

### Phase 2 — livrée : corps, Markdown & glossaire
- **Itération** sur `organization` (actes → chapitres / pages spéciales) ; lecture des `.txt`/`.md`
  dans `directories.sources` ; une **section** par acte / chapitre / page spéciale.
- **En-tête dynamique** = **titre de l'élément en cours** (chapitre/acte), centré au-dessus d'un trait
  continu (`inject_running_headers`) ; **première page de section vide** (`<w:titlePg/>` +
  `headerReference w:type="first"` → partie vide).
- **Titre de chapitre** : concaténation « **`displayName - #titre`** » (ex. *« Chapitre 1 - Le Dernier
  Soir »*) via `combined_chapter_title` ; un **saut de page** (`<w:br w:type="page"/>`) **suit** le titre
  → page de titre isolée, corps sur la page suivante.
- **Markdown** : chaque ligne non vide = un paragraphe (justifié, interligne configurable,
  **retrait de première ligne 0,5 cm**) ; `**gras**` et `*italique*` interprétés.
- **Notes / glossaire** (règles identiques Word **et** PDF — §11) :
  - **Cas A** (aucune page `glossary`) : renvois `[^x]` retirés, lignes de définition ignorées.
  - **Cas B** (page `glossary`) : renvois → **numéros en exposant** (réinitialisés à 1 par chapitre ;
    Word = `<w:vertAlign superscript>`, PDF = `#super[n]`) ; définitions extraites, **agrégées par
    chapitre**, rendues sous un sous-titre `Acte X, chapitre Y` (**Titre 2**) + entrées
    `<n° super> **Terme** : définition`, séparateur `* * *` entre groupes.
- **Pages spéciales** : **dédicace** = alignée à droite, italique, repoussée en bas de page
  (`LineSpacing::before(5670)`) ; prologue / épilogue / note / remerciements = titre + corps.
- **Post-traitement `oddPage`** : `<w:type w:val="oddPage"/>` injecté dans le `<w:sectPr>` de chaque
  section du **corps** (démarrage sur page impaire).

**Tests Rust** (`cargo test`) : couverture / liminaires, repères KDP, **corps + notes exposant +
glossaire agrégé + `oddPage` + en-tête dynamique**, et `chapter_title_combines_number_and_name_and_breaks_after`.

### Phase 3 — livrée : syntaxe étendue, images & finitions
- **Markdown étendu** : `#` → **Titre 1** (texte **capturé pour l'en-tête** courant), `##` → **Titre 2**
  (sans retrait de première ligne, espacements) ; `---`/`***` → **séparateur de scène** centré `* * *` ;
  `> ` → **citation** (retrait gauche/droite, justifiée, italique) ; `- `/`* ` et `1. ` → **listes
  natives** (`w:numPr` : puces `numId=1`, numérotée `numId=2`).
- **Styles de titres (collecte TOC)** : les paragraphes issus de `#`/`##` **ainsi que les titres de
  section / acte / glossaire** portent l'attribut local `.style("Heading1"/"Heading2")` **et** un
  `outlineLvl` ; les styles sont **déclarés globalement** via `add_style` avec le **nom intégré Word**
  (`"heading 1"`/`"heading 2"`) et `outlineLvl` (`0`/`1`). Sans cette déclaration globale, Word ignore
  l'assignation locale (`w:pStyle`) et la table des matières reste vide (« aucun style de titre »).
- **Images** : `![alt](cible)` → image **centrée** (`<w:jc w:val="center"/>`), lue dans
  `directories.sources` ; **dimensionnée en EMU pour exploiter la largeur utile maximale** de la zone de
  contenu KDP (`KdpPageSpec` : largeur totale − marges − gouttière), **ratio conservé** (`<wp:extent>`).
  Nœud `image` de l'organisation → **section isolée** (`oddPage` avant/après) et **centrée verticalement**
  (`<w:vAlign w:val="center"/>` via `vertical_center` — plus d'espacement `before` fixe).
- **Glossaire raffiné** : plus de syntaxe littérale `[^n]:` ; **numéro en exposant** suivi du **terme en
  gras** auto-détecté avant les deux-points (`¹ **Neumes** : …`).
- **Sous-titres (`##`)** : **saut de page systématique** avant (Word `page_break_before` ; PDF
  `#show heading.where(level: 2): pagebreak` ; ePUB `h2 { break-before: page }`).
- **Limite connue (§4 — dé-foliotation des pages fantômes)** : la page « fantôme » insérée par Word
  avant un saut `oddPage` appartient à la **section précédente** et hérite de son pied ; sa
  dé-foliotation **n'est pas exprimable statiquement en OOXML** (elle dépend du rendu de mise en page).
  À traiter via un moteur de mise en page ou une politique « folio sur recto seulement ».

**Tests Rust** (`cargo test`) : 3 tests — dont le test de bout en bout couvrant titres `#`/`##`,
séparateur de scène, citation, listes natives, **image embarquée**, note en exposant, glossaire sans
syntaxe littérale et **en-tête dynamique issu du `#`**.

### Phase 4 — livrée : table des matières, achevé d'imprimer & UX
- **Table des matières native** : section `oddPage` « Sommaire » insérée **après les liminaires et
  les dédicaces/épigraphes d'ouverture**, avant le prologue/chapitre 1 ; champ **TOC** (`\o "1-2"`,
  hyperliens) listant les **Titres 1 et 2** ; `settings.xml` reçoit `<w:updateFields w:val="true"/>`
  → Word actualise la pagination à l'ouverture (ou via `F9`).
- **Achevé d'imprimer (colophon)** : **ultime section** (saut de page simple, **hors** `oddPage`),
  texte **centré ~10 pt** repoussé en bas de page : *« Achevé d'imprimer en <date> par <lieu>. »* +
  *« Dépôt légal : <date>. »* (données `Informations`).
- **UX frontend** : `ExportView` reçoit `isExporting` → boutons désactivés + indicateur
  *« Génération du manuscrit en cours… »* ; **`Toast`** discret (`src/components/Toast.tsx`) en fin
  d'opération (succès / annulation / erreur), auto-fermant.

**Tests Rust** (`cargo test`) : 3 tests — le test de bout en bout vérifie en outre le **champ TOC**,
le titre « Table des matières », le **colophon** (`par France`, `Dépôt légal`) et `updateFields`.

### Phase 5 — livrée : lettrines (drop caps)
- **Détection** : le **premier paragraphe standard** qui suit un titre (`#` → Titre 1, ou le titre
  synthétique de section) reçoit une lettrine ; les blocs intermédiaires (Titre 2, séparateur de
  scène, citation, liste, image) l'annulent (`drop_cap_pending`).
- **Rendu** : la **première lettre** est détachée dans une `Run` de corps supérieur
  (`DROP_CAP_SCALE × bodySize`) ; le paragraphe reçoit un cadre `w:framePr` (`wrap="around"`,
  `vAnchor`/`hAnchor="text"`) et **pas** de retrait de première ligne.
- **Limite `docx-rs` & post-traitement** : `docx-rs` 0.4.22 expose `w:framePr` mais **pas** les
  attributs `w:dropCap` / `w:lines` requis par Word pour un vrai drop cap. Ces deux attributs sont
  **injectés en post-traitement** (`apply_drop_caps`) sur les `w:framePr` du `document.xml` →
  `<w:framePr w:dropCap="drop" w:lines="2" …>` (aucun autre usage de `framePr` dans le document).
- **Contournement OOXML — paragraphes courts** : un cadre `w:framePr` plus haut que son paragraphe
  s'étend sous lui ; avec `wrap="around"`, Word fait alors **remonter le paragraphe suivant** autour du
  cadre (mise en page détruite). La lettrine n'est donc appliquée **que** si le paragraphe couvre au moins
  la hauteur du cadre (`DROP_CAP_LINES`) ; sinon le paragraphe reste standard (décision prise au rendu,
  `export/body.rs`).

**Tests Rust** (`cargo test`) : le test de bout en bout vérifie `w:framePr`, `w:dropCap="drop"`,
`w:lines="2"`, `w:wrap="around"`, et la séparation de la 1ʳᵉ lettre (« P » + « remier paragraphe… »).
- **PDF (Typst)** : lettrine rendue par `#dropcap(letter)[corps]` avec un balisage **jointif** (l'appel et
  le corps se suivent sans espace) → aucun caractère d'espace parasite (« L a flamme » → « La flamme »).

### Phase 6 — livrée : export EPUB 3 (`epub-builder`)
- **Commande** `generate_epub(payload)` (Tauri, asynchrone) : métadonnées (titre, auteur, langue `fr`),
  **page de titre** (saga / titre / sous-titre / auteur), **un XHTML par acte / chapitre / page**
  (Markdown → `<h1>/<h2>/<p>/<blockquote>`, échappement HTML), feuille de style minimale et
  **sommaire** (nav) généré ; dialogue d'enregistrement **natif** `.epub` + progression `export-progress`.
- **Spécificités EPUB** (format fluide, sans pagination fixe) :
  - le **glossaire global** (page spéciale `glossary` agrégeant les définitions de tout le livre) est
    **exclu** ;
  - les **notes sont placées en fin de chapitre** (notes de fin, reflowables) — **jamais** en bas de page.
- **Aération structurelle (CSS)** — parité avec PDF/Word :
  - `h1 { break-before: page; min-height: 100vh; display: flex; align-items: center; justify-content: center; }`
    (titres de niveau 1 isolés, centrés H+V) ;
  - `h2 { break-before: page; }` (sous-titres → saut de page) ;
  - `.figure-page { break-before: page; break-after: page; min-height: 100vh; display: flex; … }` —
    toute illustration `![alt](cible)` est rendue `<div class="figure-page"><img …/></div>` (page dédiée,
    centrée) ;
  - `.title-page` centrée verticalement (`min-height: 100vh`).
- **Frontend** : `App.runExport` route `Ebook (.epub)` → `generate_epub` (PDF → `export_pdf`, Word sinon).
- **Limites (à venir)** : embarquement des médias image, couverture et lettrine en EPUB.

**Tests Rust** (`cargo test`) : `build_epub_produces_a_zip` (signature ZIP « PK »),
`default_epub_name_uses_slug`, et `test_layout_parity_page_breaks_across_formats` (CSS `break-before`/
`.figure-page` + `class="figure-page"` dans une section).

> **Phase 7 — livrée : export PDF natif.** `commands::export_pdf` → AST Markdown **post-traité** →
> balisage Typst → `typst`/`typst-pdf` (PDF prêt-à-imprimer KDP : format & gouttière via `kdp.rs`,
> marges miroir, en-tête courant = **titre du chapitre** + trait, folio, **sommaire** (`#outline`),
> **glossaire final unifié**, lettrines, images). Typographie **câblée depuis l'UI** (`LayoutConfig`) et
> polices avec **repli système/embarquées**. Restent à traiter : foliotation configurable et
> dé-foliotation des pages fantômes via le moteur de mise en page, et embarquement média en EPUB.

### Règles d'aération structurelles (PDF / DOCX / ePUB — Single Source of Truth)

Appliquées **à l'identique** par les trois moteurs :

| Règle | Typst (PDF) | Word (DOCX) | ePUB |
|---|---|---|---|
| **Titre niveau 1** (Acte/Chapitre) isolé, centré H+V | show-rule : `pagebreak` + `block(height: 100%, align(center + horizon, it))` | saut de section `oddPage` ; titre repoussé (`before`) ; `<w:br w:type="page"/>` après le titre | `h1 { break-before: page; min-height: 100vh; flex center }` |
| **Sous-titre** (`##`) → saut de page | `#show heading.where(level: 2): pagebreak(weak: true)` | `page_break_before` **systématique** sur `Heading2` | `h2 { break-before: page }` |
| **Illustration** isolée, centrée H+V | `figure(image(...))` dans `block(height: 100%, align(center + horizon, …))` | section isolée (`oddPage`) + `<w:vAlign w:val="center"/>` ; image en pleine largeur utile | `.figure-page { break-before/after: page; 100vh flex center }` |
| **En-tête courant** | titre du chapitre, masqué sur page structurelle | titre de l'élément, centré + trait ; page de titre vide (`titlePg` + `first`) | — (reflowable) |

**Test de parité** : `commands::tests::test_layout_parity_page_breaks_across_formats` (même AST →
sauts de page/centrages vérifiés dans les 3 formats).

### Cycle de vie & purge du cache volatile (`lib.rs`)

- **Dossier `volatile`** : `app_cache_dir()/volatile` — **strictement distinct** des données utilisateur
  (`app_data_dir`, projet `project.danoe`) et du profil WebView (`localStorage`).
- **Purge garantie & synchrone** : `run()` construit l'app puis `app.run(|app, event| …)` ; sur
  **`RunEvent::Exit`**, `purge_directory(&volatile)` s'exécute **avant** l'arrêt du thread principal —
  quel que soit le chemin de fermeture (bouton applicatif, croix de fenêtre, Alt+F4, `app.exit()`).
  La commande frontend `clear_cache_and_exit` (purge + `exit(0)`) reste disponible (double purge idempotente).
- **Log WebView2 `Chrome_WidgetWin_0 … Error = 1412`** : bruit **upstream** (race interne à Chromium à la
  destruction des fenêtres). Sans incidence sur le code de sortie (`0`) ni sur la terminaison du processus
  (aucun processus fantôme) → **aucune correction requise**.

### Couverture cuir (page de garde)

> **Non incluse dans le fichier intérieur KDP Print** (couverture = PDF séparé) : le rendu Word
> démarre sur les liminaires. Cette section décrit le rendu « livre complet » activé par
> `include_cover = true`.

### 1. Asset & paquetage
- Placer le fichier **`Couverture_Cuir_01.jpg`** dans **`src-tauri/assets/`** (emplacement
  réservé par `src-tauri/assets/README.md`).
- Déclarer la ressource dans **`tauri.conf.json`** :
  ```json
  { "bundle": { "resources": ["assets/Couverture_Cuir_01.jpg"] } }
  ```
- Charger en mémoire : `include_bytes!("../assets/Couverture_Cuir_01.jpg")` **ou**
  `app.path().resolve("assets/Couverture_Cuir_01.jpg", BaseDirectory::Resource)`.

### 2. Injection de l'image de fond (1ʳᵉ page)
- Image insérée **pleine page**, dimensionnée **exactement** sur le **format de coupe KDP**
  sélectionné (`layoutConfig.trimSize`, p. ex. `6x9` → `TRIM_PRESETS` : `width=6, height=9,
  unit='in'`).
- **Habillage** = *Derrière le texte* (**Behind Text**) ; **marges nulles** ; ancrage **page**
  (distances `0`) → couverture **bord à bord** ; texte superposé (§3).

### 3. Superposition typographique (métadonnées)
- **Couleur** : claire et contrastée — beige/blanc cassé **`#F5F5DC`** (ou doré/cuivré clair).
- **Cadrage** : tenir compte de la **ligne de couture** (bord gauche) → centrer le texte
  **visuellement sur la zone lisse**, à droite de la couture.
- **Titre du livre** : tiers supérieur, **Serif majuscules**, ~**42 pt**.
- **Sous-titre** : sous le titre, **Serif italique**, ~**18 pt**.
- **Nom de l'auteur** : quart inférieur, **Serif**, ~**24 pt**.
- Source : `metadata` du projet (`bookTitle`, `subtitle`, `authorName`).

### 4. Transition
- **Saut de section** immédiat après la page de garde → retour à un **fond blanc standard** et
  aux **marges de gouttière KDP** classiques pour les pages liminaires (faux-titre, etc.).

---

## 23. Matrice de conformité et limites connues

Synthèse « source de vérité » : fonctionnalités **validées** vs **limites connues**, par moteur / module.

| Domaine | Statut | Validé (implémenté) | Limite connue |
|---|---|---|---|
| **Word (`.docx`)** | ✅ Livré | Styles natifs, aération (titres `oddPage`, `page_break_before`), en-tête courant dynamique, images (EMU pleine largeur utile, ratio conservé), glossaire en exposant + notes, TOC native (`Heading1/2` + `outlineLvl`), colophon | Dé-foliotation des **pages fantômes** non exprimable statiquement en OOXML (§20 Phase 3) |
| **PDF Typst** | ✅ Livré | Prêt-à-imprimer KDP (gouttière/fond perdu via `kdp.rs`), marges **miroir**, `#outline`, glossaire final unifié, lettrines (`#dropcap`), images, 100 % embarqué (aucun binaire externe) | Foliotation configurable ; polices dépendantes du **repli système/embarqué** |
| **EPUB 3** | ✅ Livré | Structure **fluide**, parité des sauts de page (`break-before`), notes **fin de chapitre**, `nav` généré, dialogue natif `.epub` | **Non embarqués** : médias image, couverture, lettrine |
| **Correcteur** | ✅ Livré | LanguageTool (`fr`, `fr-FR/-BE/-CA/-CH`), masquage Markdown **non destructif**, dictionnaires persistants (`app_data_dir`), mode **Pointilleux**, règles humanisées + « Réactiver » | Dépendance au **serveur d'analyse** ; suggestions **non auto-appliquées** |
| **KDP Print** | ✅ Livré | 7 formats de coupe (`TRIM_PRESETS`), gouttière **dynamique** selon la pagination, marges minimales, fond perdu ; couverture = **PDF séparé** | Formats de coupe **personnalisés** (saisie libre) non pris en charge ; dé-foliotage recto/verso |
| **Couverture KDP (Full Wrap)** | ✅ Oui (100 %) | Tests unitaires Rust (`cover`) · Planche PDF 300 DPI conforme KDP Print | Hardcover reporté en v1.2 |

> **Légende** : ✅ Livré · ⚠️ Partiel · ❌ Non implémenté.

---

## 24. Atelier de Couverture (Cover Studio)

Sous-module de génération de la **couverture complète KDP** (« full wrap » : plat 4 | tranche | plat 1).
Rendu par `CoverStudioView.tsx` (onglet **Couverture**, `activeMenu = 'couverture'`), placé
**entre Organisation et Correcteur** dans le ruban gauche (§7.3).

### 24.1 Fonctionnalités
- **Inspection DPI temps réel** : `inspect_cover_images` lit l'**en-tête** des images (sans décoder
  les pixels) → dimensions, canal alpha (`Rgba8`/`Rgba16`), DPI effectif vs format de coupe ; statut
  🟢 ≥ 300 · 🟡 250–299 · 🔴 < 250 · 🔵 alpha aplati.
- **Calcul KDP de tranche** (`calculate_cover_geometry`) : parité des pages (`pages + pages % 2`),
  coefficients papier **blanc 0,05720 / crème 0,06350 / couleur 0,05960 mm/page**, texte de tranche
  éligible dès **80 pages**, gabarit `3,2 + coupe + tranche + coupe + 3,2` et réserve code-barres
  (50,8 × 30,5 mm) au coin inférieur droit du plat 4.
- **Couleur de tranche** (`extract_spine_color`) : médiane RVB des **10 colonnes** internes de chaque
  plat ; dégradé recommandé si l'écart euclidien sRGB dépasse **40 points**.
- **Rendu Typst 300 DPI** (`export_kdp_cover_pdf`) : pré-traitement (aplatissement des images à canal
  alpha sur blanc `#ffffff` dans `std::env::temp_dir()`), balisage Typst **4 calques** (plat 4, plat 1,
  tranche vectorielle + texte tourné, réserve code-barres), compilation PDF embarquée.
- **Table de montage 2D** (`CoverCanvas2D.tsx`) : planche SVG étalée aux proportions calculées, avec
  repères commutables — fond perdu `3,2 mm` (pointillés rouges), lignes de pliure (bleues), zone de
  sécurité (verte), réserve code-barres (blanche).
- **Visualisation orbitale 3D** (`Cover3DPreview.tsx`) : livre fermé **CSS 3D** (`transform-style:
  preserve-3d`, `perspective`), rotation horizontale (−60°…+60°) et verticale (−15°…+15°) au
  glisser-déposer, texture/couleur de tranche extraite + texte tourné, ombre portée douce. Bascule
  `[ Planche 2D ]` / `[ Modèle 3D ]` dans la barre d'outils.
- **Dialogue de pré-export** (`CoverPreExportModal.tsx`) : contrôles automatiques (DPI, fond perdu,
  épaisseur de tranche, réserve code-barres, texte de tranche) + validations manuelles de l'auteur,
  puis boîte de dialogue native d'enregistrement.

### 24.2 Commandes Tauri (`src-tauri/src/cover/`)
| Commande | Rôle |
|---|---|
| `calculate_cover_geometry` | Géométrie complète du gabarit (tranche, dimensions, code-barres) |
| `inspect_cover_images` | Dimensions / canal alpha / DPI des deux plats |
| `extract_spine_color` | Couleur unie ou dégradé suggéré pour la tranche |
| `export_kdp_cover_pdf` | Rendu + écriture du PDF 300 DPI |
| `pick_cover_image` / `pick_cover_output_path` | Sélecteurs natifs (Tauri v2) |

Modules Rust : `geometry.rs` (moteur géométrique), `inspector.rs` (métadonnées images),
`color_extractor.rs` (couleur de tranche), `typst_generator.rs` (balisage + compilation).
Tests unitaires préfixés `cover::` (`cargo test cover`).

### 24.3 Modèle de données (persistance `.danoe`)
Champ optionnel **`cover?: CoverStudioState`** intégré au fichier projet (JSON v1.0) : `frontPath`,
`backPath`, `pageCount`, `paperType`, `spineText`, `spineColorFront`, `spineColorBack`, `gradient`,
`pageCountSnapshot`, `structureSignature`. La géométrie et les rapports d'inspection sont **recalculés
à la volée** (non persistés).

### 24.4 Détection de dérive (§10)
Si la **pagination** ou la **signature de structure** diffère du snapshot enregistré lors de la
validation de la tranche, une bannière d'avertissement propose **[ Recalculer la tranche ]**.

---

*Fin du document. Il décrit fidèlement l'implémentation du dépôt `DanoeStudioII` à sa dernière
validation (`tsc` 0 erreur, `vite build` OK, `cargo test` 105/105, `clippy -D warnings` 0 warning,
`tauri build` OK). Toute évolution doit préserver : la règle « configuration d'export ≠ interface »,
l'absence de pop-up, les conventions §5, et la **Tolérance Zéro** (aucun échec silencieux — §20).*













---

## 21. Module Correcteur linguistique (LanguageTool)

Rendu par `CorrectorView.tsx` (onglet **Correcteur**) : éditeur décoré + volet latéral.

### 21.1 Moteur d'analyse (`corrector/client.rs`)
- **Découpage en blocs sûrs** : `MAX_CHUNK_BYTES = 8 000` octets ; coupe préférentielle
  `\n\n` → `\n` → espace → frontière de caractère (`floor_char_boundary`) : **jamais**
  au milieu d'un mot ni d'un caractère UTF-8.
- **Requêtes concurrentes** : `futures::future::join_all` sur les tronçons ; chaque
  correspondance est réindexée (`entry.offset += chunk_start`, `chunk_start` cumulé en
  **caractères**) puis fusionnée dans l'ordre du texte.
- **Tolérance aux pannes partielles** : un tronçon en échec (HTTP 500, timeout) est journalisé
  (`avertissement : tronçon du correcteur ignoré (…)`) sans interrompre les autres ; un échec
  **total** remonte en `Result::Err` (Tolérance Zéro).
- `disabledRules` nettoyés : `trim`, jeu de caractères `[A-Za-z0-9_.-]`, chaînes vides éliminées.

### 21.2 Masquage Markdown non destructif (`mask_markdown`)
Avant l'envoi, la syntaxe est remplacée par des **espaces de même longueur en caractères**
(offsets LanguageTool strictement préservés) :
- frontmatter **YAML Obsidian** de tête (`--- … ---`) ;
- appels de notes `[^label]` ;
- code inline `` `…` `` et texte barré `~~…~~`.

### 21.3 Dictionnaires persistants (`app_data_dir`)
| Fichier | Contenu | Commandes |
|---|---|---|
| `ignored_words.json` | Mots ignorés (toutes règles) | `list_ignored_words`, `update_ignored_words` |
| `places.json` | Toponymes / noms propres (règles **orthographiques** uniquement) | `list_places`, `add_place` |
| `corrector_options.json` | `language`, `picky`, `disabled_rules` | `get_corrector_options`, `set_corrector_options` |

Écriture **atomique** (`.tmp` + `rename`) ; `analyze_chapter` écarte les mots ignorés, les
alertes orthographiques visant un toponyme connu et les règles désactivées.

### 21.4 Ergonomie
- **`rule_description`** (libellé humain) remplace l'ID technique : `title` du bouton « Règle »
  = `Désactiver la règle : « … »` (jamais `rule_id` à l'écran).
- **Bouton « Localiser »** : `ChapterEditorHandle::locate(index)` → `scrollTo` du textarea
  (centrage vertical sur `mark.offsetTop`), synchronisation du calque miroir,
  `setSelectionRange(offset, offset + length)`, surlignage rouge vif
  (`bg-red-500/35 ring-2 ring-red-500/70`).
- **Journal des opérations** (accordéon sous l'en-tête) : corrections de session
  (`ancien terme` barré → terme corrigé, `HH:MM`) ; mention italique si vide.
- **Disparition animée rétro** : la carte validée s'évanouit (`opacity-0`, `-translate-x-2`,
  `scale-95`, repli `max-h-0`, teinte `bg-copper/10`, `transition-all 300 ms ease-out`) puis
  est retirée après ~380 ms.
- **Sauvegarde synchronisée** : chaque remplacement écrit le chapitre (`write_chapter_file`,
  atomique) ; indicateur « Sauvegardé » (~2 s).

---

## 22. Cinématique 3D de fermeture & effet Glow

### 22.1 Interception native (`lib.rs`)
`WindowEvent::CloseRequested` → `api.prevent_close()` + émission **`app-close-requested`**
(la fenêtre n'est jamais fermée brutalement). L'action permanente **« Quitter l'atelier »**
(tranche gauche, `LeftPage` → prop `onExitApp` → `handleQuit` dans `App.tsx`) et la croix
native déclenchent la même cinématique (`isClosing = true`, `src/utils/shutdown.ts`).
L'écran d'accueil (`WelcomeCover`) ne conserve qu'**une** action : **« Entrer dans l'atelier › »**.

### 22.2 Animation 3D (`Layout.tsx`)
- Perspective générale **1400 px** (`[perspective:1400px]`).
- Volet droit : `transform-origin: left center`, `transform-style: preserve-3d`,
  `rotateY(-180deg)`, transition **1000 ms** `cubic-bezier(0.22, 1, 0.36, 1)`.
- **Structure double-face** — deux calques superposés, chacun `backface-visibility: hidden` :
  - **Recto (face interne)** : le livre virtuel (`MenuFlipBook`).
  - **Verso (couverture externe)** : `absolute inset-0`, `transform: rotateY(180deg)` — fond cuir
    patiné `bg-[#1c140e]`, bordure laiton (`border-2 border-brass/50 rounded-r-md m-2`), filet
    intérieur estampé à chaud + **coins renforcés**, **sceau central doré « D »** avec mention
    discrète « Danoë Studio », ombrage de pliure `shadow-[inset_20px_0_30px_rgba(0,0,0,0.6)]`.

### 22.3 Effet Glow
- **Halo d'ambiance** : calque radial `rgba(198, 134, 66, 0.4) → transparent 70 %`,
  `blur-3xl`, opacité 0 → 1 (`duration-500`).
- **Lueur de reliure** : bande centrale `w-[3px]` portant
  `box-shadow: 0 0 25px 6px rgba(217, 119, 6, 0.6)`, apparition synchronisée avec la rotation.

### 22.4 Destruction propre (`commands.rs`)
`finalize_exit` (≈ 1150 ms après le début de la cinématique) : `window.destroy()` — libération
ordonnée des threads WebView2, évitant l'erreur Chromium **Win32 1412** — puis `app.exit(0)`.
Purge du cache volatil assurée sur `RunEvent::Exit` (§14/§17).

