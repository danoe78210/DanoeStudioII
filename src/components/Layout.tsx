import React, { type ReactNode } from "react";
import { ProgressBar } from "./ProgressBar";
import { LogPanel } from "./LogPanel";
import { MenuFlipBook } from "./MenuFlipBook";
import type { LogEntry } from "../types";

interface LayoutProps {
  /** Contenu de la page de gauche (navigation & identité). */
  leftContent: ReactNode;
  /** Pages du livre virtuel de droite (une vue par menu). */
  bookPages: ReactNode[];
  /** Index de la page active (piloté par le ruban de navigation). */
  activeIndex: number;
  /** Avancement global de 0 à 100. */
  progress: number;
  /** Libellé de l'opération en cours. */
  progressLabel: string;
  /** Indique qu'un traitement lourd est actif. */
  running: boolean;
  /** Historique des opérations journalisées. */
  logs: LogEntry[];
  /** État de sauvegarde automatique du projet. */
  saveState: 'idle' | 'saving' | 'saved';
  /** Chemin du fichier de projet (null si non défini). */
  projectFilePath: string | null;
  /** Choisit l'emplacement du fichier de projet. */
  onPickProjectFile: () => void;
  /** Cinématique de fermeture du livre en cours (volet 3D + glow). */
  closing?: boolean;
}

export const Layout: React.FC<LayoutProps> = ({
  leftContent,
  bookPages,
  activeIndex,
  progress,
  progressLabel,
  running,
  logs,
  saveState,
  projectFilePath,
  onPickProjectFile,
  closing = false,
}) => {
  return (
    <div className="relative flex h-screen w-full flex-col items-center gap-6 overflow-hidden bg-desk p-6">
      {/* Colonne commune : la barre de progression, le livre et le journal
          partagent EXACTEMENT la même emprise horizontale (largeur + position).
          `px-[6.5rem]` réserve une gouttière symétrique pour les marque-pages
          (à gauche) : le ruban reste dans la fenêtre et l'ensemble reste centré.
          `flex-1 min-h-0` fait tenir le tout dans la hauteur de la fenêtre
          (aucun ascenseur global). */}
      <div className="flex w-full min-h-0 max-w-[85rem] flex-1 flex-col items-center gap-6 px-[6.5rem]">
        {/* Barre de progression — au-dessus du livre */}
        <ProgressBar progress={progress} label={progressLabel} running={running} />

        {/* Le Registre — livre ancien : navigation à gauche, pages tournantes à droite.
            Perspective 3D (1400 px) : le volet droit se rabat vers la gauche à la fermeture. */}
        <div className="relative flex w-full min-h-0 flex-1 [perspective:1400px]">
          {/* Halo lumineux d'ambiance (backdrop glow) — derrière le livre. */}
          <div
            aria-hidden
            className={`pointer-events-none absolute -inset-16 z-0 rounded-full blur-3xl transition-opacity duration-500 ${
              closing ? "opacity-100" : "opacity-0"
            }`}
            style={{
              background: "radial-gradient(circle, rgba(198, 134, 66, 0.4) 0%, transparent 70%)",
            }}
          />

          <div className="relative flex h-full w-full min-h-0 rounded-md shadow-[0_20px_50px_rgba(0,0,0,0.6)]">
            {/* Page de gauche : ombre interne simulant la courbure vers la reliure */}
            <section className="relative h-full w-1/2 rounded-l-md bg-parchment p-10 shadow-[inset_-25px_0_25px_-20px_rgba(0,0,0,0.15)]">
              {leftContent}
            </section>

            {/* Page de droite : volet qui pivote autour de la reliure (`left`). */}
            <section
              className="relative flex h-full w-1/2 flex-col overflow-hidden rounded-r-md bg-parchment shadow-[inset_25px_0_25px_-20px_rgba(0,0,0,0.15)]"
              style={{
                transformOrigin: "left center",
                transformStyle: "preserve-3d",
                transition: "transform 1000ms cubic-bezier(0.22, 1, 0.36, 1)",
                transform: closing ? "rotateY(-180deg)" : "rotateY(0deg)",
              }}
            >
              {/* Face interne : le livre virtuel (pages de menus). */}
              <div
                className="flex h-full min-h-0 flex-col p-6"
                style={{ backfaceVisibility: "hidden" }}
              >
                <MenuFlipBook pages={bookPages} activeIndex={activeIndex} />
              </div>

              {/* Face externe (Verso) : couverture cuir patinée, révélée pendant le rabat. */}
              <div
                aria-hidden
                className="absolute inset-0 flex items-center justify-center overflow-hidden rounded-r-md bg-[#1c140e] shadow-[inset_20px_0_30px_rgba(0,0,0,0.6)]"
                style={{ transform: "rotateY(180deg)", backfaceVisibility: "hidden" }}
              >
                {/* Cadre extérieur laiton. */}
                <div className="m-2 flex h-[calc(100%-1rem)] w-[calc(100%-1rem)] items-center justify-center rounded-r-md border-2 border-brass/50">
                  {/* Filet intérieur estampé à chaud + coins renforcés. */}
                  <div className="relative flex h-full w-full items-center justify-center rounded-sm border border-brass/25">
                    <span className="pointer-events-none absolute top-1 left-1 h-5 w-5 border-t-2 border-l-2 border-brass/40" />
                    <span className="pointer-events-none absolute top-1 right-1 h-5 w-5 border-t-2 border-r-2 border-brass/40" />
                    <span className="pointer-events-none absolute bottom-1 left-1 h-5 w-5 border-b-2 border-l-2 border-brass/40" />
                    <span className="pointer-events-none absolute right-1 bottom-1 h-5 w-5 border-r-2 border-b-2 border-brass/40" />

                    {/* Sceau central : monogramme doré + mention discrète. */}
                    <div className="flex flex-col items-center gap-3">
                      <span className="flex h-20 w-20 items-center justify-center rounded-full bg-gold/90 shadow-[inset_0_2px_6px_rgba(0,0,0,0.35),0_4px_12px_rgba(0,0,0,0.5)]">
                        <span className="font-serif text-4xl font-bold leading-none text-[#3d2210]">
                          D
                        </span>
                      </span>
                      <span className="font-serif text-xs uppercase tracking-[0.35em] text-brass/70">
                        Dano&euml; Studio
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            </section>
          </div>

          {/* Lueur de la reliure (seam glow) — s'intensifie au début de la rotation. */}
          <div
            aria-hidden
            className="pointer-events-none absolute inset-y-2 left-1/2 z-20 w-[3px] -translate-x-1/2 rounded-full transition-all duration-500"
            style={{
              boxShadow: closing ? "0 0 25px 6px rgba(217, 119, 6, 0.6)" : "none",
              opacity: closing ? 1 : 0,
            }}
          />
        </div>

        {/* Terminal — journal des opérations, sous le livre */}
        <LogPanel
          logs={logs}
          saveState={saveState}
          projectFilePath={projectFilePath}
          onPickProjectFile={onPickProjectFile}
        />
      </div>
    </div>
  );
};

