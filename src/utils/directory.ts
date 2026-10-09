import { invokeCommand, isInvokeAvailable } from "./tauri";

interface TauriDialog {
  open: (options: {
    directory?: boolean;
    multiple?: boolean;
  }) => Promise<string | string[] | null>;
}

interface TauriFs {
  readDir: (path: string) => Promise<Array<{ name: string }>>;
  copyFile?: (source: string, destination: string) => Promise<void>;
  exists?: (path: string) => Promise<boolean>;
}

interface TauriGlobal {
  dialog?: TauriDialog;
  fs?: TauriFs;
}

interface WindowWithTauri {
  __TAURI__?: TauriGlobal;
}

/** Poignée de fichier (File System Access API). */
interface FsWritable {
  write: (data: Blob) => Promise<void>;
  close: () => Promise<void>;
}

interface FsFileHandle {
  createWritable: () => Promise<FsWritable>;
}

interface FsDirEntry {
  kind: "file" | "directory";
  name: string;
}

/** Poignée de dossier (File System Access API — lecture + écriture). */
export interface FsDirectoryHandle {
  name: string;
  values: () => AsyncIterableIterator<FsDirEntry>;
  getFileHandle: (
    name: string,
    options?: { create?: boolean },
  ) => Promise<FsFileHandle>;
}

/** Indique si l'API de dialogue Tauri est disponible. */
export const isTauriAvailable = (): boolean => {
  if (typeof window === "undefined") {
    return false;
  }
  return Boolean((window as unknown as WindowWithTauri).__TAURI__?.dialog?.open);
};

/**
 * Ouvre le sélecteur de dossier **natif** de Tauri et renvoie le **chemin absolu**.
 * - Tente d'abord l'API globale `__TAURI__.dialog.open` (Tauri v1 / anciens builds).
 * - Sinon (Tauri v2 : les API de plugins ne sont **pas** injectées sur
 *   `window.__TAURI__` par `withGlobalTauri`), appelle la commande Rust
 *   `pick_directory` (plugin `tauri-plugin-dialog`).
 * Renvoie `null` hors Tauri ou si l'utilisateur annule.
 */
const pickNativeDirectory = async (): Promise<string | null> => {
  if (typeof window === "undefined") {
    return null;
  }

  const tauri = (window as unknown as WindowWithTauri).__TAURI__;
  if (tauri?.dialog?.open) {
    try {
      const result = await tauri.dialog.open({ directory: true, multiple: false });
      return Array.isArray(result) ? result[0] ?? null : result ?? null;
    } catch {
      // Échec de l'API globale → on retente via la commande Rust ci-dessous.
    }
  }

  if (isInvokeAvailable()) {
    try {
      return (await invokeCommand<string | null>("pick_directory")) ?? null;
    } catch {
      return null;
    }
  }

  return null;
};

/**
 * Ouvre l'explorateur de fichiers natif pour choisir un dossier.
 * - Application empaquetée (Tauri) : sélecteur natif `dialog.open({ directory: true })`
 *   (via l'API globale v1, sinon la commande Rust `pick_directory` en v2).
 * - Sinon (navigateur) : repli sur le sélecteur de dossier `webkitdirectory`.
 * Retourne le chemin (ou le nom du dossier) choisi, ou `null` si annulé.
 */
export const pickDirectory = async (): Promise<string | null> => {
  if (typeof window === "undefined") {
    return null;
  }

  // Application empaquetée : sélecteur natif (jamais de repli webkitdirectory ici,
  // qui donnerait un simple nom de dossier au lieu du chemin absolu).
  if (isTauriAvailable() || isInvokeAvailable()) {
    return pickNativeDirectory();
  }

  return new Promise<string | null>((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.setAttribute("webkitdirectory", "");
    input.setAttribute("directory", "");
    input.style.display = "none";
    input.addEventListener("change", () => {
      const files = input.files;
      if (files && files.length > 0) {
        const first = files[0] as File & { webkitRelativePath?: string };
        const relative = first.webkitRelativePath ?? first.name;
        resolve(relative.split("/")[0]);
      } else {
        resolve(null);
      }
      input.remove();
    });
    document.body.appendChild(input);
    input.click();
  });
};

