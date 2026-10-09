export type ActiveMenu = 'reglages' | 'infos' | 'organisation' | 'correcteur' | 'export';

export type LogLevel = 'info' | 'success' | 'warning' | 'error';

export interface LogEntry {
  id: string;
  time: string;
  level: LogLevel;
  message: string;
}

/** Correction appliquée pendant une session de relecture (journal du correcteur). */
export interface AppliedCorrection {
  id: string;
  original: string;
  replacement: string;
  timestamp: Date;
}

/** Unité de mesure d'un format de coupe. */
export type TrimUnit = 'in' | 'mm';

/**
 * Configuration de mise en page du manuscrit EXPORTÉ (Word / PDF KDP).
 *
 * Portée stricte : ces valeurs ne décrivent que le gabarit du document final
 * généré (métadonnées + règles de mise en page pour les moteurs d'export).
 * Elles ne modifient en aucun cas la typographie de l'interface logicielle
 * ni les styles CSS globaux du studio.
 */
export interface ManuscriptLayoutConfig {
  /** Format physique (trim size), ex. « 6x9 ». */
  trimSize: string;

  // Corps de texte
  /** Police du corps de texte, ex. « Garamond ». */
  bodyFont: string;
  /** Taille du corps : 9 | 10 | 10.5 | 11 | 12 | 13 | 14. */
  bodySize: number;
  /** Interligne : 1 | 1.15 | 1.25 | 1.5 | 2. */
  lineSpacing: number;
  /** Justification : 'left' (gauche) ou 'justify' (justifié). */
  textAlignment: 'left' | 'justify';
  /** Lettrine (grand D initial en tête de chapitre). */
  dropCap: boolean;

  // Titres et sous-titres
  /** Police du titre de chapitre ('body' = identique au corps de texte). */
  chapterTitleFont: string;
  /** Taille du titre de chapitre : 12 | 14 | 16 | 18 | 20. */
  chapterTitleSize: number;
  /** Police des sous-titres ('body' = identique au corps de texte). */
  subtitleFont: string;
  /** Taille des sous-titres : 11 | 12 | 14 | 16. */
  subtitleSize: number;
}

/** Traitement colorimétrique des images pour l'export PDF. */
export type ImageColorMode = 'color' | 'grayscale';

/**
 * Configuration des sources brutes du roman (dossier unique « Mes sources »).
 *
 * Un **dossier unique (Workspace)** centralise l'intégralité des fichiers du
 * projet : chapitres et illustrations y cohabitent et sont répartis
 * automatiquement par le moteur de tri selon leur extension (voir
 * `utils/sources.ts`).
 *
 * Portée « configuration de projet » : ne modifie pas l'interface du studio.
 */
export interface SourcesConfig {
  /** Chemin du dossier unique contenant tous les fichiers (null si non défini). */
  directory: string | null;
  /** Traitement des images pour le PDF (couleur ou niveaux de gris). */
  colorMode: ImageColorMode;
}

/** Niveau de verbosité du journal de diagnostic. */
export type LogDiagnosticLevel = 'standard' | 'diagnostic';

/**
 * Configuration du journal des erreurs.
 * Portée « configuration de projet » : ne modifie pas l'interface du studio.
 */
export interface ErrorLogConfig {
  /** Niveau de diagnostic : 'standard' (alertes) ou 'diagnostic' (tout). */
  level: LogDiagnosticLevel;
  /** Dossier du fichier journal (null → dossier système par défaut). */
  directory: string | null;
}

/**
 * Métadonnées du roman (onglet « Informations »).
 * Portée « configuration de projet » : utilisées pour la génération du livre
 * (page de copyright, métadonnées EPUB/Word). Ne modifient pas l'interface.
 */
export interface BookInfoConfig {
  // --- Section A : Identité de l'œuvre ---
  /** Titre du livre (obligatoire). */
  title: string;
  /** Sous-titre. */
  subtitle: string;
  /** Nom de la série / saga (optionnel). */
  sagaTitle: string;
  /** Numéro de tome (champ numérique, conservé en texte). */
  volumeNumber: string;

  // --- Section B : Auteurs et contributeurs ---
  /** Nom de l'auteur principal. */
  author: string;
  /** Traducteur ou illustrateur (optionnel). */
  contributor: string;

