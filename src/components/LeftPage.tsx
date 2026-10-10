import React from "react";
import { Settings, Info, ListTree, Share2, SpellCheck, LogOut } from "lucide-react";
import type { ActiveMenu } from "../types";

interface LeftPageProps {
  /** Onglet actuellement sélectionné. */
  activeMenu: ActiveMenu;
  /** Sélectionne un onglet. */
  setActiveMenu: (menu: ActiveMenu) => void;
  /** Ferme proprement l'application (cinématique 3D puis sortie native). */
  onExitApp: () => void;
}

interface TabItem {
  id: ActiveMenu;
  label: string;
  icon: React.ComponentType<{ size?: number; className?: string; strokeWidth?: number }>;
  /** Couleur thématique de l'onglet. */
  color: string;
}

// Marque-pages / onglets extérieurs (débordent à gauche du livre).
// Codes couleurs thématiques rétro : cuivre, bleu atelier, vert-de-gris, violet.
const tabs: TabItem[] = [
  { id: "reglages", label: "Réglages", icon: Settings, color: "bg-copper text-amber-50" },
  { id: "infos", label: "Informations", icon: Info, color: "bg-atelier text-sky-50" },
  { id: "organisation", label: "Organisation", icon: ListTree, color: "bg-verdigris text-emerald-50" },
  { id: "correcteur", label: "Correcteur", icon: SpellCheck, color: "bg-gold text-amber-950" },
  { id: "export", label: "Export", icon: Share2, color: "bg-retro-violet text-violet-50" },
];

export const LeftPage: React.FC<LeftPageProps> = ({
  activeMenu,
  setActiveMenu,
  onExitApp,
}) => {
  return (
    <div className="relative flex h-full flex-col items-center justify-center text-center">
      {/* Marque-pages empilés verticalement, en débordement sur la gauche */}
      <nav className="absolute top-16 -left-36 flex flex-col gap-1">
        {tabs.map((tab) => {
          const Icon = tab.icon;
          const active = activeMenu === tab.id;
          return (
            <button
              key={tab.id}
              type="button"
              onClick={() => setActiveMenu(tab.id)}
              aria-current={active ? "page" : undefined}
              className={`flex w-36 items-center gap-3 rounded-l-md px-4 py-3 text-left text-sm font-medium shadow-md transition-all duration-150 ${tab.color} ${
                active
                  ? "translate-x-1 shadow-lg ring-1 ring-white/30"
                  : "opacity-80 hover:translate-x-1 hover:opacity-100"
              }`}
            >
              <Icon size={18} strokeWidth={1.75} />
              <span className="tracking-wide">{tab.label}</span>
            </button>
          );
        })}

        {/* Action permanente dédiée : sortie de l'atelier depuis tout écran. */}
        <div className="mt-4 border-t border-stone-400/20 pt-2">
          <button
            type="button"
            onClick={onExitApp}
            title="Fermer Danoe Studio"
            className="flex w-36 items-center gap-3 rounded-l-md border-l-2 border-amber-600/60 bg-[#1c222e] px-4 py-3 text-left text-sm font-medium text-stone-300 shadow-md transition-all duration-150 hover:translate-x-1 hover:bg-[#252d3d] hover:shadow-lg"
          >
            <LogOut size={16} strokeWidth={1.75} />
            <span className="tracking-wide">Quitter l&rsquo;atelier</span>
          </button>
        </div>
      </nav>

      {/* Page de garde : identité visuelle */}
      <div className="mb-8 flex h-24 w-24 items-center justify-center rounded-sm bg-gold shadow-[inset_0_4px_10px_rgba(0,0,0,0.35),inset_0_-2px_6px_rgba(255,255,255,0.15)]">
        <span className="font-serif text-5xl font-bold leading-none text-parchment drop-shadow-[0_1px_1px_rgba(0,0,0,0.4)]">
          D
        </span>
      </div>

      <h1 className="font-serif text-4xl uppercase tracking-[0.4em] text-stone-800">
        Ano&euml; Studio
      </h1>

      <div className="mt-6 flex w-72 items-center gap-4">
        <span className="h-px flex-1 bg-stone-400/60" />
        <span className="whitespace-nowrap font-serif text-sm italic text-stone-500">
          Machine &agrave; romans
        </span>
        <span className="h-px flex-1 bg-stone-400/60" />
      </div>
    </div>
  );
};
