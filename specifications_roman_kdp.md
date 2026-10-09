# Spécifications Fonctionnelles et Techniques : Logiciel d'Édition Littéraire KDP & Vibe Coding

## 1. Introduction et Objectifs du Projet

Le projet consiste à développer un logiciel moderne conçu en "vibe coding" permettant de transformer des chapitres rédigés en Markdown ou texte brut (`.txt`), accompagnés d'illustrations, en un livre professionnel prêt pour l'édition (aux normes strictes d'**Amazon KDP** pour le format broché et le format Ebook).

L'interface utilisateur doit s'éloigner des environnements bureautiques traditionnels pour offrir une expérience immersive de "livre ouvert" interactive.

## 2. Spécifications Front-End (Interface Utilisateur & Expérience)

### 2.1. Concept Visuel & Ambiance

* **Le "Livre Ouvert" (Manuscript View)** : L'application simule un livre physique ouvert sur un bureau.
* **Navigation 3D (Page-Turning)** : Le passage d'une section à une autre déclenche une animation réaliste de page qui se tourne (pliure, ombres portées, fluidité).
* **Typographie & Thèmes** : Polices serif littéraires pour le contenu (Lora, EB Garamond), modes visuels adaptés (Papier ancien, Nuit d'écriture).

### 2.2. Architecture des Vues & Navigation

1. **Tableau de Bord / Couverture** : Gestion du projet et métadonnées générales.
2. **Table des Matières Interactive** : Arbre de navigation par glisser-déposer (Drag & Drop) pour réorganiser les chapitres.
3. **Double Page d'Édition (Split-View)** :
   * Page de gauche : Éditeur Markdown épuré.
   * Page de droite : Prévisualisation typographique en temps réel.
4. **Panneau de Paramétrage & Exportation** : Console de configuration des styles et déclenchement des exports.

## 3. Workflow de Création en 5 Étapes

### Étape 1 : Importation des Sources (Chapitres & Illustrations)

* **Zone de dépôt (Dropzone)** : Import par glisser-déposer de fichiers `.md` ou `.txt`.
* **Galerie d'actifs** : Import d'images (`.png`, `.jpg`) avec génération automatique de snippets Markdown d'insertion.
* **Contrôles préliminaires** : Nettoyage typographique automatique (guillemets français `« »`, apostrophes incurvées) et vérification de la résolution des illustrations (exigence minimale de 300 DPI pour KDP Broché).

### Étape 2 : Organisation de la Structure Narrative

* Structuration rigoureuse du roman :
  * **Pages Préliminaires (Front Matter)** : Faux-titre, page de titre, copyright, dédicace, sommaire.
  * **Corps du texte (Body Matter)** : Regroupement par Actes / Parties et chapitres numérotés.
  * **Pages Finales (Back Matter)** : Remerciements, biographie, aperçu d'œuvres.
* **Contrainte KDP** : Forçage automatique pour que chaque début de grande section ou de chapitre commence obligatoirement sur une page impaire (recto).

### Étape 3 : Renseignements & Métadonnées du Roman

Le logiciel doit fournir un formulaire dédié pour collecter toutes les informations nécessaires à la génération automatique des pages préliminaires (notamment la page de copyright) et des balises techniques (EPUB/Word).
Les champs requis sont :
* **Le titre complet**
* **Le sous-titre** (optionnel)
* **Le nom de l'auteur** (ou nom de plume)
* **L'année de publication**
* **Le code ISBN**
* **Le nom de l'éditeur** (ou auto-édition)
* **Le dépôt légal** (Mois et Année)
* **La mention de copyright** (ex: *© 2026 Nom. Tous droits réservés.*)
* **Le site web** de l'auteur ou de l'éditeur
* **Le résumé / Quatrième de couverture** (utilisé pour les métadonnées internes du fichier).

### Étape 4 : Paramétrage Typographique & Éditorial KDP

Cette section intègre le moteur de calcul automatisé basé sur les normes officielles Amazon KDP.

* **Taille de coupe (Trim Size)** :
  * Format standard roman : $152,4 \times 228,6 \text{ mm}$ (6 x 9 pouces).
  * *Note du moteur* : Tout format dépassant $155,5 \text{ mm}$ en largeur ou $228,6 \text{ mm}$ en hauteur basculera l'ouvrage dans la catégorie "Grand Format" KDP.
* **Gestion du fond perdu (Bleed)** :
  * En présence d'illustrations allant jusqu'au bord, le logiciel agrandit la zone d'exportation : $+ 3,2 \text{ mm}$ sur le bord extérieur, et $+ 6,4 \text{ mm}$ sur la hauteur totale.
  * *Exemple* : Un $15,24 \times 22,86 \text{ cm}$ (6 x 9 pouces) avec fond perdu sera généré avec des dimensions de page de $15,54 \times 23,46 \text{ cm}$ (6,125 x 9,25 pouces).
* **Marges de reliure (Gutter) dynamiques** :
  * Le logiciel calcule la marge intérieure automatiquement en fonction du nombre total de pages estimé à l'export :
    * 24 à 150 pages : $9,6 \text{ mm}$
    * 151 à 300 pages : $12,7 \text{ mm}$
    * 301 à 500 pages : $15,9 \text{ mm}$
    * 501 à 700 pages : $19,1 \text{ mm}$
    * 701 à 828 pages : $22,3 \text{ mm}$
* **Marges de sécurité (Garde)** :
  * L'interface bloquera les réglages manuels pour respecter les minimas KDP : marges supérieure et inférieure $\geq 6,4 \text{ mm}$, marge extérieure $\geq 9,6 \text{ mm}$.
* **Styles & Typographie** :
  * Corps du texte justifié, taille de police, interlignage (1.15 ou 1.20).
  * **Règles d'alinéa** : Indentation automatique sur tous les paragraphes *sauf* le premier d'un chapitre ou après une coupure de scène (`* * *`).
  * **En-têtes et Pieds de page (Running Heads)** : Alternance dynamique du titre de l'auteur et du titre du livre/chapitre avec masquage automatique sur les pages de titre de chapitre.

### Étape 5 : Module d'Exportation Multi-Formats

Génération en un clic vers trois formats conformes :

1. **Format Word (`.docx`)** : Structuré avec des styles natifs Word et des sauts de section propres (sauts de page pairs/impairs).
2. **Format PDF Prêt-à-imprimer (Print-Ready PDF)** : Verrouillé pour KDP Broché, intégrant automatiquement les calculs de Gutter et de Bleed listés à l'étape 4, avec toutes les polices incorporées (embedded).
3. **Format Ebook (`.epub`)** : EPUB 3 fluide (reflowable) avec table de navigation dédiée (`nav.xhtml` / `toc.ncx`) optimisée pour les liseuses Kindle.

#### Gestion des notes de bas de page & du Glossaire

Le moteur détecte la présence d'une **« Page Spéciale : Glossaire »** dans la structure du livre et adapte le traitement des notes Markdown (`[^x]` = renvoi, `[^x]: …` = définition) :

* **Glossaire absent** : les renvois `[^x]` sont **retirés** du corps du texte et les blocs de définition `[^x]: …` sont **ignorés**.
* **Glossaire présent** : les renvois `[^x]` sont **conservés** avec une numérotation **réinitialisée à chaque chapitre** ; les définitions `[^x]: …` sont **extraites** de tous les fichiers sources et **regroupées par chapitre** dans le Glossaire, sous un sous-titre automatique (ex. « Acte 2, chapitre 7 »).

#### Couverture cuir (page de garde, export Word)

La **première page** du `.docx` reçoit une **image de fond** (`Couverture_Cuir_01.jpg`, ressource Tauri `src-tauri/assets/`) dimensionnée au **format de coupe KDP** retenu, ancrée « **Derrière le texte** » avec marges nulles (bord à bord). Les métadonnées sont superposées en **texte clair `#F5F5DC`** (police Serif) : **titre** (tiers supérieur, ~42 pt, majuscules), **sous-titre** (italique, ~18 pt), **nom de l'auteur** (quart inférieur, ~24 pt) — le centrage compense la **ligne de couture** à gauche. Un **saut de section** ramène ensuite au **fond blanc** et aux **marges de gouttière KDP** standard (faux-titre, pages liminaires).

## 4. Stack Technique Recommandée (Vibe Coding)

* **Front-End** :
  * **React** (via Vite, Next.js ou Remix) pour la structure des composants.
  * **Tailwind CSS** pour le design system, les ombres 3D et les textures "papier".
  * **`react-pageflip`** ou **Framer Motion** pour les animations de rotation de pages 3D.
* **Back-End / Moteur de Conversion** :
  * **Rust** via **Tauri v2** (`src-tauri/`) : empaquetage bureau natif, accès fichiers, et moteur d'export.
  * **`docx-rs`** pour générer les fichiers **Word (`.docx`)** (styles natifs, sections, sauts pairs/impairs, image de couverture ancrée « derrière le texte »).
  * **`serde` / `serde_json`** pour la désérialisation du payload de projet transmis par le frontend.
  * Évolutions ultérieures : **PDF prêt-à-imprimer** et **EPUB 3** (compilation locale / outils dédiés).