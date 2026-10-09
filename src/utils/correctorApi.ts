import { invokeCommand } from "./tauri";

/**
 * Correspondance renvoyée par le backend (`analyze_chapter`) — miroir de la
 * struct `corrector::client::CorrectionMatch`.
 */
export interface CorrectionMatch {
  /** Décalage du fragment fautif (en **caractères**). */
  offset: number;
  /** Longueur du fragment fautif (en **caractères**). */
  length: number;
  /** Message explicatif de la règle. */
  message: string;
  /** Remplacements suggérés. */
  replacements: string[];
  /** Phrase de contexte (`null` si absente). */
  context: string | null;
  /** Identifiant de la règle (`null` si absent) — usage interne uniquement. */
  rule_id: string | null;
  /** Libellé **humain** de la règle (`null` si absent). */
  rule_description: string | null;
}

/** Analyse un texte brut (correcteur LanguageTool + filtre du dictionnaire local). */
export const analyzeChapter = (text: string): Promise<CorrectionMatch[]> =>
  invokeCommand<CorrectionMatch[]>("analyze_chapter", { text });

/** Liste les mots ignorés persistés (dictionnaire local). */
export const listIgnoredWords = (): Promise<string[]> =>
  invokeCommand<string[]>("list_ignored_words");

/** Remplace le dictionnaire local par la liste fournie (mots ignorés). */
export const updateIgnoredWords = (words: string[]): Promise<void> =>
  invokeCommand<void>("update_ignored_words", { words });

/** Liste les **toponymes / noms propres** persistés (`list_places`). */
export const listPlaces = (): Promise<string[]> =>
  invokeCommand<string[]>("list_places");

/**
 * Ajoute un **toponyme / nom propre** (`add_place`) : les alertes
 * orthographiques le visant sont ensuite écartées par le backend.
 */
export const addPlace = (word: string): Promise<string[]> =>
  invokeCommand<string[]>("add_place", { word });

/** Options d'analyse du correcteur — miroir de `corrector::options::LtOptions`. */
export interface LtOptions {
  /** Code de langue / région (`fr`, `fr-FR`, `fr-BE`, `fr-CA`, `fr-CH`). */
  language: string;
  /** Mode « pointilleux » (style, typographie, sémantique avancée). */
  picky: boolean;
  /** IDs de règles désactivées. */
  disabled_rules: string[];
}

/** Charge les options persistées (`get_corrector_options`). */
export const getCorrectorOptions = (): Promise<LtOptions> =>
  invokeCommand<LtOptions>("get_corrector_options");

/** Remplace les options persistées (`set_corrector_options`). */
export const setCorrectorOptions = (options: LtOptions): Promise<void> =>
  invokeCommand<void>("set_corrector_options", { options });
