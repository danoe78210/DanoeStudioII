/**
 * Pont vers l'API **Tauri** (application de bureau).
 *
 * Détection sur `window.__TAURI__` (exposé grâce à `"withGlobalTauri": true`
 * dans `tauri.conf.json`) → **aucune dépendance npm imposée**, et repli
 * silencieux lorsque l'app tourne dans un simple navigateur (SPA Vite).
 */

type TauriInvoke = (
  command: string,
  args?: Record<string, unknown>,
) => Promise<unknown>;

/** Fonction de désabonnement renvoyée par `event.listen`. */
type TauriUnlisten = () => void;

/** Écouteur d'événement Tauri (`window.__TAURI__.event.listen`, API cœur). */
type TauriListen = <T>(
  event: string,
  handler: (event: { event: string; payload: T }) => void,
) => Promise<TauriUnlisten>;

interface TauriGlobal {
  /** API Tauri v2 (`window.__TAURI__.core.invoke`). */
  core?: { invoke?: TauriInvoke };
  /** API « plate » (v1 / variantes). */
  invoke?: TauriInvoke;
  /** API v1 (`window.__TAURI__.tauri.invoke`). */
  tauri?: { invoke?: TauriInvoke };
  /** Bus d'événements (API cœur, exposée avec `withGlobalTauri`). */
  event?: { listen?: TauriListen };
}

/** Résout la fonction `invoke` de Tauri (liée à son objet), ou `null`. */
const resolveInvoke = (): TauriInvoke | null => {
  if (typeof window === "undefined") {
    return null;
  }
  const tauri = (window as unknown as { __TAURI__?: TauriGlobal }).__TAURI__;
  if (!tauri) {
    return null;
  }
  if (tauri.core && typeof tauri.core.invoke === "function") {
    return tauri.core.invoke.bind(tauri.core);
  }
  if (typeof tauri.invoke === "function") {
    return tauri.invoke.bind(tauri);
  }
  if (tauri.tauri && typeof tauri.tauri.invoke === "function") {
    return tauri.tauri.invoke.bind(tauri.tauri);
  }
  return null;
};

/** Indique si les commandes Tauri sont disponibles (application empaquetée). */
export const isInvokeAvailable = (): boolean => resolveInvoke() !== null;

/**
 * Appelle une commande Tauri (`invoke`).
 * Lève une erreur si l'application n'est pas empaquetée — utiliser
 * `isInvokeAvailable()` en amont pour prévoir un repli.
 */
export const invokeCommand = async <T,>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> => {
  const invoke = resolveInvoke();
  if (!invoke) {
    throw new Error(
      "Commandes Tauri indisponibles (application non empaquetée).",
    );
  }
  return (await invoke(command, args)) as T;
};

/** Résout l'écouteur d'événements Tauri (`window.__TAURI__.event.listen`), ou `null`. */
const resolveListen = (): TauriListen | null => {
  if (typeof window === "undefined") {
    return null;
  }
  const tauri = (window as unknown as { __TAURI__?: TauriGlobal }).__TAURI__;
  const bus = tauri?.event;
  const listen = bus?.listen;
  return typeof listen === "function" ? listen.bind(bus) : null;
};

/** Indique si le bus d'événements Tauri est disponible (application empaquetée). */
export const isEventAvailable = (): boolean => resolveListen() !== null;

/**
 * Abonne un gestionnaire à un événement Tauri (`app.emit` côté Rust).
 * Renvoie la fonction de **désabonnement** (`unlisten`) à appeler au démontage
 * du composant ou à la résolution de l'opération, ou `null` hors Tauri
 * (SPA navigateur) — dans ce cas aucun abonnement n'est posé.
 */
export const listenEvent = async <T,>(
  event: string,
  handler: (payload: T) => void,
): Promise<TauriUnlisten | null> => {
  const listen = resolveListen();
  if (!listen) {
    return null;
  }
  try {
    return await listen<T>(event, (message) => handler(message.payload));
  } catch {
    return null;
  }
};