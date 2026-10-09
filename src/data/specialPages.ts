import type { SpecialPageRole } from "../types";

export interface SpecialPageRoleOption {
  id: SpecialPageRole;
  label: string;
}

/**
 * Liste **stricte** des rôles de page spéciale (ordre d'affichage imposé) :
 * Dédicace / Épigraphe / Prologue / Épilogue / Note de l'auteur / Remerciements / Glossaire.
 */
export const SPECIAL_PAGE_ROLES: SpecialPageRoleOption[] = [
  { id: "dedication", label: "Dédicace" },
  { id: "epigraph", label: "Épigraphe" },
  { id: "prologue", label: "Prologue" },
  { id: "epilogue", label: "Épilogue" },
  { id: "authorNote", label: "Note de l'auteur" },
  { id: "acknowledgements", label: "Remerciements" },
  { id: "glossary", label: "Glossaire" },
];

/** Rôle appliqué par défaut lors de la création d'une page spéciale. */
export const DEFAULT_SPECIAL_ROLE: SpecialPageRole = "dedication";

/**
 * Rôles dont le rendu n'admet **pas** de titre dans le livre (champ grisé) :
 * une dédicace ou une épigraphe n'a pas de titre de chapitre.
 */
export const TITLELESS_SPECIAL_ROLES: ReadonlySet<SpecialPageRole> = new Set([
  "dedication",
  "epigraph",
]);

/** Indique si un rôle admet un titre affiché dans le livre. */
export const specialRoleHasTitle = (role: SpecialPageRole): boolean =>
  !TITLELESS_SPECIAL_ROLES.has(role);

/**
 * Rôles dont la **source n'est pas requise** : le **Glossaire** est généré
 * dynamiquement par le moteur à partir des notes de bas de page des chapitres.
 * Pour ces rôles, ni sélecteur de fichier ni bouton « Importer » ne sont affichés.
 */
export const SOURCELESS_SPECIAL_ROLES: ReadonlySet<SpecialPageRole> = new Set([
  "glossary",
]);

/** Indique si un rôle requiert un fichier source à lier/importer. */
export const specialRoleHasSource = (role: SpecialPageRole): boolean =>
  !SOURCELESS_SPECIAL_ROLES.has(role);

/** Libellé lisible d'un rôle (repli sur l'identifiant brut). */
export const specialRoleLabel = (role: SpecialPageRole): string =>
  SPECIAL_PAGE_ROLES.find((option) => option.id === role)?.label ?? role;
