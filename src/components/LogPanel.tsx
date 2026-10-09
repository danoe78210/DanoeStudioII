import React, { useEffect, useRef } from "react";
import { Feather, FolderOpen } from "lucide-react";
import type { LogEntry, LogLevel } from "../types";

interface LogPanelProps {
  logs: LogEntry[];
  /** État de sauvegarde automatique du projet. */
  saveState?: 'idle' | 'saving' | 'saved';
  /** Chemin du fichier de projet (null si non défini). */
  projectFilePath?: string | null;
  /** Choisit l'emplacement du fichier de projet. */
  onPickProjectFile?: () => void;
}

const levelStyles: Record<LogLevel, string> = {
  info: "text-amber-100/80",
  success: "text-emerald-300",
  warning: "text-amber-300",
  error: "text-red-400",
};

const levelPrefix: Record<LogLevel, string> = {
  info: "\u203A",
  success: "\u2713",
  warning: "!",
  error: "\u2717",
};

export const LogPanel: React.FC<LogPanelProps> = ({
  logs,
  saveState = 'idle',
  projectFilePath = null,
  onPickProjectFile,
}) => {
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const node = scrollRef.current;
    if (node) {
      node.scrollTop = node.scrollHeight;
    }
  }, [logs]);

  const fileLabel = projectFilePath
    ? projectFilePath.split(/[\\/]/).pop()
    : "Dossier du projet";

  return (
    <div className="w-full max-w-6xl">
      {/* Registre de bord — console d'atelier */}
      <div className="relative overflow-hidden rounded-md border border-brass/40 bg-linear-to-b from-[#10141c] to-[#0a0d13] shadow-[inset_0_2px_14px_rgba(0,0,0,0.9),0_6px_14px_rgba(0,0,0,0.5)]">
        {/* Vis de fixation aux quatre coins */}
        <span className="absolute left-2 top-2 h-1.5 w-1.5 rounded-full bg-brass/60 shadow-[inset_0_1px_1px_rgba(0,0,0,0.6)]" />
        <span className="absolute right-2 top-2 h-1.5 w-1.5 rounded-full bg-brass/60 shadow-[inset_0_1px_1px_rgba(0,0,0,0.6)]" />
        <span className="absolute bottom-2 left-2 h-1.5 w-1.5 rounded-full bg-brass/60 shadow-[inset_0_1px_1px_rgba(0,0,0,0.6)]" />
        <span className="absolute bottom-2 right-2 h-1.5 w-1.5 rounded-full bg-brass/60 shadow-[inset_0_1px_1px_rgba(0,0,0,0.6)]" />

        <div className="flex items-center justify-between gap-3 border-b border-brass/30 px-5 py-2">
          <span className="font-mono text-[11px] uppercase tracking-[0.3em] text-amber-100/60">
            Journal des op&eacute;rations
          </span>
          <span className="flex items-center gap-4">
            {/* Témoin discret de sauvegarde automatique */}
            {saveState === "saved" && (
              <span className="flex items-center gap-1 font-mono text-[10px] uppercase tracking-widest text-emerald-300">
                <Feather size={12} />
                Consign&eacute;
              </span>
            )}
            {/* Emplacement du fichier de projet */}
            <button
              type="button"
              onClick={onPickProjectFile}
              title={
                projectFilePath ?? "Choisir l'emplacement du fichier de projet"
              }
              className="flex items-center gap-1 font-mono text-[10px] uppercase tracking-widest text-amber-100/50 transition-colors hover:text-copper"
            >
              <FolderOpen size={12} />
              {fileLabel}
            </button>
            <span className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-amber-100/35">
              <span className="inline-block h-1.5 w-1.5 animate-pulse rounded-full bg-emerald-400" />
              Enregistrement actif
            </span>
          </span>
        </div>

        <div
          ref={scrollRef}
          className="h-32 overflow-y-auto px-5 py-2 font-mono text-xs leading-relaxed"
        >
          {logs.length === 0 ? (
            <p className="text-amber-100/30">Aucune op&eacute;ration enregistr&eacute;e.</p>
          ) : (
            logs.map((log) => (
              <div key={log.id} className="flex gap-3">
                <span className="shrink-0 text-amber-100/30">[{log.time}]</span>
                <span className={levelStyles[log.level]}>
                  {levelPrefix[log.level]} {log.message}
                </span>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};
