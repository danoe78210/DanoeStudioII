import type {
  BookInfoConfig,
  ErrorLogConfig,
  ManuscriptLayoutConfig,
  ProjectFile,
  SourcesConfig,
  StructureItem,
  StructureNode,
} from "../types";
import { invokeCommand, isInvokeAvailable } from "./tauri";

/** Version **courante** du format de fichier de projet. */
export const PROJECT_VERSION = "1.0";

/** Clé de repli navigateur (Tauri indisponible). */
const STORAGE_KEY = "danoe-studio-project";

/** Compare deux versions « majeure.mineure » (négatif / 0 / positif). */
const compareVersions = (a: string, b: string): number => {
  const left = a.split(".").map((part) => Number.parseInt(part, 10) || 0);
  const right = b.split(".").map((part) => Number.parseInt(part, 10) || 0);
  const length = Math.max(left.length, right.length);
  for (let index = 0; index < length; index += 1) {
    const l = left[index] ?? 0;
    const r = right[index] ?? 0;
    if (l !== r) {
      return l - r;
    }
  }
  return 0;
};

/**
 * **Migrations successives** du format de projet.
 *
 * Chaque étape cible une version et transforme les données héritées ; elles sont
 * appliquées dans l'ordre tant que la version du fichier est inférieure à la
 * version cible. **Aucune donnée n'est supprimée** : les nouveaux champs sont
 * ajoutés, les anciens conservés.
 *
 * Pour une future évolution :
 * ```ts
 * { version: "1.1", migrate: (data) => ({ ...data, newOption: "default" }) }
 * ```
 */
const PROJECT_MIGRATIONS: {
  version: string;
  migrate: (data: Partial<ProjectFile>) => Partial<ProjectFile>;
}[] = [
  // Aucune migration nécessaire entre 0.x et 1.0 : les champs absents sont comblés
  // par les valeurs par défaut au chargement (`{ ...DEFAULT, ...data }`).
];

/**
 * Applique les migrations de format puis **estampille la version courante**.
 * Les données existantes sont préservées.
 */
export const migrateProject = (
  data: Partial<ProjectFile>,
): Partial<ProjectFile> => {
  let current = data;
  let version =
    typeof current.projectVersion === "string" ? current.projectVersion : "0.0";

  for (const step of PROJECT_MIGRATIONS) {
    if (compareVersions(version, step.version) < 0) {
      current = step.migrate(current);
      version = step.version;
    }
  }

  return { ...current, projectVersion: PROJECT_VERSION };
};

/** Nom de fichier de sauvegarde proposé (JSON texte). */
export const PROJECT_FILE_NAME = "danoe-studio.danoe";

/** Transforme la structure plate ordonnée en arbre hiérarchique. */
export const flatToTree = (items: StructureItem[]): StructureNode[] => {
  const roots: StructureNode[] = [];
  let currentAct: StructureNode | null = null;

  for (const item of items) {
    if (item.type === "act") {
      currentAct = {
        id: item.id,
        type: "act",
        displayName: item.displayName,
        children: [],
      };
      roots.push(currentAct);
    } else {
      const node: StructureNode = {
        id: item.id,
        type: item.type,
        displayName: item.displayName,
        sourceFileName: item.sourceFileName,
        role: item.role,
      };
      if (currentAct) {
        currentAct.children = [...(currentAct.children ?? []), node];
      } else {
        roots.push(node);
      }
    }
  }

  return roots;
};

/** Aplatit l'arbre hiérarchique en structure plate ordonnée. */
export const treeToFlat = (nodes: StructureNode[]): StructureItem[] => {
  const flat: StructureItem[] = [];
  for (const node of nodes) {
    flat.push({
      id: node.id,
      type: node.type,
      displayName: node.displayName,
      sourceFileName: node.sourceFileName,
      role: node.role,
    });
    if (node.children && node.children.length > 0) {
      flat.push(...treeToFlat(node.children));
    }
  }
  return flat;
};

/** État complet à sérialiser. */
export interface ProjectState {
  bookInfo: BookInfoConfig;
  manuscriptConfig: ManuscriptLayoutConfig;
  sourceConfig: SourcesConfig;
  errorLogConfig: ErrorLogConfig;
  structure: StructureItem[];
  /** Fichiers chapitres détectés dans le dossier « Mes sources » (cache). */
  chapterFiles: string[];
  /** Fichiers images détectés dans le dossier « Mes sources » (cache). */
  imageFiles: string[];
}

