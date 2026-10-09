import React, { useCallback, useEffect, useRef, useState } from "react";
import { Check, Eraser, Loader2, PanelRight, SpellCheck } from "lucide-react";
import { useStudio } from "./StudioContext";
import { CorrectorSidebar } from "./CorrectorSidebar";
import { ChapterEditor, type ChapterEditorHandle } from "./ChapterEditor";
import { readChapterFile, writeChapterFile } from "../utils/projectApi";
import type { CorrectionMatch } from "../utils/correctorApi";
import type { AppliedCorrection } from "../types";

/**
 * Page **Correcteur** du livre virtuel.
 *
 * Câblage temporaire (Phase 2) : le texte brut du chapitre actif est saisi /
 * collé dans la zone centrale ; les remplacements suggérés y sont appliqués
 * directement. L'intégration des soulignements **dans** l'éditeur relève de la Phase 3.
 */
export const CorrectorView: React.FC = () => {
  const { chapterFiles } = useStudio();
  const [chapter, setChapter] = useState("");
  const [text, setText] = useState("");
  const [loadingChapter, setLoadingChapter] = useState(false);
  const [readError, setReadError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const saveTimer = useRef<number | null>(null);
  // Révision incrémentée à chaque remplacement → relance immédiate de l'analyse.
  const [revision, setRevision] = useState(0);
  // Valeur **exacte** produite par le callback d'état, en attente de sauvegarde.
  const pendingPersist = useRef<string | null>(null);
  // Correspondances remontées par la sidebar → décorations de l'éditeur in-situ.
  const [matches, setMatches] = useState<CorrectionMatch[]>([]);
  const [activeMatch, setActiveMatch] = useState<number | null>(null);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  // Journal de session des corrections appliquées (visible depuis l'en-tête).
  const [history, setHistory] = useState<AppliedCorrection[]>([]);
  // Correction produite par le callback d'état, en attente d'enregistrement.
  const pendingHistory = useRef<AppliedCorrection | null>(null);
  // Éditeur (méthode impérative) + faute ciblée par « Localiser ».
  const editorRef = useRef<ChapterEditorHandle | null>(null);
  const [focusedMatch, setFocusedMatch] = useState<number | null>(null);

  /** Charge le texte brut du chapitre sélectionné (lecture native du fichier). */
  const handleChapterChange = useCallback(async (file: string) => {
    setChapter(file);
    setReadError(null);
    if (!file) {
      setText("");
      return;
    }
    setLoadingChapter(true);
    try {
      setText(await readChapterFile(file));
    } catch (cause) {
      setText("");
      setReadError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoadingChapter(false);
    }
  }, []);

  const clearSaveTimer = useCallback(() => {
    if (saveTimer.current !== null) {
      window.clearTimeout(saveTimer.current);
      saveTimer.current = null;
    }
  }, []);

  // Nettoyage du minuteur au démontage.
  useEffect(() => clearSaveTimer, [clearSaveTimer]);

  /** Sauvegarde atomique du chapitre courant + retour visuel « Sauvegardé ». */
  const persist = useCallback(
    async (next: string) => {
      if (!chapter) {
        return;
      }
      try {
        await writeChapterFile(chapter, next);
        setSaved(true);
        clearSaveTimer();
        saveTimer.current = window.setTimeout(() => {
          setSaved(false);
          saveTimer.current = null;
        }, 2000);
      } catch (cause) {
        setReadError(cause instanceof Error ? cause.message : String(cause));
      }
    },
    [chapter, clearSaveTimer],
  );

  /**
   * Applique un remplacement (offsets en caractères) **sans dépendre de la
   * fermeture de `text`** : la valeur corrigée est calculée depuis la valeur
   * courante du callback d'état (aucun écrasement lors de clics rapides). La
   * valeur exacte est mémorisée pour la sauvegarde, et `revision` force la
   * sidebar à relancer l'analyse (offsets recalculés).
   */
  const applyReplacement = useCallback((match: CorrectionMatch, replacement: string) => {
    setRevision((value) => value + 1);
    setText((current) => {
      const chars = Array.from(current);
      const original = chars.slice(match.offset, match.offset + match.length).join("");
      const next = [
        ...chars.slice(0, match.offset),
        ...Array.from(replacement),
        ...chars.slice(match.offset + match.length),
      ].join("");
      pendingPersist.current = next;
      pendingHistory.current = {
        id: `${Date.now()}-${match.offset}-${match.length}`,
        original,
        replacement,
        timestamp: new Date(),
      };
      return next;
    });
  }, []);

  // Sauvegarde la **valeur exacte** issue du callback d'état (après commit),
  // puis consigne la correction au journal de session.
  useEffect(() => {
    const next = pendingPersist.current;
    if (next === null) {
      return;
    }
    pendingPersist.current = null;
    const entry = pendingHistory.current;
    pendingHistory.current = null;
    void persist(next).then(() => {
      if (entry) {
        setHistory((previous) => [entry, ...previous]);
      }
    });
  }, [text, persist]);

  /** Clic sur une faute : ouvre le panneau et cible la carte correspondante. */
  const handleSelectMatch = useCallback((index: number) => {
    setSidebarOpen(true);
    setActiveMatch(index);
  }, []);

  /** « Localiser » : centre l'éditeur sur la faute et la met en évidence. */
  const handleLocateMatch = useCallback((index: number) => {
    setSidebarOpen(true);
    setActiveMatch(index);
    setFocusedMatch(index);
    editorRef.current?.locate(index);
  }, []);

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <header className="flex items-center gap-3 border-b border-stone-400/40 pb-3">
        <SpellCheck size={20} strokeWidth={1.75} className="text-copper" />
        <h2 className="flex-1 font-serif text-2xl uppercase tracking-[0.25em] text-stone-700">
          Correcteur
        </h2>
        <button
          type="button"
          onClick={() => setSidebarOpen((open) => !open)}
          aria-pressed={sidebarOpen}
          title="Panneau d'analyse"
          className={`flex items-center rounded-sm border border-stone-400/40 p-1.5 transition-colors hover:bg-copper hover:text-amber-50 ${
            sidebarOpen ? "text-copper" : "text-stone-600"
          }`}
        >
          <PanelRight size={13} strokeWidth={1.75} />
        </button>
        {loadingChapter && (
          <Loader2
            size={15}
            className="animate-spin text-copper"
            aria-label="Chargement du chapitre"
          />
        )}
        <select
          value={chapter}
          onChange={(event) => void handleChapterChange(event.target.value)}
          disabled={loadingChapter}
          className="max-w-[12rem] rounded-sm border border-stone-400/40 bg-parchment px-2 py-1 text-xs text-stone-700 disabled:opacity-60"
        >
          <option value="">Chapitre…</option>
          {chapterFiles.map((file) => (
            <option key={file} value={file}>
              {file}
            </option>
          ))}
        </select>
        <button
          type="button"
          onClick={() => setText("")}
          disabled={!text || loadingChapter}
          className="flex items-center gap-1.5 rounded-sm border border-stone-400/40 px-2.5 py-1 text-[10px] font-medium uppercase tracking-widest text-stone-600 transition-colors hover:bg-stone-500/15 disabled:opacity-50"
        >
          <Eraser size={13} strokeWidth={1.75} /> Vider
        </button>
        {saved && (
          <span
            aria-live="polite"
            className="flex items-center gap-1 text-[10px] font-medium uppercase tracking-widest text-verdigris"
          >
            <Check size={13} strokeWidth={2.25} /> Sauvegardé
          </span>
        )}
      </header>

      {readError && (
        <p className="rounded-sm border border-red-400/40 bg-red-500/10 px-3 py-2 text-xs text-red-800">
          {readError}
        </p>
      )}

      <ChapterEditor
        ref={editorRef}
        text={text}
        matches={matches}
        activeIndex={activeMatch}
        focusedIndex={focusedMatch}
        onChange={setText}
        onSelectMatch={handleSelectMatch}
        placeholder={
          chapter
            ? `Texte brut de « ${chapter} »…`
            : "Collez le texte brut du chapitre à corriger…"
        }
      />

      {sidebarOpen && (
        <CorrectorSidebar
          text={text}
          revision={revision}
          activeIndex={activeMatch}
          onHoverMatch={setActiveMatch}
          onMatchesChange={setMatches}
          onLocateMatch={handleLocateMatch}
          onApplyReplacement={applyReplacement}
          history={history}
        />
      )}
    </div>
  );
};
