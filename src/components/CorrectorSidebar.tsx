import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  CheckCheck,
  ChevronDown,
  Eraser,
  EyeOff,
  Loader2,
  LocateFixed,
  MapPin,
  RefreshCw,
  ShieldAlert,
} from "lucide-react";
import {
  addPlace,
  analyzeChapter,
  getCorrectorOptions,
  listIgnoredWords,
  setCorrectorOptions,
  updateIgnoredWords,
  type CorrectionMatch,
  type LtOptions,
} from "../utils/correctorApi";
import type { AppliedCorrection } from "../types";

interface CorrectorSidebarProps {
  /** Texte brut soumis à l'analyse (chapitre courant). */
  text: string;
  /**
   * Révision incrémentée par le parent après un remplacement : force une
   * **relance immédiate** de l'analyse (offsets recalculés, pas de débounce).
   */
  revision?: number;
  /** Applique un remplacement suggéré au texte source (éditeur temporaire). */
  onApplyReplacement?: (match: CorrectionMatch, replacement: string) => void;
  /** Remonte les correspondances courantes (décorations de l'éditeur). */
  onMatchesChange?: (matches: CorrectionMatch[]) => void;
  /** Index de la carte **active** (surlignée dans l'éditeur). */
  activeIndex?: number | null;
  /** Survol / focus d'une carte (`null` = plus aucune carte active). */
  onHoverMatch?: (index: number | null) => void;
  /** Cible une carte dans l'éditeur (« Localiser »). */
  onLocateMatch?: (index: number) => void;
  /** Journal de session des corrections appliquées (affiché sous l'en-tête). */
  history?: AppliedCorrection[];
}

/** Délai (ms) d'inactivité avant relance automatique de l'analyse. */
const ANALYZE_DEBOUNCE_MS = 800;

/** Extrait le fragment visé (offsets **en caractères**, comme le backend). */
const matchToken = (text: string, offset: number, length: number): string =>
  Array.from(text).slice(offset, offset + length).join("");

/** `true` si la règle est **orthographique** (MORFOLOGIK / SPELL / FRENCH_…). */
const isSpellingRule = (ruleId: string | null): boolean => {
  const id = (ruleId ?? "").toUpperCase();
  return id.includes("MORFOLOGIK") || id.includes("SPELL") || id.includes("FRENCH_");
};

/** Libellé lisible d'un remplacement (rend visibles les caractères blancs). */
const formatReplacement = (replacement: string): string => {
  if (replacement === "") {
    return "Supprimer";
  }
  if (replacement === " ") {
    return "⎵ 1 espace";
  }
  if (replacement.includes("\u00A0") || replacement.includes("\u202F")) {
    return "⎵ Espace insécable";
  }
  return replacement;
};

/** Découpe un contexte en [avant, cible, après] pour la mise en évidence. */
const highlight = (context: string, needle: string): [string, string, string] => {
  if (!needle) {
    return [context, "", ""];
  }
  const at = context.indexOf(needle);
  if (at < 0) {
    return [context, "", ""];
  }
  return [context.slice(0, at), needle, context.slice(at + needle.length)];
};

/**
 * Panneau latéral du **Correcteur** : liste les erreurs d'un chapitre.
 *
 * - relance l'analyse (débouncée) à chaque changement de texte ;
 * - met en évidence le mot fautif dans la phrase de contexte ;
 * - propose les remplacements et l'action « Ignorer » (dictionnaire local,
 *   retrait immédiat de la carte **sans** recharger l'API).
 */