/** Compile l'arbre de données du fichier de projet. */
export const buildProjectFile = (state: ProjectState): ProjectFile => ({
  projectVersion: PROJECT_VERSION,
  metadata: {
    sagaTitle: state.bookInfo.sagaTitle,
    bookTitle: state.bookInfo.title,
    subtitle: state.bookInfo.subtitle,
    volumeNumber: state.bookInfo.volumeNumber,
    authorName: state.bookInfo.author,
    contributor: state.bookInfo.contributor,
    year: state.bookInfo.year,
    isbn: state.bookInfo.isbn,
    publisher: state.bookInfo.publisher,
    printLocation: state.bookInfo.printLocation,
    copyrightText: state.bookInfo.copyright,
    website: state.bookInfo.website,
    otherBooks: state.bookInfo.otherBooks,
  },
  layoutConfig: state.manuscriptConfig,
  directories: {
    sources: state.sourceConfig.directory,
    logs: state.errorLogConfig.directory,
  },
  options: {
    imageColorMode: state.sourceConfig.colorMode,
    logLevel: state.errorLogConfig.level,
  },
  organization: flatToTree(state.structure),
  fileCache: {
    chapters: state.chapterFiles,
    images: state.imageFiles,
  },
});

/**
 * Analyse le texte JSON d'un fichier de projet (retourne `null` si invalide) puis
 * applique les **migrations de format** afin de préserver les données héritées.
 */
export const parseProjectJson = (text: string): Partial<ProjectFile> | null => {
  try {
    const data = JSON.parse(text) as Partial<ProjectFile>;
    if (typeof data !== "object" || data === null) {
      return null;
    }
    return migrateProject(data);
  } catch {
    return null;
  }
};

interface TauriFsApi {
  writeTextFile: (path: string, contents: string) => Promise<void>;
  readTextFile: (path: string) => Promise<string>;
}

/** Méthode utilisée lors de la sauvegarde. */
export type SaveMethod = "file" | "storage" | "error";

/**
 * Sauvegarde asynchrone du projet, par ordre de préférence :
 * 1. **stockage persistant Tauri** (`app_data_dir/project.danoe`) — survit aux
 *    mises à jour et n'est jamais purgé ;
 * 2. fichier choisi par l'utilisateur (Tauri `fs.writeTextFile`) ;
 * 3. repli navigateur (`localStorage`).
 */
export const saveProject = async (
  json: string,
  filePath: string | null,
): Promise<SaveMethod> => {
  if (typeof window === "undefined") {
    return "error";
  }

  let saved = false;

  // 1) Copie **persistante** (dossier de données utilisateur).
  if (isInvokeAvailable()) {
    try {
      await invokeCommand("write_project_data", { contents: json });
      saved = true;
    } catch {
      // repli ci-dessous.
    }
  }

  // 2) Fichier choisi par l'utilisateur (s'il y en a un).
  const tauri = (window as unknown as { __TAURI__?: { fs?: TauriFsApi } })
    .__TAURI__;
  if (filePath && tauri?.fs?.writeTextFile) {
    try {
      await tauri.fs.writeTextFile(filePath, json);
      saved = true;
    } catch {
      // repli ci-dessous.
    }
  }

  if (saved) {
    return "file";
  }

  // 3) Repli navigateur.
  try {
    window.localStorage.setItem(STORAGE_KEY, json);
    return "storage";
  } catch {
    return "error";
  }
};

/**
 * Charge le projet, par ordre de préférence :
 * 1. fichier choisi par l'utilisateur (Tauri `fs.readTextFile`) ;
 * 2. **stockage persistant Tauri** (`app_data_dir/project.danoe`) ;
 * 3. repli navigateur (`localStorage`).
 * Retourne le texte JSON, ou `null`.
 */
export const loadProject = async (
  filePath: string | null,
): Promise<string | null> => {
  if (typeof window === "undefined") {
    return null;
  }

  const tauri = (window as unknown as { __TAURI__?: { fs?: TauriFsApi } })
    .__TAURI__;
  if (filePath && tauri?.fs?.readTextFile) {
    try {
      return await tauri.fs.readTextFile(filePath);
    } catch {
      // repli ci-dessous.
    }
  }

  // 2) Projet persistant (dossier de données utilisateur).
  if (isInvokeAvailable()) {
    try {
      const stored = await invokeCommand<string | null>("read_project_data");
      if (stored) {
        return stored;
      }
    } catch {
      // repli ci-dessous.
    }
  }

  // 3) Repli navigateur.
  try {
    return window.localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
};
