import React from "react";
import { ChevronLeft } from "lucide-react";
import type { ManuscriptLayoutConfig } from "../../types";
import { TRIM_PRESETS } from "../../data/trimSizes";
import { RadioRow } from "./controls";

interface FormatPageProps {
  config: ManuscriptLayoutConfig;
  onTrimChange: (presetId: string) => void;
  onBack: () => void;
}

/** Étape 3 — sélection du format de coupe KDP (gabarit d'export). */
export const FormatPage: React.FC<FormatPageProps> = ({
  config,
  onTrimChange,
  onBack,
}) => {
  return (
    <div className="h-full">
      <button
        type="button"
        onClick={onBack}
        className="mb-4 flex items-center gap-1 font-mono text-[11px] uppercase tracking-widest text-stone-500 transition hover:text-copper"
      >
        <ChevronLeft size={14} />
        Retour aux paramètres généraux
      </button>

      <h2 className="mb-6 border-b border-stone-400/40 pb-4 text-center font-serif text-2xl uppercase tracking-[0.25em] text-stone-700">
        Format du livre
      </h2>

      <div className="space-y-1.5">
        {TRIM_PRESETS.map((preset) => (
          <RadioRow
            key={preset.id}
            active={config.trimSize === preset.id}
            onClick={() => onTrimChange(preset.id)}
          >
            <span className="flex-1">
              <span className="block font-serif text-sm font-semibold text-stone-800">
                {preset.label}{" "}
                <span className="font-normal italic text-stone-500">
                  ({preset.cm})
                </span>
              </span>
              <span className="block text-xs text-stone-500">{preset.note}</span>
            </span>
          </RadioRow>
        ))}
      </div>
    </div>
  );
};
