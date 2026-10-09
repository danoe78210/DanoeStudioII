import React from "react";
import { ChevronLeft, FileText, FolderOpen } from "lucide-react";
import type { ErrorLogConfig, LogDiagnosticLevel } from "../../types";
import { pickDirectory } from "../../utils/directory";
import { PillButton } from "./controls";

interface ErrorLogPageProps {
  config: ErrorLogConfig;
  onLevelChange: (level: LogDiagnosticLevel) => void;
  onDirectoryChange: (path: string) => void;
  /** Demande à l'OS d'ouvrir le fichier journal. */
  onOpenLog: () => void;
  onBack: () => void;
}

interface LevelOption {
  id: LogDiagnosticLevel;
  label: string;
    hint: string;
}

const LEVEL_OPTIONS: LevelOption[] = [
  {
    id: "standard",
    label: "Standard",
    hint: "N'enregistre que les alertes et les erreurs critiques",
  },
  {
    id: "diagnostic",
    label: "Mécanique / Diagnostic",
    hint: "Enregistre toutes les opérations de compilation et d'exportation",
  },
];

/** Page « Journal des erreurs » — diagnostic, emplacement, accès rapide. */
export const ErrorLogPage: React.FC<ErrorLogPageProps> = ({
  config,
  onLevelChange,
  onDirectoryChange,
  onOpenLog,
  onBack,
}) => {
  const handleBrowse = async () => {
    const path = await pickDirectory();
    if (path) {
      onDirectoryChange(path);
    }
  };

  return (
    <div className="h-full">
      <button
        type="button"
        onClick={onBack}
        className="mb-3 flex items-center gap-1 font-mono text-[11px] uppercase tracking-widest text-stone-500 transition hover:text-copper"
      >
        <ChevronLeft size={14} />
        Retour
      </button>

      <h2 className="mb-5 border-b border-stone-400/40 pb-3 text-center font-serif text-2xl uppercase tracking-[0.25em] text-stone-700">
        Journal des erreurs
      </h2>

      {/* Section 1 — Niveau de diagnostic */}
      <h3 className="mb-1.5 font-mono text-[10px] uppercase tracking-widest text-stone-500">
        Niveau de diagnostic
      </h3>
      <p className="mb-2 font-serif text-xs italic text-stone-500">
        Réglez la verbosité du journal&nbsp;: sobre au quotidien, exhaustif en cas
        de bogue.
      </p>
      <div className="mb-5 flex flex-wrap gap-2">
        {LEVEL_OPTIONS.map((option) => (
          <PillButton
            key={option.id}
            active={config.level === option.id}
            onClick={() => onLevelChange(option.id)}
            title={option.hint}
          >
            {option.label}
          </PillButton>
        ))}
      </div>

      {/* Section 2 — Emplacement du fichier journal */}
      <h3 className="mb-2 font-mono text-[10px] uppercase tracking-widest text-stone-500">
        Emplacement du fichier journal
      </h3>
      <div className="flex flex-row items-center gap-4">
        <button
          type="button"
          onClick={() => {
            void handleBrowse();
          }}
          className="flex shrink-0 items-center gap-2 rounded-sm border border-brass/50 bg-linear-to-b from-copper to-[#8a5426] px-4 py-2 font-mono text-[11px] uppercase tracking-widest text-amber-50 shadow-[inset_0_1px_0_rgba(255,255,255,0.3),0_2px_4px_rgba(0,0,0,0.5)] transition-all hover:brightness-110 active:translate-y-px active:shadow-[inset_0_2px_4px_rgba(0,0,0,0.6)]"
        >
          <FolderOpen size={14} strokeWidth={2} />
          Parcourir…
        </button>

        <div
          title={config.directory ?? undefined}
          className="flex-1 rounded-sm border border-stone-400/40 bg-stone-500/10 px-3 py-2 shadow-[inset_0_2px_4px_rgba(0,0,0,0.08)]"
        >
          {config.directory ? (
            <span className="block truncate font-mono text-xs text-stone-700">
              {config.directory}
            </span>
          ) : (
            <span className="block truncate font-mono text-xs italic text-stone-400/70">
              Dossier par défaut du système (AppData / .config)
            </span>
          )}
        </div>
      </div>

      {/* Section 3 — Accès rapide */}
      <h3 className="mb-2 mt-5 font-mono text-[10px] uppercase tracking-widest text-stone-500">
        Accès rapide
      </h3>
      <button
        type="button"
        onClick={() => {
          void onOpenLog();
        }}
        className="flex items-center gap-2 rounded-sm border border-stone-400/50 bg-stone-500/5 px-4 py-2 font-mono text-[11px] uppercase tracking-widest text-stone-600 transition-colors hover:bg-stone-500/10 hover:text-copper"
      >
        <FileText size={14} strokeWidth={2} />
        Ouvrir le fichier de bord
      </button>
    </div>
  );
};
