interface TauriShell {
  open: (path: string) => Promise<void>;
}

interface TauriGlobal {
  shell?: TauriShell;
}

interface WindowWithTauri {
  __TAURI__?: TauriGlobal;
}

/** Indique si l'API shell Tauri est disponible. */
export const isShellAvailable = (): boolean => {
  if (typeof window === "undefined") {
    return false;
  }
  return Boolean((window as unknown as WindowWithTauri).__TAURI__?.shell?.open);
};

/**
 * Demande à l'OS d'ouvrir un fichier avec l'application par défaut
 * (via l'API Tauri `shell.open`).
 * Retourne `true` si l'ouverture a été déclenchée, `false` sinon (hors Tauri).
 */
export const openPathExternal = async (path: string): Promise<boolean> => {
  if (typeof window === "undefined") {
    return false;
  }
  const tauri = (window as unknown as WindowWithTauri).__TAURI__;
  if (tauri?.shell?.open) {
    try {
      await tauri.shell.open(path);
      return true;
    } catch {
      return false;
    }
  }
  return false;
};