/**
 * Liste les fichiers d'un dossier.
 * - Tauri v1 / anciens builds : API globale `__TAURI__.fs.readDir`.
 * - **Tauri v2** : le plugin `fs` n'est **pas** exposé sur `window.__TAURI__`
 *   (`withGlobalTauri` n'injecte que l'API cœur) → appel de la commande Rust
 *   `list_directory_files` (voir `src-tauri/src/lib.rs`).
 * - Sinon (navigateur) : retourne un tableau vide (l'énumération est alors
 *   fournie par le sélecteur de dossier lui-même).
 */
export const listDirectoryFiles = async (
  path: string | null,
): Promise<string[]> => {
  if (typeof window === "undefined" || !path) {
    console.log("[directory] listDirectoryFiles ignoré (chemin vide) :", path);
    return [];
  }

  // 1) API globale `fs.readDir` (Tauri v1 / anciens builds).
  const tauri = (window as unknown as WindowWithTauri).__TAURI__;
  if (tauri?.fs?.readDir) {
    try {
      const entries = await tauri.fs.readDir(path);
      const names = entries.map((entry) => entry.name);
      console.log("[directory] fs.readDir →", { path, names });
      return names;
    } catch (error) {
      console.warn("[directory] fs.readDir a échoué :", { path, error });
    }
  }

  // 2) Commande Rust `list_directory_files` (Tauri v2).
  if (isInvokeAvailable()) {
    try {
      const names = await invokeCommand<string[]>("list_directory_files", {
        path,
      });
      console.log("[directory] list_directory_files →", { path, names });
      return names;
    } catch (error) {
      console.warn("[directory] list_directory_files a échoué :", {
        path,
        error,
      });
    }
  }

  console.log(
    "[directory] aucune API d'énumération disponible (navigateur) → [] :",
    path,
  );
  return [];
};

/**
 * Indique si une API d'énumération de dossier est disponible : `fs.readDir`
 * (Tauri v1) **ou** les commandes Tauri (`list_directory_files`, Tauri v2).
 */
export const isFsAvailable = (): boolean => {
  if (typeof window === "undefined") {
    return false;
  }
  return (
    Boolean((window as unknown as WindowWithTauri).__TAURI__?.fs?.readDir) ||
    isInvokeAvailable()
  );
};

/** Résultat d'une sélection de dossier (chemin + fichiers détectés). */
export interface DirectoryPick {
  /** Chemin (Tauri) ou nom du dossier (navigateur), ou `null` si annulé. */
  path: string | null;
  /** Noms des fichiers détectés directement dans le dossier. */
  files: string[];
  /** Poignée de dossier réutilisable (File System Access API), sinon `null`. */
  handle: FsDirectoryHandle | null;
}

/** Liste les fichiers d'un dossier à partir d'une poignée (FSA). */
const listHandleFiles = async (
  handle: FsDirectoryHandle,
): Promise<string[]> => {
  const names: string[] = [];
  try {
    for await (const entry of handle.values()) {
      if (entry.kind === "file") {
        names.push(entry.name);
      }
    }
  } catch {
    // L'énumération peut échouer (permissions, poignée indisponible) : on ne
    // doit jamais perdre le dossier sélectionné pour autant.
  }
  names.sort((a, b) => a.localeCompare(b));
  return names;
};

/**
 * Sélectionne un dossier **et** renvoie les fichiers qu'il contient.
 * - Tauri : sélecteur natif (`dialog.open` v1, sinon commande Rust
 *   `pick_directory`) puis énumération (`fs.readDir` v1, sinon commande Rust
 *   `list_directory_files`).
 * - Navigateur : File System Access API, puis repli `webkitdirectory` (la liste
 *   des fichiers est disponible au moment de la sélection ; seuls les enfants
 *   directs sont conservés, les chemins relatifs comportant exactement 2 segments).
 */