  // --- Section C : Édition et mentions légales ---
  /** Nom de l'éditeur (vide → « Autoédition »). */
  publisher: string;
  /** Code ISBN. */
  isbn: string;
  /** Date de parution (ex. « Octobre 2026 »). */
  year: string;
  /** Lieu d'impression (achevé d'imprimer — conformité légale). */
  printLocation: string;

  // --- Section D : Éléments annexes ---
  /** Autres œuvres du même auteur (page « Du même auteur » si renseigné). */
  otherBooks: string;

  // --- Champs hérités (conservés pour compatibilité de persistance) ---
  /** Mention de copyright (vide → standard légal calculé à l'export). */
  copyright: string;
  /** Site web de l'auteur. */
  website: string;
}

/** Type d'élément de la structure du roman (onglet Organisation). */
export type StructureItemType = 'act' | 'chapter' | 'image' | 'special';

/**
 * Rôle éditorial d'une **page spéciale** (page modulaire rédigée par l'auteur).
 * Le rôle oriente le moteur de compilation (mise en forme, saut de page impaire…).
 */
export type SpecialPageRole =
  | 'dedication'
  | 'epigraph'
  | 'prologue'
  | 'epilogue'
  | 'authorNote'
  | 'acknowledgements'
  | 'glossary';

/**
 * Élément de la structure du roman (« chemin de fer »).
 *
 * Le tableau `StructureItem[]` est **plat et ordonné** ; la hiérarchie est
 * déduite de l'ordre : les éléments qui suivent un `act` lui appartiennent
 * (visuellement indentés) jusqu'au prochain `act`.
 */
export interface StructureItem {
  /** Identifiant unique. */
  id: string;
  /** Nature de l'élément. */
  type: StructureItemType;
  /** Nom affiché (nom de l'acte, ou nom renommé du chapitre). */
  displayName?: string;
  /** Nom du fichier source lié (chapitres, images et pages spéciales). */
  sourceFileName?: string;
  /** Rôle éditorial (uniquement renseigné pour les pages spéciales). */
  role?: SpecialPageRole;
}

/** Nœud hiérarchique d'un élément de structure (forme sérialisée). */
export interface StructureNode {
  id: string;
  type: StructureItemType;
  displayName?: string;
  sourceFileName?: string;
  /** Rôle éditorial (pages spéciales). */
  role?: SpecialPageRole;
  /** Enfants (chapitres, images et pages spéciales placés sous un Acte). */
  children?: StructureNode[];
}

/** Métadonnées du roman telles que sérialisées dans le fichier de projet. */
export interface ProjectMetadata {
  sagaTitle: string;
  bookTitle: string;
  subtitle: string;
  /** Numéro de tome (absent des anciens fichiers). */
  volumeNumber?: string;
  authorName: string;
  /** Traducteur ou illustrateur (absent des anciens fichiers). */
  contributor?: string;
  year: string;
  isbn: string;
  publisher: string;
  /** Lieu d'impression (absent des anciens fichiers). */
  printLocation?: string;
  copyrightText: string;
  website: string;
  otherBooks: string;
}

/** Dossiers de travail du projet. */
export interface ProjectDirectories {
  /** Dossier unique « Mes sources » (workspace — tous les fichiers). */
  sources: string | null;
  logs: string | null;
  /** (obsolète) Ancien dossier des chapitres — lu uniquement pour migration. */
  chapters?: string | null;
  /** (obsolète) Ancien dossier des images — lu uniquement pour migration. */
  images?: string | null;
}

/** Options annexes du projet. */
export interface ProjectOptions {
  imageColorMode: ImageColorMode;
  logLevel: LogDiagnosticLevel;
}

/**
 * Racine du **fichier de projet** (JSON, extension `.danoe`).
 * Sérialise l'intégralité du store de l'application (métadonnées, réglages KDP,
 * dossiers, options et structure/organisation).
 */
export interface ProjectFile {
  /** Version du format de fichier. */
  projectVersion: string;
  metadata: ProjectMetadata;
  layoutConfig: ManuscriptLayoutConfig;
  directories: ProjectDirectories;
  options: ProjectOptions;
  organization: StructureNode[];
  /**
   * Cache des fichiers détectés (extension) : permet de repeupler les menus
   * sources au rechargement lorsqu'on est hors Tauri (où l'énumération d'un
   * dossier n'est pas possible après coup).
   */
  fileCache?: {
    chapters: string[];
    images: string[];
  };
}
