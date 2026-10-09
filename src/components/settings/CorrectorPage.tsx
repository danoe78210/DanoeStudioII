import React, { useCallback, useEffect, useState } from "react";
import { ChevronLeft, HelpCircle, RotateCcw, Settings2 } from "lucide-react";
import {
  getCorrectorOptions,
  setCorrectorOptions,
  type LtOptions,
} from "../../utils/correctorApi";
import { PillButton } from "./controls";

interface CorrectorPageProps {
  onBack: () => void;
}

/** Régions / dialectes français proposés. */
const REGIONS: { value: string; label: string }[] = [
  { value: "fr", label: "Français standard (fr)" },
  { value: "fr-FR", label: "France (fr-FR)" },
  { value: "fr-BE", label: "Belgique (fr-BE)" },
  { value: "fr-CA", label: "Canada (fr-CA)" },
  { value: "fr-CH", label: "Suisse (fr-CH)" },
];

/** Options par défaut (repli hors Tauri). */
const DEFAULT_OPTIONS: LtOptions = { language: "fr", picky: false, disabled_rules: [] };

/** Rend un identifiant de règle **lisible** (jamais le jargon technique brut). */
const humanize = (ruleId: string): string => {
  const words = ruleId.replace(/[_-]+/g, " ").trim().toLowerCase();
  return words.charAt(0).toUpperCase() + words.slice(1);
};

/**
 * Infobulle d'aide discrète (icône `?`) : panneau flottant au survol / focus,
 * dans un langage simple orienté relecture littéraire.
 */
const InfoTooltip: React.FC<{ text: string }> = ({ text }) => (
  <span className="group relative ml-1.5 inline-flex align-middle">
    <span
      role="button"
      tabIndex={0}
      aria-label="Aide"
      className="flex items-center text-stone-400 transition-colors hover:text-copper focus:text-copper focus:outline-none"
    >
      <HelpCircle size={13} strokeWidth={1.75} />
    </span>
    <span
      role="tooltip"
      className="pointer-events-none absolute top-full left-1/2 z-30 mt-2 w-72 -translate-x-1/2 rounded-sm border border-stone-400/50 bg-parchment px-3 py-2 text-left font-serif text-[11px] leading-relaxed tracking-normal text-stone-700 normal-case opacity-0 shadow-[0_8px_20px_rgba(0,0,0,0.25)] transition-opacity duration-150 group-hover:opacity-100 group-focus-within:opacity-100"
    >
      {text}
    </span>
  </span>
);

/**
 * Page « Correcteur linguistique » — options globales LanguageTool : région /
 * dialecte, mode pointilleux, et règles désactivées (avec réactivation).
 */
export const CorrectorPage: React.FC<CorrectorPageProps> = ({ onBack }) => {
  const [options, setOptions] = useState<LtOptions>(DEFAULT_OPTIONS);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const loaded = await getCorrectorOptions();
        if (!cancelled) {
          setOptions(loaded);
        }
      } catch (cause) {
        if (!cancelled) {
          setError(cause instanceof Error ? cause.message : String(cause));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const persist = useCallback(async (next: LtOptions) => {
    setOptions(next);
    try {
      await setCorrectorOptions(next);
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

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
        Correcteur linguistique
      </h2>

      {/* Section 1 — Région / dialecte */}
      <h3 className="mb-1.5 flex items-center font-mono text-[10px] uppercase tracking-widest text-stone-500">
        Région / dialecte
        <InfoTooltip text="Adapte les règles de correction aux particularités de votre pays : conventions d'espaces autour de la ponctuation en France, ou vocabulaire et expressions spécifiques au Québec, à la Belgique et à la Suisse." />
      </h3>
      <p className="mb-2 font-serif text-xs italic text-stone-500">
        Adapte le correcteur à l'usage régional du français.
      </p>
      <div className="mb-5 flex flex-wrap gap-2">
        {REGIONS.map((region) => (
          <PillButton
            key={region.value}
            active={options.language === region.value}
            onClick={() => void persist({ ...options, language: region.value })}
          >
            {region.label}
          </PillButton>
        ))}
      </div>

      {/* Section 2 — Mode pointilleux */}
      <h3 className="mb-2 font-mono text-[10px] uppercase tracking-widest text-stone-500">
        Niveau d'exigence
      </h3>
      <button
        type="button"
        role="switch"
        aria-checked={options.picky}
        onClick={() => void persist({ ...options, picky: !options.picky })}
        className="flex w-full max-w-xl items-center justify-between gap-3 rounded-sm border border-stone-400/30 bg-stone-500/5 px-3 py-2 text-left transition-colors hover:bg-stone-500/10"
      >
        <span className="text-xs text-stone-700">
          <span className="flex items-center">
            Mode Pointilleux
            <InfoTooltip text="Recommandé pour le travail littéraire. Va plus loin que la simple orthographe en traquant les répétitions, les pléonasmes, les phrases trop lourdes ou passives, et les maladresses de style." />
          </span>
          <span className="block text-[10px] text-stone-500">
            Style, typographie et sémantique avancée
          </span>
        </span>
        <span
          className={`relative h-4 w-8 shrink-0 rounded-full transition-colors ${
            options.picky ? "bg-copper" : "bg-stone-400/50"
          }`}
        >
          <span
            className={`absolute top-0.5 h-3 w-3 rounded-full bg-parchment transition-all ${
              options.picky ? "left-4" : "left-0.5"
            }`}
          />
        </span>
      </button>

      {/* Section 3 — Règles désactivées */}
      <h3 className="mb-2 mt-5 flex items-center font-mono text-[10px] uppercase tracking-widest text-stone-500">
        Règles désactivées
        <InfoTooltip text="Retrouvez ici les types d'erreurs que vous avez choisi de ne plus signaler pendant votre correction. Cliquez sur « Réactiver » pour qu'une règle soit de nouveau vérifiée dans vos chapitres." />
      </h3>
      {options.disabled_rules.length === 0 ? (
        <p className="font-serif text-xs italic text-stone-500">Aucune règle désactivée.</p>
      ) : (
        <ul className="max-w-xl space-y-1.5">
          {options.disabled_rules.map((ruleId) => (
            <li
              key={ruleId}
              className="flex items-center gap-3 rounded-sm border border-stone-400/30 bg-stone-500/5 px-3 py-2"
            >
              <Settings2 size={13} strokeWidth={1.75} className="shrink-0 text-stone-400" />
              <span className="flex-1 truncate text-xs text-stone-700">{humanize(ruleId)}</span>
              <button
                type="button"
                onClick={() =>
                  void persist({
                    ...options,
                    disabled_rules: options.disabled_rules.filter((id) => id !== ruleId),
                  })
                }
                className="flex items-center gap-1.5 rounded-sm border border-stone-400/40 px-2 py-1 font-mono text-[10px] uppercase tracking-widest text-stone-600 transition-colors hover:bg-copper hover:text-amber-50"
              >
                <RotateCcw size={12} strokeWidth={2} />
                Réactiver
              </button>
            </li>
          ))}
        </ul>
      )}

      {loading && (
        <p className="mt-3 font-mono text-[10px] uppercase tracking-widest text-stone-400">
          Chargement…
        </p>
      )}
      {error && (
        <p className="mt-3 max-w-xl rounded-sm border border-red-400/40 bg-red-500/10 px-3 py-2 text-xs text-red-800">
          {error}
        </p>
      )}
    </div>
  );
};
