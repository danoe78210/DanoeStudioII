import React from "react";
import { ChevronLeft } from "lucide-react";
import type { ManuscriptLayoutConfig } from "../../types";
import {
  BODY_FONT_SIZES,
  FONT_FAMILIES,
  LINE_SPACINGS,
  numberFr,
} from "../../data/fonts";
import { FontCard, PillButton } from "./controls";

interface BodyTextPageProps {
  config: ManuscriptLayoutConfig;
  onChange: (patch: Partial<ManuscriptLayoutConfig>) => void;
  onBack: () => void;
}

/** Étape — configuration fine du corps de texte (gabarit d'export). */
export const BodyTextPage: React.FC<BodyTextPageProps> = ({
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
        Corps du texte
      </h2>

      {/* Taille */}
      <h3 className="mb-1.5 font-mono text-[10px] uppercase tracking-widest text-stone-500">
        Taille
      </h3>
      <div className="mb-4 flex flex-wrap gap-2">
        {BODY_FONT_SIZES.map((size) => (
          <PillButton
            key={size}
            active={size === config.bodySize}
            onClick={() => onChange({ bodySize: size })}
            title={size === 11 ? "Standard recommandé pour la lecture" : undefined}
          >
            {numberFr(size)} pt
          </PillButton>
        ))}
      </div>

      {/* Police — grille compacte 2 colonnes, sans ascenseur */}
      <h3 className="mb-1.5 font-mono text-[10px] uppercase tracking-widest text-stone-500">
        Police
      </h3>
      <div className="mb-4 grid grid-cols-2 gap-2">
        {FONT_FAMILIES.map((font) => (
          <FontCard
            key={font.name}
            active={font.name === config.bodyFont}
            onClick={() => onChange({ bodyFont: font.name })}
            name={font.name}
            stack={font.stack}
          />
        ))}
      </div>

      {/* Interligne */}
      <h3 className="mb-1.5 font-mono text-[10px] uppercase tracking-widest text-stone-500">
        Interligne
      </h3>
      <div className="mb-4 flex flex-wrap gap-2">
        {LINE_SPACINGS.map((spacing) => (
          <PillButton
            key={spacing}
            active={spacing === config.lineSpacing}
            onClick={() => onChange({ lineSpacing: spacing })}
          >
            {numberFr(spacing)}
          </PillButton>
        ))}
      </div>

      {/* Justification + Lettrine — sur une même rangée pour gagner en hauteur */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <h3 className="mb-1.5 font-mono text-[10px] uppercase tracking-widest text-stone-500">
            Justification
          </h3>
          <div className="flex flex-wrap gap-2">
            <PillButton
              active={config.textAlignment === "left"}
              onClick={() => onChange({ textAlignment: "left" })}
            >
              Gauche
            </PillButton>
            <PillButton
              active={config.textAlignment === "justify"}
              onClick={() => onChange({ textAlignment: "justify" })}
            >
              Justifié
            </PillButton>
          </div>
        </div>

        <div>
          <h3 className="mb-1.5 font-mono text-[10px] uppercase tracking-widest text-stone-500">
            Lettrine (grand D initial)
          </h3>
          <div className="flex flex-wrap gap-2">
            <PillButton
              active={config.dropCap}
              onClick={() => onChange({ dropCap: true })}
            >
              Oui
            </PillButton>
            <PillButton
              active={!config.dropCap}
              onClick={() => onChange({ dropCap: false })}
            >
              Non
            </PillButton>
          </div>
        </div>
      </div>
    </div>
  );
};
