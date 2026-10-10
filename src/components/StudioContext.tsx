import { createContext, useContext } from "react";
import type {
  BookInfoConfig,
  ErrorLogConfig,
  ImageColorMode,
  LogDiagnosticLevel,
  ManuscriptLayoutConfig,
  SourcesConfig,
  SpecialPageRole,
  StructureItem,
} from "../types";
import type { FsDirectoryHandle } from "../utils/directory";

/**
 * État applicatif exposé aux **pages du livre virtuel** (vue de droite).
 *
 * Les pages de menus sont rendues dans `react-pageflip`, dont le contenu ne doit
 * pas changer d'identité à chaque rendu (sinon le livre se ré-initialise et
 * l'animation de page tournée est interrompue). En lisant l'état via ce contexte,
 * les pages conservent une **identité stable** tout en restant **vivantes**.
 */
export interface StudioContextValue {
  // --- Réglages ---
  manuscriptConfig: ManuscriptLayoutConfig;
  sourceConfig: SourcesConfig;
  errorLogConfig: ErrorLogConfig;
  onTrimChange: (presetId: string) => void;
  onLayoutChange: (patch: Partial<ManuscriptLayoutConfig>) => void;
  onSourcesDirectoryChange: (
    path: string,
    files: string[],
    handle: FsDirectoryHandle | null,
  ) => void;
  onImageColorModeChange: (mode: ImageColorMode) => void;
  onLogLevelChange: (level: LogDiagnosticLevel) => void;
  onLogDirectoryChange: (path: string) => void;
  onOpenLog: () => void;

  // --- Informations ---
  bookInfo: BookInfoConfig;
  onInfoFieldChange: (
    section: string,
    label: string,
    field: keyof BookInfoConfig,
    value: string,
  ) => void;

  // --- Organisation ---
  structure: StructureItem[];
  chapterFiles: string[];
  imageFiles: string[];
  sourcesConfigured: boolean;
  onAddAct: () => void;
  onAddChapter: () => void;
  onAddImage: () => void;
  onAddSpecial: () => void;
  onRemoveStructureItem: (id: string) => void;
  onRenameStructureItem: (id: string, value: string) => void;
  onSpecialRoleChange: (id: string, role: SpecialPageRole) => void;
  onStructureSourceChange: (id: string, fileName: string) => void;
  onStructureReorder: (items: StructureItem[]) => void;
  onPickSourcesDirectory: () => void;
  onImportIntoFolder: (id: string) => void;

  // --- Export ---
  onExport: (format: string) => void;
  /** Ouvre l'aperçu interactif (flipbook) du manuscrit rendu par Typst. */
  onPreview: () => void;
  isExporting: boolean;

  // --- Application ---
  /** Ferme proprement l'application (cinématique 3D du livre + purge des caches). */
  onQuit: () => void;
}

const StudioContext = createContext<StudioContextValue | null>(null);

/** Fournit l'état applicatif aux pages du livre virtuel. */
export const StudioProvider = StudioContext.Provider;

/** Accède à l'état applicatif (hors du fournisseur, lève une erreur explicite). */
export const useStudio = (): StudioContextValue => {
  const value = useContext(StudioContext);
  if (!value) {
    throw new Error("useStudio doit être utilisé dans un StudioProvider.");
  }
  return value;
};
