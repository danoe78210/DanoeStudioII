import React, { useEffect } from "react";
import { AlertTriangle, CheckCircle2, Info } from "lucide-react";

export type ToastKind = "success" | "error" | "info";

export interface ToastMessage {
  kind: ToastKind;
  message: string;
}

interface ToastProps {
  toast: ToastMessage;
  onClose: () => void;
}

/** Notification discrète (coin inférieur droit), auto-fermante. */
export const Toast: React.FC<ToastProps> = ({ toast, onClose }) => {
  // Les notifications d'erreur **ne se ferment jamais automatiquement** : l'échec
  // d'un export doit rester visible tant que l'utilisateur ne l'a pas acquitté
  // (succès / info : auto-fermeture après ~6 s).
  useEffect(() => {
    if (toast.kind === "error") {
      return;
    }
    const timer = window.setTimeout(onClose, 6000);
    return () => window.clearTimeout(timer);
  }, [toast, onClose]);

  const Icon =
    toast.kind === "success"
      ? CheckCircle2
      : toast.kind === "error"
        ? AlertTriangle
        : Info;
  const tone =
    toast.kind === "success"
      ? "text-emerald-600"
      : toast.kind === "error"
        ? "text-red-600"
        : "text-stone-500";
  const container =
    toast.kind === "error"
      ? "border-red-500/70 bg-red-50 shadow-[0_10px_30px_rgba(153,27,27,0.35)]"
      : "border-stone-400/40 bg-parchment shadow-[0_10px_30px_rgba(0,0,0,0.35)]";
  const closeLabel = toast.kind === "error" ? "J'ai compris" : "Fermer";

  return (
    <div
      role={toast.kind === "error" ? "alert" : "status"}
      className={`fixed bottom-6 right-6 z-50 flex max-w-sm items-start gap-3 rounded-sm border px-4 py-3 ${container}`}
    >
      <Icon size={18} className={`mt-0.5 shrink-0 ${tone}`} />
      <p className="flex-1 font-serif text-sm text-stone-700">{toast.message}</p>
      <button
        type="button"
        onClick={onClose}
        className="shrink-0 font-mono text-[10px] uppercase tracking-widest text-stone-400 transition hover:text-copper"
      >
        {closeLabel}
      </button>
    </div>
  );
};