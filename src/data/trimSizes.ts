import type { TrimUnit } from "../types";

export interface TrimPreset {
  id: string;
  width: number;
  height: number;
  /** Unité de référence du format. */
  unit: TrimUnit;
  /** Libellé principal (pouces, ou nom du format). */
  label: string;
  /** Correspondance en centimètres. */
  cm: string;
  note: string;
}

// Formats de coupe KDP officiels, avec équivalences centimétriques.
export const TRIM_PRESETS: TrimPreset[] = [
  {
    id: "5x8",
    width: 5,
    height: 8,
    unit: "in",
    label: "5 × 8 po",
    cm: "environ 12,7 × 20,3 cm",
    note: "Format de poche / pocket.",
  },
  {
    id: "5.25x8",
    width: 5.25,
    height: 8,
    unit: "in",
    label: "5,25 × 8 po",
    cm: "environ 13,3 × 20,3 cm",
    note: "Format de poche allongé.",
  },
  {
    id: "5.5x8.5",
    width: 5.5,
    height: 8.5,
    unit: "in",
    label: "5,50 × 8,50 po",
    cm: "environ 14 × 21,6 cm",
    note: "Format roman compact.",
  },
  {
    id: "6x9",
    width: 6,
    height: 9,
    unit: "in",
    label: "6 × 9 po",
    cm: "environ 15,2 × 22,9 cm",
    note: "Format standard dominant pour les romans.",
  },
  {
    id: "7x10",
    width: 7,
    height: 10,
    unit: "in",
    label: "7 × 10 po",
    cm: "environ 17,8 × 25,4 cm",
    note: "Format roman grand, confort de lecture.",
  },
  {
    id: "8x10",
    width: 8,
    height: 10,
    unit: "in",
    label: "8 × 10 po",
    cm: "environ 20,3 × 25,4 cm",
    note: "Format illustré / beau-livre.",
  },
  {
    id: "a4",
    width: 210,
    height: 297,
    unit: "mm",
    label: "A4",
    cm: "21,0 × 29,7 cm",
    note: "Format standard international pour documents et manuels techniques.",
  },
];

/** Recherche un format KDP par identifiant. */
export const findTrimPreset = (id: string): TrimPreset | undefined =>
  TRIM_PRESETS.find((preset) => preset.id === id);

/** Libellé complet d'un format (dimensions en pouces + correspondance cm). */
export const trimLabelFor = (id: string): string => {
  const preset = findTrimPreset(id);
  return preset ? `${preset.label} (${preset.cm})` : id;
};

/** Sous-titre compact d'un format, ex. « 6 × 9 po · 15,2 × 22,9 cm ». */
export const trimSubtitleFor = (id: string): string => {
  const preset = findTrimPreset(id);
  if (!preset) {
    return id;
  }
  const cm = preset.cm.replace(/^environ\s*/i, "");
  return `${preset.label} · ${cm}`;
};

