import React, { type ReactNode } from "react";
import { LogOut } from "lucide-react";
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
  /** Ferme proprement l'application (vidage du cache + sortie native). */
  onQuit: () => void;
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
  onQuit,
  closing = false,
}) => {
  return (
    <div className="relative flex h-screen w-full flex-col items-center gap-6 overflow-hidden bg-desk p-6">
      {/* Fermeture propre : vide les caches locaux puis quitte l'application. */}
      <button
        type="button"
        onClick={onQuit}
        title="Vider le cache applicatif puis fermer Danoë Studio"
        className="absolute top-5 right-6 z-10 flex items-center gap-2 rounded-sm border border-stone-500/40 bg-desk-light/90 px-3 py-2 text-[11px] font-medium uppercase tracking-[0.2em] text-parchment/90 shadow-[0_6px_16px_rgba(0,0,0,0.45)] transition-colors hover:bg-copper hover:text-amber-50"
      >
        <LogOut size={15} strokeWidth={1.75} />
        Quitter &amp; vider le cache
      </button>

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

              {/* Face externe : couverture cuir/cuivre rétro, logo doré (visible au verso). */}
              <div
                aria-hidden
                className="absolute inset-0 flex items-center justify-center rounded-r-md border border-brass/40 bg-linear-to-br from-[#7a4a24] via-[#5c3418] to-[#3d2210] shadow-[inset_0_0_60px_rgba(0,0,0,0.5)]"
                style={{ transform: "rotateY(180deg)", backfaceVisibility: "hidden" }}
              >
                <span className="flex h-20 w-20 items-center justify-center rounded-full bg-gold/90 shadow-[inset_0_2px_6px_rgba(0,0,0,0.35)]">
                  <span className="font-serif text-4xl font-bold leading-none text-[#3d2210]">
                    D
                  </span>
                </span>
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

