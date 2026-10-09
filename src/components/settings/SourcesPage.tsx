import React from "react";
import { ChevronLeft, FolderOpen } from "lucide-react";
import type { ImageColorMode, SourcesConfig } from "../../types";
import { pickDirectoryWithFiles, type FsDirectoryHandle } from "../../utils/directory";
import { PillButton } from "./controls";

interface SourcesPageProps {
  config: SourcesConfig;
  onColorModeChange: (mode: ImageColorMode) => void;
  onDirectoryChange: (
    path: string,
    files: string[],
    handle: FsDirectoryHandle | null,
  ) => void;
  onBack: () => void;
}

interface ColorOption {
  id: ImageColorMode;
  label: string;
}

const COLOR_OPTIONS: ColorOption[] = [
  { id: "color", label: "Conserver les couleurs" },
  { id: "grayscale", label: "Convertir en Noir & Blanc" },
];

/**
 * Page « Mes sources » — dossier unique (workspace) et traitement des images.
 *
 * Centralise toutes les ressources brutes du roman : un seul dossier contient
 * l'intégralité des fichiers du projet ; le moteur de tri les répartit par
 * extension (chapitres / images).
 */
export const SourcesPage: React.FC<SourcesPageProps> = ({
  config,
  onColorModeChange,
  onDirectoryChange,
  onBack,
}) => {
  const handleBrowse = async () => {
    const { path, files, handle } = await pickDirectoryWithFiles();
    if (path) {
      onDirectoryChange(path, files, handle);
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
        Mes sources
      </h2>

      {/* Section 1 — Emplacement du dossier unique (workspace) */}
      <h3 className="mb-2 font-mono text-[10px] uppercase tracking-widest text-stone-500">
        Emplacement du dossier unique (Workspace)
      </h3>
      {/* Bouton + chemin sur une même ligne : bouton à gauche, champ étiré à droite */}
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

        {/* Chemin sélectionné (chasse fixe) — occupe l'espace restant */}
        <div
          title={config.directory ?? undefined}
          className="flex-1 rounded-sm border border-stone-400/40 bg-stone-500/10 px-3 py-2"
        >
          {config.directory ? (
            <span className="block truncate font-mono text-xs text-stone-700">
              {config.directory}
            </span>
          ) : (
            <span className="block truncate font-mono text-xs italic text-stone-400">
              Aucun dossier sélectionné
            </span>
          )}
        </div>
      </div>

      {/* Note d'atelier — tris automatiques par extension */}
      <p className="mt-2 font-mono text-[10px] italic tracking-wide text-[#8a5a2b]">
        Chapitres : .txt, .md, .docx · Images : .png, .jpg, .jpeg, .webp, .tiff
      </p>

      {/* Section 2 — Traitement des images (export PDF) */}
      <h3 className="mt-6 mb-1.5 font-mono text-[10px] uppercase tracking-widest text-stone-500">
        Traitement des images (export PDF)
      </h3>
      <div className="mb-2 flex flex-wrap gap-2">
        {COLOR_OPTIONS.map((option) => (
          <PillButton
            key={option.id}
            active={config.colorMode === option.id}
            onClick={() => onColorModeChange(option.id)}
            title={
              option.id === "grayscale"
                ? "Option recommandée — impression KDP noir & blanc moins coûteuse"
                : undefined
            }
          >
            {option.label}
          </PillButton>
        ))}
      </div>
    </div>
  );
};
