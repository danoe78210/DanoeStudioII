import React, { forwardRef, useCallback, useImperativeHandle, useMemo, useRef } from "react";
import type { CorrectionMatch } from "../utils/correctorApi";

interface ChapterEditorProps {
  /** Texte du chapitre (source de l'analyse). */
  text: string;
  /** Correspondances à souligner (offsets en caractères). */
  matches: CorrectionMatch[];
  /** Index de la correspondance surlignée (survol d'une carte). */
  activeIndex: number | null;
  /** Index de la faute **ciblée** par « Localiser » (surlignage rouge vif). */
  focusedIndex?: number | null;
  /** Mise à jour du texte (saisie manuelle). */
  onChange: (value: string) => void;
  /** Clic sur une faute soulignée. */
  onSelectMatch: (index: number) => void;
  /** Indication quand le champ est vide. */
  placeholder?: string;
}

/** Méthode impérative exposée au parent via `ref`. */
export interface ChapterEditorHandle {
  /** Centre l'éditeur sur la faute `index`, la sélectionne et la surligne. */
  locate: (index: number) => void;
}

/** Couleur de soulignement selon la règle déclenchée. */
const decorationOf = (ruleId: string | null): string => {
  const id = (ruleId ?? "").toUpperCase();
  if (id.includes("MORFOLOGIK") || id.includes("SPELL") || id.includes("FRENCH_")) {
    return "decoration-red-500"; // orthographe
  }
  if (
    id.includes("STYLE") ||
    id.includes("REDUND") ||
    id.includes("WORDINESS") ||
    id.includes("TYPO")
  ) {
    return "decoration-amber-500"; // style / typographie
  }
  return "decoration-sky-500"; // grammaire / sémantique
};

/**
 * Éditeur de chapitre **décoré in-situ** : les fautes sont soulignées (ondulé,
 * colorées par catégorie) au calque exact des `offset`/`length` de l'analyse.
 *
 * Technique : un `<textarea>` (texte transparent, caret et sélection natifs)
 * superposé à un calque miroir qui porte le texte visible + les soulignements.
 * Le clic dans la zone de saisie est mappé au caractère (`selectionStart`), ce
 * qui identifie la faute touchée sans intercepter la frappe.
 */
export const ChapterEditor = forwardRef<ChapterEditorHandle, ChapterEditorProps>(
  (
    { text, matches, activeIndex, focusedIndex = null, onChange, onSelectMatch, placeholder },
    ref,
  ) => {
  const areaRef = useRef<HTMLTextAreaElement>(null);
  const mirrorRef = useRef<HTMLDivElement>(null);
  // Références des nœuds de soulignement (pour le défilement de « Localiser »).
  const markRefs = useRef<(HTMLElement | null)[]>([]);

  const segments = useMemo(() => {
    const chars = Array.from(text);
    const nodes: React.ReactNode[] = [];
    let cursor = 0;
    matches.forEach((match, index) => {
      const start = match.offset;
      const end = match.offset + match.length;
      if (start < cursor || start >= chars.length) {
        return; // chevauchement ou hors bornes : ignoré
      }
      if (start > cursor) {
        nodes.push(chars.slice(cursor, start).join(""));
      }
      const token = chars.slice(start, Math.min(end, chars.length)).join("");
      nodes.push(
        <mark
          key={`${index}-${start}`}
          data-match-index={index}
          ref={(element) => {
            markRefs.current[index] = element;
          }}
          className={`bg-transparent underline decoration-wavy decoration-2 ${decorationOf(
            match.rule_id,
          )} ${
            focusedIndex === index
              ? "rounded-xs bg-red-500/35 text-stone-900 ring-2 ring-red-500/70"
              : activeIndex === index
                ? "bg-copper/30"
                : ""
          }`}
        >
          {token}
        </mark>,
      );
      cursor = Math.min(end, chars.length);
    });
    if (cursor < chars.length) {
      nodes.push(chars.slice(cursor).join(""));
    }
    return nodes;
  }, [text, matches, activeIndex, focusedIndex]);

  // Centrage impératif demandé par « Localiser » (via le `ref` du parent).
  useImperativeHandle(
    ref,
    () => ({
      locate(index: number) {
        const area = areaRef.current;
        const mirror = mirrorRef.current;
        if (!area || !mirror) {
          return;
        }
        const target = markRefs.current[index];
        if (target) {
          // Défilement direct du `<textarea>` (la cible est centrée verticalement).
          area.scrollTo({
            top: Math.max(0, target.offsetTop - area.clientHeight / 2),
            behavior: "smooth",
          });
        }
        const match = matches[index];
        if (match) {
          area.focus();
          area.setSelectionRange(match.offset, match.offset + match.length);
        }
        // Calque miroir aligné sur le champ (immédiat, puis à chaque animation).
        mirror.scrollTop = area.scrollTop;
        window.requestAnimationFrame(() => {
          mirror.scrollTop = area.scrollTop;
        });
      },
    }),
    [matches],
  );

  const handleClick = useCallback(() => {
    const area = areaRef.current;
    if (!area) {
      return;
    }
    const position = area.selectionStart ?? 0;
    const index = matches.findIndex(
      (match) => position >= match.offset && position <= match.offset + match.length,
    );
    if (index >= 0) {
      onSelectMatch(index);
    }
  }, [matches, onSelectMatch]);

  const handleScroll = useCallback(() => {
    const area = areaRef.current;
    const mirror = mirrorRef.current;
    if (area && mirror) {
      mirror.scrollTop = area.scrollTop;
    }
  }, []);

  return (
    <div className="relative h-64 overflow-hidden rounded-sm border border-stone-400/30 bg-parchment/80 shadow-[inset_0_2px_6px_rgba(0,0,0,0.06)]">
      <div
        aria-hidden
        ref={mirrorRef}
        className="pointer-events-none absolute inset-0 overflow-hidden p-3 font-serif text-sm leading-relaxed whitespace-pre-wrap break-words text-stone-800"
      >
        {segments}
      </div>
      <textarea
        ref={areaRef}
        value={text}
        spellCheck={false}
        onChange={(event) => onChange(event.target.value)}
        onClick={handleClick}
        onScroll={handleScroll}
        placeholder={placeholder}
        className="absolute inset-0 h-full w-full resize-none bg-transparent p-3 font-serif text-sm leading-relaxed text-transparent caret-stone-900 placeholder:text-stone-400 selection:bg-copper/30 focus:outline-none"
      />
    </div>
    );
  },
);

ChapterEditor.displayName = "ChapterEditor";
