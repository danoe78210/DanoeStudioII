import React from "react";
import {
  Settings,
  FolderOpen,
  ShieldAlert,
  SlidersHorizontal,
  ChevronRight,
  SpellCheck,
} from "lucide-react";

interface SettingsItem {
  id: string;
  label: string;
  icon: React.ComponentType<{ size?: number; strokeWidth?: number; className?: string }>;
  /** Action de navigation — rend la ligne cliquable. */
  action?: () => void;
}

interface SettingsViewProps {
  /** Ouvre la page « Paramètres du livre ». */
  onOpenGeneralSettings: () => void;
  /** Ouvre la page « Mes sources » (dossier unique du projet). */
  onOpenSources: () => void;
  /** Ouvre la page « Journal des erreurs ». */
  onOpenErrorLog: () => void;
  /** Ouvre la page « Correcteur linguistique ». */
  onOpenCorrector: () => void;
  /** Libellé du format de coupe actuellement appliqué au projet. */
  trimLabel: string;
}

export const SettingsView: React.FC<SettingsViewProps> = ({
  onOpenGeneralSettings,
  onOpenSources,
  onOpenErrorLog,
  onOpenCorrector,
  trimLabel,
}) => {
  // Ordre du sommaire : livre, sources, erreurs, correcteur.
  const settingsItems: SettingsItem[] = [
    { id: "general", label: "Paramètres du livre", icon: Settings, action: onOpenGeneralSettings },
    { id: "sources", label: "Mes sources", icon: FolderOpen, action: onOpenSources },
    { id: "erreurs", label: "Journal des erreurs", icon: ShieldAlert, action: onOpenErrorLog },
    {
      id: "correcteur",
      label: "Correcteur linguistique",
      icon: SpellCheck,
      action: onOpenCorrector,
    },
  ];

  return (
    <div className="h-full">
      <h2 className="mb-8 border-b border-stone-400/40 pb-4 text-center font-serif text-3xl uppercase tracking-[0.3em] text-stone-700">
        Réglages
      </h2>

      {/* Lignes de registre épurées */}
      <div className="mx-auto max-w-xl">
        {settingsItems.map((item) => {
          const Icon = item.icon;
          const isGeneral = item.id === "general";
          const navigable = Boolean(item.action);
          return (
            <div
              key={item.id}
              onClick={item.action}
              className={`flex items-center gap-4 border-b border-stone-400/25 bg-stone-500/5 px-4 py-4 transition-colors hover:bg-stone-500/10 ${
                navigable ? "cursor-pointer" : ""
              }`}
            >
              <Icon size={20} strokeWidth={1.5} className="text-stone-600" />
              <span className="flex-1">
                <span className="block font-medium text-stone-700">{item.label}</span>
                {isGeneral && (
                  <span className="block font-mono text-[10px] uppercase tracking-widest text-stone-500">
                    Format de coupe : {trimLabel}
                  </span>
                )}
              </span>
              {isGeneral && (
                <button
                  type="button"
                  onClick={(event) => {
                    event.stopPropagation();
                    onOpenGeneralSettings();
                  }}
                  className="flex items-center gap-2 rounded-sm border border-brass/50 bg-linear-to-b from-copper to-[#8a5426] px-3 py-1.5 font-mono text-[11px] uppercase tracking-widest text-amber-50 shadow-[inset_0_1px_0_rgba(255,255,255,0.3),0_2px_4px_rgba(0,0,0,0.5)] transition-all hover:brightness-110 active:translate-y-px active:shadow-[inset_0_2px_4px_rgba(0,0,0,0.6)]"
                >
                  <SlidersHorizontal size={14} strokeWidth={2} />
                  Ouvrir
                </button>
              )}
              {navigable && !isGeneral && (
                <ChevronRight size={18} className="text-stone-400" />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};

