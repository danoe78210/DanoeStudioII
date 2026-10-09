export interface FontChoice {
  /** Nom de la famille. */
  name: string;
  /** Pile CSS utilisée pour l'aperçu visuel. */
  stack: string;
}

/** Texte d'aperçu affiché dans chaque police. */
export const SAMPLE_TEXT = "Les Schattenjägers — Danoë Studio";

// Polices professionnelles et littéraires.
export const FONT_FAMILIES: FontChoice[] = [
  { name: "Aptos", stack: "Aptos, Calibri, sans-serif" },
  { name: "Cinzel", stack: "Cinzel, 'Times New Roman', serif" },
  {
    name: "Garamond",
    stack: "Garamond, 'EB Garamond', 'Times New Roman', serif",
  },
  { name: "Times New Roman", stack: "'Times New Roman', Times, serif" },
  { name: "Georgia", stack: "Georgia, serif" },
  { name: "Calibri", stack: "Calibri, Candara, sans-serif" },
  { name: "Arial", stack: "Arial, Helvetica, sans-serif" },
  {
    name: "Book Antiqua",
    stack: "'Book Antiqua', 'Palatino Linotype', Palatino, serif",
  },
  { name: "Cambria", stack: "Cambria, Georgia, serif" },
];

// Tailles normalisées.
export const BODY_FONT_SIZES: number[] = [9, 10, 10.5, 11, 12, 13, 14];
export const CHAPTER_TITLE_SIZES: number[] = [12, 14, 16, 18, 20];
export const SUBTITLE_SIZES: number[] = [11, 12, 14, 16];

// Interlignes sélectionnables.
export const LINE_SPACINGS: number[] = [1, 1.15, 1.25, 1.5, 2];

// Valeur sentinelle pour « identique au corps de texte ».
export const INHERIT_FONT = "body";
export const INHERIT_LABEL = "Identique au corps du texte";

/** Libellé d'une police (gère la valeur « identique au corps »). */
export const fontLabel = (font: string): string =>
  font === INHERIT_FONT ? INHERIT_LABEL : font;

/** Résout la police effective (remplace l'héritage par la police du corps). */
export const resolveFont = (font: string, bodyFont: string): string =>
  font === INHERIT_FONT ? bodyFont : font;

/** Formatage d'un nombre à la française (ex. 1.15 → « 1,15 »). */
export const numberFr = (value: number): string =>
  String(value).replace(".", ",");

