import React from "react";
import { ChevronLeft } from "lucide-react";
import type { ManuscriptLayoutConfig } from "../../types";
import {
  FONT_FAMILIES,
  INHERIT_FONT,
  INHERIT_LABEL,
  SUBTITLE_SIZES,
  numberFr,
} from "../../data/fonts";
import { FontCard, FontRadioRow, PillButton } from "./controls";

interface SubtitlePageProps {
  config: ManuscriptLayoutConfig;
  onChange: (patch: Partial<ManuscriptLayoutConfig>) => void;
  onBack: () => void;
}

/** Étape — typographie des sous-titres (gabarit d'export). */
export const SubtitlePage: React.FC<SubtitlePageProps> = ({
  config,
  onChange,
  onBack,
}) => {
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

      <h2 className="mb-4 border-b border-stone-400/40 pb-3 text-center font-serif text-2xl uppercase tracking-[0.25em] text-stone-700">
        Sous-titres
      </h2>

      {/* Taille */}
      <h3 className="mb-1.5 font-mono text-[10px] uppercase tracking-widest text-stone-500">
        Taille
      </h3>
      <div className="mb-4 flex flex-wrap gap-2">
        {SUBTITLE_SIZES.map((size) => (
          <PillButton
            key={size}
            active={size === config.subtitleSize}
            onClick={() => onChange({ subtitleSize: size })}
          >
            {numberFr(size)} pt
          </PillButton>
        ))}
      </div>

      {/* Police — option d'héritage puis grille compacte 2 colonnes */}
      <h3 className="mb-1.5 font-mono text-[10px] uppercase tracking-widest text-stone-500">
        Police
      </h3>
      <div className="space-y-2">
        <FontRadioRow
          active={config.subtitleFont === INHERIT_FONT}
          onClick={() => onChange({ subtitleFont: INHERIT_FONT })}
          name={INHERIT_LABEL}
        />
        <div className="grid grid-cols-2 gap-2">
          {FONT_FAMILIES.map((font) => (
            <FontCard
              key={font.name}
              active={font.name === config.subtitleFont}
              onClick={() => onChange({ subtitleFont: font.name })}
              name={font.name}
              stack={font.stack}
            />
          ))}
        </div>
      </div>
    </div>
  );
};
