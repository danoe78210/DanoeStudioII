/**
 * Fermeture « propre » de l'application : vidage du **cache volatile** puis
 * sortie native.
 *
 * Le nettoyage ne porte **jamais** sur les données utilisateur :
 * 1. **Web** — seuls `sessionStorage` et le Cache API (caches volatils) sont
 *    purgés. `localStorage` est **préservé** (il contient le projet en repli).
 * 2. **Tauri** — le cache volatil est purgé côté Rust (sous-dossier dédié), puis
 *    la fenêtre est fermée (`exit(0)`) via `clear_cache_and_exit`. Le dossier de
 *    données persistant (`app_data_dir`) n'est pas touché.
 *
 * Hors Tauri (SPA Vite dans un navigateur), on retombe sur `window.close()`.
 */

import { invokeCommand, isInvokeAvailable, listenEvent } from "./tauri";

/**
 * Vide **uniquement les caches volatils** (best-effort).
 *
 * ⚠️ `localStorage` n'est **pas** effacé : il contient les données utilisateur
 * (projet) en repli navigateur.
 */
export const clearWebCaches = async (): Promise<void> => {
  try {
    window.sessionStorage.clear();
  } catch {
    // Stockage indisponible (mode privé / quota) : ignoré.
  }

  if (typeof caches !== "undefined") {
    try {
      const keys = await caches.keys();
      await Promise.all(keys.map((key) => caches.delete(key)));
    } catch {
      // Échec de purge du Cache API : ignoré (non bloquant).
    }
  }
};

/**
 * Sauvegarde implicite (l'auto-save a déjà persisté l'état), vide les caches
 * locaux, puis invoque la fermeture native de la fenêtre Tauri.
 */
export const shutdownApp = async (): Promise<void> => {
  await clearWebCaches();

  if (isInvokeAvailable()) {
    try {
      await invokeCommand("clear_cache_and_exit");
      return;
    } catch {
      // Repli ci-dessous si la commande échoue.
    }
  }

  // Repli navigateur : ferme l'onglet/fenêtre si possible.
  window.close();
};

/**
 * **Finalise** la sortie, une fois l'animation 3D de fermeture du livre achevée :
 * purge des caches volatils puis sortie native (`finalize_exit`).
 */
export const finalizeExit = async (): Promise<void> => {
  await clearWebCaches();

  if (isInvokeAvailable()) {
    try {
      await invokeCommand("finalize_exit");
      return;
    } catch {
      // Repli ci-dessous.
    }
  }

  window.close();
};

/**
 * S'abonne à la **demande de fermeture** de la fenêtre (croix native) émise par
 * Tauri (`app-close-requested`) : l'UI joue alors la cinématique avant la sortie.
 */
export const onCloseRequested = (handler: () => void): void => {
  void listenEvent("app-close-requested", handler);
};
