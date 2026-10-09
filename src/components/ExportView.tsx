import React, { useEffect, useState } from "react";
import { BookOpen, FileText, FileType, Loader2 } from "lucide-react";
import { listenEvent } from "../utils/tauri";

interface ExportViewProps {
  /** Déclenche l'export du format demandé (moteur Rust via Tauri, ou simulé en navigateur). */
  onExport: (format: string) => void;
  /** Ouvre l'aperçu interactif (flipbook) du manuscrit rendu par Typst. */
  onPreview?: () => void;
  /** Un export est en cours : désactive les boutons et affiche l'indicateur de chargement. */
  isExporting?: boolean;
}

interface ExportFormat {
  id: string;
  label: string;
  description: string;
  icon: React.ComponentType<{ size?: number; className?: string; strokeWidth?: number }>;
}

/** Charge utile de l'événement `export-progress` émis par le moteur Rust. */
interface ExportProgress {
  /** Libellé de l'étape en cours (ex. « Lecture des sources… »). */
  step: string;
  /** Avancement global de 0 à 100. */
  progress: number;
}

const formats: ExportFormat[] = [
  {
    id: "Word (.docx)",
    label: "Word (.docx)",
    description: "Document structuré avec styles natifs et sauts de section.",
    icon: FileText,
  },
  {
    id: "PDF prêt-à-imprimer",
    label: "PDF prêt-à-imprimer",
    description: "Verrouillé KDP Broché (Gutter & Bleed calculés).",
    icon: FileType,
  },
  {
    id: "Ebook (.epub)",
    label: "Ebook (.epub)",
    description: "EPUB 3 fluide optimisé pour les liseuses Kindle.",
    icon: BookOpen,
  },
];

export const ExportView: React.FC<ExportViewProps> = ({
  onExport,
  onPreview,
  isExporting = false,
}) => {
  // Progression réelle (0–100) et étape en cours, alimentées par `export-progress`.
  const [progress, setProgress] = useState(0);
  const [step, setStep] = useState("");

  // Abonnement au bus d'événements Tauri : posé au montage (avant tout clic),
  // désabonné au démontage — aucun événement n'est manqué.
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let disposed = false;
    void listenEvent<ExportProgress>("export-progress", (payload) => {
      setProgress(Math.min(100, Math.max(0, Math.round(payload.progress))));
      setStep(payload.step);
    }).then((off) => {
      if (disposed) {
        off?.();
      } else {
        unlisten = off;
      }
    });
    return () => {
      disposed = true;
      unlisten?.();
      unlisten = null;
    };
  }, []);

  // Réinitialise la jauge au démarrage d'un nouvel export.
  useEffect(() => {
    if (isExporting) {
      setProgress(0);
      setStep("Génération du manuscrit en cours…");
    }
  }, [isExporting]);

  const clamped = Math.min(100, Math.max(0, progress));

  return (
    <div className="h-full">
      <h2 className="mb-8 border-b border-stone-400/40 pb-4 text-center font-serif text-3xl uppercase tracking-[0.3em] text-stone-700">
        Export
      </h2>

      {/* Jauge de progression — l'export est une tâche lourde. */}
      {isExporting && (
        <div className="mx-auto mb-5 max-w-xl">
          <div className="mb-2 flex items-center justify-between gap-3">
            <span className="flex items-center gap-2 font-mono text-[11px] uppercase tracking-widest text-copper">
              <Loader2 size={15} className="animate-spin" />
              {step || "Génération du manuscrit en cours…"}
            </span>
            <span className="font-mono text-xs tabular-nums text-stone-600">
              {clamped}&#8202;%
            </span>
          </div>
          <div className="h-2.5 w-full overflow-hidden rounded-sm border border-stone-400/40 bg-stone-500/10 shadow-[inset_0_2px_4px_rgba(0,0,0,0.15)]">
            <div
              className="h-full bg-linear-to-r from-copper to-brass transition-all duration-300 ease-out"
              style={{ width: `${clamped}%` }}
            />
          </div>
        </div>
      )}

      <div className="mx-auto max-w-xl space-y-3">
        {formats.map((format) => {
          const Icon = format.icon;
          return (
            <button
              key={format.id}
              type="button"
              disabled={isExporting}
              onClick={() => onExport(format.id)}
              className="flex w-full items-center gap-4 rounded-sm border border-stone-400/30 bg-stone-500/5 px-4 py-4 text-left transition-colors hover:bg-stone-500/15 disabled:cursor-not-allowed disabled:opacity-50"
            >
              <Icon size={22} strokeWidth={1.5} className="text-stone-600" />
              <span className="flex-1">
                <span className="block font-medium text-stone-800">{format.label}</span>
                <span className="block text-xs text-stone-500">{format.description}</span>
              </span>
            </button>
          );
        })}

        {/* Aperçu interactif — rendu fidèle du PDF Typst (feuilletage du livre). */}
        {onPreview && (
          <button
            type="button"
            disabled={isExporting}
            onClick={onPreview}
            className="flex w-full items-center gap-4 rounded-sm border border-copper/40 bg-copper/10 px-4 py-4 text-left transition-colors hover:bg-copper/20 disabled:cursor-not-allowed disabled:opacity-50"
          >
            <BookOpen size={22} strokeWidth={1.5} className="text-copper" />
            <span className="flex-1">
              <span className="block font-medium text-stone-800">Aperçu interactif</span>
              <span className="block text-xs text-stone-500">
                Feuilletez le livre rendu par le moteur Typst (KDP).
              </span>
            </span>
          </button>
        )}
      </div>
    </div>
  );
};