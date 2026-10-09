import { invokeCommand, isInvokeAvailable } from "./tauri";

/**
 * Rend les octets du **PDF prêt-à-imprimer** du projet **en mémoire** via la
 * commande Tauri `render_pdf` — aucun dialogue, aucune écriture sur le disque.
 *
 * Le backend renvoie un `ArrayBuffer` brut (`tauri::ipc::Response`) ; on tolère
 * aussi un tableau d'octets JSON pour rester robuste selon la version de Tauri.
 */
export const renderPdfBytes = async (payload: unknown): Promise<Uint8Array> => {
  if (!isInvokeAvailable()) {
    throw new Error(
      "Aperçu indisponible : les commandes Tauri nécessitent l'application empaquetée.",
    );
  }
  const raw = await invokeCommand<unknown>("render_pdf", { payload });
  if (raw instanceof Uint8Array) {
    return raw;
  }
  if (raw instanceof ArrayBuffer) {
    return new Uint8Array(raw);
  }
  if (Array.isArray(raw)) {
    return Uint8Array.from(raw as number[]);
  }
  throw new Error("Réponse PDF inattendue du moteur d'export.");
};