export const pickDirectoryWithFiles = async (): Promise<DirectoryPick> => {
  if (typeof window === "undefined") {
    return { path: null, files: [], handle: null };
  }

  // Application empaquetée (Tauri) : sélecteur natif → chemin **absolu**.
  // `pickNativeDirectory` tente l'API globale `dialog.open` (v1) puis la
  // commande Rust `pick_directory` (v2 : les plugins ne sont **pas** exposés
  // sur `window.__TAURI__`). Les fichiers sont ensuite énumérés via
  // `listDirectoryFiles` (`fs.readDir` v1 ou commande Rust `list_directory_files`).
  if (isTauriAvailable() || isInvokeAvailable()) {
    const path = await pickNativeDirectory();
    console.log("[directory] dossier sélectionné (natif Tauri) :", path);
    if (!path) {
      return { path: null, files: [], handle: null };
    }
    const files = await listDirectoryFiles(path);
    return { path, files, handle: null };
  }

  // File System Access API (Chrome/Edge) : poignée réutilisable (lecture + écriture).
  const picker = (
    window as unknown as { showDirectoryPicker?: () => Promise<FsDirectoryHandle> }
  ).showDirectoryPicker;
  if (typeof picker === "function") {
    try {
      // Appel lié à `window` (une API détachée lèverait « Illegal invocation »).
      const handle = await picker.call(window);
      if (!handle) {
        return { path: null, files: [], handle: null };
      }
      // Énumération ISOLÉE : un échec de listage ne doit pas annuler la sélection.
      const files = await listHandleFiles(handle);
      console.log("[directory] dossier sélectionné (File System Access) :", {
        path: handle.name || null,
        files,
      });
      return { path: handle.name || null, files, handle };
    } catch (error) {
      // Annulation explicite de l'utilisateur → on s'arrête.
      if ((error as { name?: string } | null)?.name === "AbortError") {
        return { path: null, files: [], handle: null };
      }
      // Autre erreur (SecurityError en iframe/aperçu webview, API indisponible…) :
      // on NE renvoie PAS null — on retombe sur le sélecteur `<input>` ci-dessous,
      // sinon la sélection échouerait silencieusement.
    }
  }

  return new Promise<DirectoryPick>((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.setAttribute("webkitdirectory", "");
    input.setAttribute("directory", "");
    input.style.display = "none";
    input.addEventListener("change", () => {
      const list = input.files;
      if (!list || list.length === 0) {
        input.remove();
        resolve({ path: null, files: [], handle: null });
        return;
      }
      const names: string[] = [];
      let root = "";
      for (const file of Array.from(list)) {
        const relative =
          (file as File & { webkitRelativePath?: string }).webkitRelativePath ??
          file.name;
        const parts = relative.split("/");
        if (!root && parts.length > 1) {
          root = parts[0];
        }
        if (parts.length === 2) {
          names.push(parts[1]);
        }
      }
      names.sort((a, b) => a.localeCompare(b));
      input.remove();
      const pickedPath = root || (list[0]?.name ?? null);
      console.log("[directory] dossier sélectionné (<input> navigateur) :", {
        path: pickedPath,
        files: names,
      });
      resolve({
        path: pickedPath,
        files: names,
        handle: null,
      });
    });
    document.body.appendChild(input);
    input.click();
  });
};

/** Résultat d'une importation de fichier dans un dossier officiel. */
export interface ImportResult {
  /** Nom du fichier dans le dossier de destination (donc chemin relatif). */
  fileName: string;
  /** `true` si le fichier a été physiquement copié. */
  copied: boolean;
}

/** Sélectionne un fichier externe via `<input type="file">`. */
const pickExternalFile = (): Promise<File | null> =>
  new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.style.display = "none";
    input.addEventListener("change", () => {
      const file = input.files && input.files[0] ? input.files[0] : null;
      input.remove();
      resolve(file);
    });
    document.body.appendChild(input);
    input.click();
  });

/** Indique si une poignée de dossier contient déjà un fichier de ce nom. */
const handleHasFile = async (
  handle: FsDirectoryHandle,
  name: string,
): Promise<boolean> => {
  for await (const entry of handle.values()) {
    if (entry.kind === "file" && entry.name === name) {
      return true;
    }
  }
  return false;
};