export const CorrectorSidebar: React.FC<CorrectorSidebarProps> = ({
  text,
  revision = 0,
  onApplyReplacement,
  onMatchesChange,
  activeIndex = null,
  onHoverMatch,
  onLocateMatch,
  history = [],
}) => {
  const [matches, setMatches] = useState<CorrectionMatch[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const runRef = useRef(0);

  const analyze = useCallback(async () => {
    const run = ++runRef.current;
    if (!text.trim()) {
      setMatches([]);
      setError(null);
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const result = await analyzeChapter(text);
      if (runRef.current === run) {
        setMatches(result);
      }
    } catch (cause) {
      if (runRef.current === run) {
        setMatches([]);
        setError(cause instanceof Error ? cause.message : String(cause));
      }
    } finally {
      if (runRef.current === run) {
        setLoading(false);
      }
    }
  }, [text]);

  // Analyse **débouncée** sur frappe ; **immédiate** après un remplacement
  // (`revision` change) afin de recalculer les offsets sans délai.
  const revisionRef = useRef(revision);
  const cardRefs = useRef<(HTMLElement | null)[]>([]);
  // Cartes en cours d'animation de disparition (rétro) + journal déplié.
  const [exiting, setExiting] = useState<Set<string>>(new Set());
  const [historyOpen, setHistoryOpen] = useState(false);
  useEffect(() => {
    const immediate = revisionRef.current !== revision;
    revisionRef.current = revision;
    const handle = window.setTimeout(() => {
      void analyze();
    }, immediate ? 0 : ANALYZE_DEBOUNCE_MS);
    return () => window.clearTimeout(handle);
  }, [analyze, revision]);

  // Désactive une règle (persistée dans les options) et retire ses occurrences.
  const disableRule = useCallback(async (match: CorrectionMatch) => {
    const ruleId = match.rule_id;
    if (!ruleId) {
      return;
    }
    const key = `${match.offset}-${match.length}`;
    setBusy(key);
    try {
      const options = await getCorrectorOptions();
      const next: LtOptions = {
        ...options,
        disabled_rules: Array.from(new Set([...options.disabled_rules, ruleId])).sort(),
      };
      await setCorrectorOptions(next);
      setMatches((previous) => previous.filter((entry) => entry.rule_id !== ruleId));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }, []);

  // Ajoute le terme courant comme **toponyme / nom propre** et retire la carte.
  const addAsPlace = useCallback(
    async (match: CorrectionMatch) => {
      const word = matchToken(text, match.offset, match.length).trim();
      const key = `${match.offset}-${match.length}`;
      if (!word) {
        return;
      }
      setBusy(key);
      try {
        await addPlace(word);
        setMatches((previous) => previous.filter((entry) => entry !== match));
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : String(cause));
      } finally {
        setBusy(null);
      }
    },
    [text],
  );

  // Valide une suggestion : **animation rétro** puis retrait de la carte.
  const handleReplace = useCallback(
    (match: CorrectionMatch, replacement: string) => {
      const key = `${match.offset}-${match.length}`;
      setExiting((previous) => new Set(previous).add(key));
      onApplyReplacement?.(match, replacement);
      window.setTimeout(() => {
        setExiting((previous) => {
          const next = new Set(previous);
          next.delete(key);
          return next;
        });
        setMatches((previous) => previous.filter((entry) => entry !== match));
      }, 380);
    },
    [onApplyReplacement],
  );

  const ignore = useCallback(
    async (match: CorrectionMatch) => {
      const word = matchToken(text, match.offset, match.length).trim();
      const key = `${match.offset}-${match.length}`;
      if (!word) {
        return;
      }
      setBusy(key);
      try {
        const current = await listIgnoredWords();
        const next = Array.from(new Set([...current, word.toLowerCase()])).sort();
        await updateIgnoredWords(next);
        setMatches((previous) => previous.filter((entry) => entry !== match));
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : String(cause));
      } finally {
        setBusy(null);
      }
    },
    [text],
  );

  // Remonte les correspondances au parent (décorations in-situ de l'éditeur).
  useEffect(() => {
    onMatchesChange?.(matches);
  }, [matches, onMatchesChange]);

  // Défilement automatique vers la carte active (clic dans l'éditeur).
  useEffect(() => {
    if (activeIndex === null) {
      return;
    }
    cardRefs.current[activeIndex]?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, [activeIndex]);

  return (
    <aside className="flex min-h-0 flex-1 flex-col rounded-sm border border-stone-400/30 bg-parchment-dark/50">
      <header className="flex items-center justify-between gap-3 border-b border-stone-400/30 px-4 py-2.5">
        <span className="flex items-center gap-2 font-mono text-[11px] uppercase tracking-widest text-stone-600">
          <ShieldAlert size={15} strokeWidth={1.75} className="text-copper" />
          {loading ? "Analyse en cours…" : `${matches.length} suggestion(s)`}
        </span>
        <button
          type="button"
          onClick={() => void analyze()}
          disabled={loading || !text.trim()}
          className="flex items-center gap-1.5 rounded-sm border border-stone-400/40 px-2.5 py-1 text-[10px] font-medium uppercase tracking-widest text-stone-600 transition-colors hover:bg-copper hover:text-amber-50 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <RefreshCw size={13} strokeWidth={1.75} /> Relancer
        </button>
      </header>

      {/* Journal des opérations — corrections validées durant la session. */}
      <div className="border-b border-stone-400/30">
        <button
          type="button"
          onClick={() => setHistoryOpen((open) => !open)}
          aria-expanded={historyOpen}
          className="flex w-full items-center justify-between gap-2 px-4 py-2 text-left transition-colors hover:bg-stone-500/10"
        >
          <span className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-stone-500">
            <CheckCheck size={13} strokeWidth={1.75} className="text-verdigris" />
            Journal des opérations ({history.length})
          </span>
          <ChevronDown
            size={13}
            className={`text-stone-400 transition-transform ${historyOpen ? "rotate-180" : ""}`}
          />
        </button>
        {historyOpen && (
          <div className="max-h-40 overflow-y-auto px-4 pb-3">
            {history.length === 0 ? (
              <p className="font-serif text-xs italic text-stone-500">
                Aucune correction appliquée dans cette session
              </p>
            ) : (
              <ul className="space-y-1">
                {history.map((entry) => (
                  <li key={entry.id} className="flex items-center gap-2 text-[11px]">
                    <del className="line-through text-stone-400">
                      {entry.original ? formatReplacement(entry.original) : "∅"}
                    </del>
                    <span className="text-stone-400">&rarr;</span>
                    <span className="font-medium text-emerald-800">
                      {formatReplacement(entry.replacement)}
                    </span>
                    <span className="ml-auto font-mono text-[10px] tabular-nums text-stone-400">
                      {entry.timestamp.toLocaleTimeString("fr-FR", {
                        hour: "2-digit",
                        minute: "2-digit",
                      })}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
      </div>

      <div className="min-h-0 flex-1 space-y-2 overflow-y-auto p-3">
        {loading && matches.length === 0 && (
          <div className="flex items-center justify-center gap-2 py-8 text-stone-500">
            <Loader2 size={18} className="animate-spin text-copper" />
            <span className="font-mono text-[11px] uppercase tracking-widest">Analyse…</span>
          </div>
        )}

        {error && (
          <p className="rounded-sm border border-red-400/40 bg-red-500/10 px-3 py-2 text-xs text-red-800">
            {error}
          </p>
        )}

        {!loading && !error && matches.length === 0 && (
          <p className="px-1 py-6 text-center text-xs italic text-stone-500">
            {text.trim() ? "Aucune correction détectée." : "Saisissez un chapitre à analyser."}
          </p>
        )}

        {matches.map((match, index) => {
          const busyKey = `${match.offset}-${match.length}`;
          const needle = matchToken(text, match.offset, match.length);
          const [before, focus, after] = highlight(match.context ?? "", needle);
          return (
            <article
              key={`${index}-${busyKey}`}
              ref={(element) => {
                cardRefs.current[index] = element;
              }}
              tabIndex={0}
              onMouseEnter={() => onHoverMatch?.(index)}
              onMouseLeave={() => onHoverMatch?.(null)}
              onFocus={() => onHoverMatch?.(index)}
              onBlur={() => onHoverMatch?.(null)}
              className={`overflow-hidden rounded-sm border shadow-sm transition-all duration-300 ease-out ${
                exiting.has(busyKey)
                  ? "max-h-0 -translate-x-2 scale-95 bg-copper/10 p-0 opacity-0"
                  : `max-h-[40rem] p-3 opacity-100 ${
                      activeIndex === index
                        ? "border-copper/60 bg-parchment ring-1 ring-copper/40"
                        : "border-stone-400/30 bg-parchment/85"
                    }`
              }`}
            >
              <p className="text-xs font-medium text-stone-800">{match.message}</p>

              {match.context && (
                <p className="mt-1.5 font-serif text-[13px] leading-relaxed text-stone-600">
                  {before}
                  {focus && (
                    <mark className="rounded-sm bg-copper/25 px-0.5 font-semibold text-stone-900">
                      {focus}
                    </mark>
                  )}
                  {after}
                </p>
              )}

              <div className="mt-2 flex flex-wrap items-center gap-1.5">
                {match.replacements.slice(0, 4).map((replacement, position) => (
                  <button
                    key={`${position}-${replacement}`}
                    type="button"
                    onClick={() => handleReplace(match, replacement)}
                    disabled={!onApplyReplacement || exiting.has(busyKey)}
                    className="min-h-[26px] min-w-[3rem] rounded-sm border border-verdigris/60 bg-verdigris/10 px-2.5 py-1 text-xs font-medium text-stone-700 transition-colors hover:border-emerald-700 hover:bg-emerald-700/15 disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    {formatReplacement(replacement)}
                  </button>
                ))}
                <span className="ml-auto flex items-center gap-1.5">
                  <button
                    type="button"
                    onClick={() => {
                      onHoverMatch?.(index);
                      onLocateMatch?.(index);
                    }}
                    disabled={!onLocateMatch}
                    title="Centrer l'éditeur sur cette correction"
                    className="flex items-center gap-1 rounded-sm border border-stone-400/40 px-2 py-0.5 text-[10px] font-medium uppercase tracking-widest text-stone-500 transition-colors hover:bg-stone-500/15 disabled:opacity-50"
                  >
                    <LocateFixed size={12} strokeWidth={1.75} /> Localiser
                  </button>
                  {isSpellingRule(match.rule_id) && (
                    <button
                      type="button"
                      onClick={() => void addAsPlace(match)}
                      disabled={busy === busyKey}
                      title="Ajouter comme lieu / nom propre"
                      className="flex items-center gap-1 rounded-sm border border-stone-400/40 px-2 py-0.5 text-[10px] font-medium uppercase tracking-widest text-stone-500 transition-colors hover:bg-stone-500/15 disabled:opacity-50"
                    >
                      <MapPin size={12} strokeWidth={1.75} /> Lieu
                    </button>
                  )}
                  {match.rule_id && (
                    <button
                      type="button"
                      onClick={() => void disableRule(match)}
                      disabled={busy === busyKey}
                      title={
                        match.rule_description
                          ? `Désactiver la règle : « ${match.rule_description} »`
                          : "Ne plus appliquer cette règle"
                      }
                      className="flex items-center gap-1 rounded-sm border border-stone-400/40 px-2 py-0.5 text-[10px] font-medium uppercase tracking-widest text-stone-500 transition-colors hover:bg-stone-500/15 disabled:opacity-50"
                    >
                      <EyeOff size={12} strokeWidth={1.75} /> Règle
                    </button>
                  )}
                  <button
                    type="button"
                    onClick={() => void ignore(match)}
                    disabled={busy === busyKey}
                    className="flex items-center gap-1 rounded-sm border border-stone-400/40 px-2 py-0.5 text-[10px] font-medium uppercase tracking-widest text-stone-500 transition-colors hover:bg-stone-500/15 disabled:opacity-50"
                  >
                    {busy === busyKey ? (
                      <Loader2 size={12} className="animate-spin" />
                    ) : (
                      <Eraser size={12} strokeWidth={1.75} />
                    )}
                    Ignorer
                  </button>
                </span>
              </div>
            </article>
          );
        })}
      </div>
    </aside>
  );
};