/** Construit un nom de fichier libre (ajoute `_copie`, `_copie2`, …). */
const freeName = async (
  exists: (name: string) => Promise<boolean>,
  name: string,
): Promise<string> => {
  if (!(await exists(name))) {
    return name;
  }
  const dot = name.lastIndexOf(".");
  const stem = dot > 0 ? name.slice(0, dot) : name;
  const ext = dot > 0 ? name.slice(dot) : "";
  let index = 1;
  let candidate = `${stem}_copie${ext}`;
  while (await exists(candidate)) {
    index += 1;
    candidate = `${stem}_copie${index}${ext}`;
  }
  return candidate;
};

/**
 * Importe un fichier externe dans le dossier officiel (« bac à sable ») :
 * le fichier est **copié physiquement** dans le dossier configuré, et seul son
 * **nom** (chemin relatif) est conservé — garantissant la portabilité.
 *
 * - **Tauri v2** : commande Rust `import_file_into_folder` (sélecteur natif +
 *   copie + renommage en `_copie`), car les plugins `dialog`/`fs` ne sont pas
 *   exposés sur `window.__TAURI__`.
 * - **Tauri v1** : `dialog.open` (fichier) puis `fs.copyFile` ; conflit renommé.
 * - **Navigateur (File System Access)** : écriture via la poignée de dossier.
 * - **Sinon** : renvoie le nom avec `copied: false` (copie impossible).
 */
export const importFileIntoFolder = async (
  handle: FsDirectoryHandle | null,
  directoryPath: string | null,
): Promise<ImportResult | null> => {
  if (typeof window === "undefined") {
    return null;
  }

  const tauri = (window as unknown as WindowWithTauri).__TAURI__;

  // --- Tauri v2 : commande Rust dédiée (sélecteur + copie + renommage) ---
  if (directoryPath && isInvokeAvailable()) {
    try {
      const fileName = await invokeCommand<string | null>(
        "import_file_into_folder",
        { directory: directoryPath },
      );
      console.log("[directory] import_file_into_folder →", fileName);
      // `null` = annulation par l'utilisateur : on s'arrête ici.
      return fileName ? { fileName, copied: true } : null;
    } catch (error) {
      // Commande absente (backend Tauri v1) ou erreur : on retente les replis.
      console.warn(
        "[directory] import_file_into_folder indisponible, repli…",
        error,
      );
    }
  }

  // --- Tauri v1 : boîte de dialogue fichier + copie native ---
  if (tauri?.dialog?.open && tauri?.fs?.copyFile && directoryPath) {
    try {
      const picked = await tauri.dialog.open({
        directory: false,
        multiple: false,
      });
      const source = Array.isArray(picked) ? picked[0] ?? null : picked ?? null;
      if (!source) {
        return null;
      }
      const fsApi = tauri.fs;
      const copyFn = fsApi?.copyFile;
      if (!copyFn) {
        return null;
      }
      const existsFn = fsApi?.exists;
      const base = source.split(/[\\/]/).pop() ?? source;
      const fileName = existsFn
        ? await freeName((name) => existsFn(`${directoryPath}/${name}`), base)
        : base;
      await copyFn(source, `${directoryPath}/${fileName}`);
      return { fileName, copied: true };
    } catch {
      return null;
    }
  }

  // --- Navigateur : File System Access (écriture réelle dans le dossier) ---
  if (handle) {
    const file = await pickExternalFile();
    if (!file) {
      return null;
    }
    const fileName = await freeName(
      (name) => handleHasFile(handle, name),
      file.name,
    );
    const fileHandle = await handle.getFileHandle(fileName, { create: true });
    const writable = await fileHandle.createWritable();
    await writable.write(file);
    await writable.close();
    return { fileName, copied: true };
  }

  // --- Dernier repli : sélection sans copie possible ---
  const file = await pickExternalFile();
  if (!file) {
    return null;
  }
  return { fileName: file.name, copied: false };
};
