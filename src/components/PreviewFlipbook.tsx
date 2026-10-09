import React, { useCallback, useEffect, useRef, useState } from "react";
import { AlertTriangle, ChevronLeft, ChevronRight, Loader2, X } from "lucide-react";
import HTMLFlipBook from "react-pageflip";
import * as pdfjsLib from "pdfjs-dist";
import pdfWorkerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import { renderPdfBytes } from "../utils/pdfPreview";

// Worker pdf.js servi par Vite (asset dédié) — indispensable sous WebView2.
pdfjsLib.GlobalWorkerOptions.workerSrc = pdfWorkerUrl;

/** Résolution de rendu : 72 dpi × facteur (netteté d'impression, mémoire maîtrisée). */
const RENDER_SCALE = 2;
/** Largeur (px) sous laquelle le livre bascule en **simple page** (petit écran). */
const TWO_PAGE_MIN_WIDTH = 720;
/** Durée (ms) de l'animation de page tournée. */
const FLIPPING_TIME_MS = 700;
/** Pages rasterisées immédiatement à l'ouverture (démarrage instantané). */
const INITIAL_PAGES = 4;
/** Rayon du buffer de pré-chargement autour de la page courante (±2 pages). */
const LAZY_RADIUS = 2;

interface PreviewFlipbookProps {
  /** Payload projet complet (identique aux commandes d'export). */
  payload: unknown;
  /** Ferme l'aperçu. */
  onClose: () => void;
}

/** Sous-ensemble de l'API `page-flip` utilisée pour la navigation. */
interface FlipBookApi {
  flipNext: (corner?: 0 | 1) => void;
  flipPrev: (corner?: 0 | 1) => void;
  turnToPage: (page: number) => void;
  getCurrentPageIndex: () => number;
  getPageCount: () => number;
}
interface FlipBookHandle {
  pageFlip: () => FlipBookApi;
}

/** Page de l'aperçu : y attache le canvas rendu par pdf.js (sans re-rendu React). */
const PdfPage: React.FC<{ canvas: HTMLCanvasElement }> = ({ canvas }) => {
  const hostRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    host.appendChild(canvas);
    return () => {
      if (host.contains(canvas)) host.removeChild(canvas);
    };
  }, [canvas]);
  return (
    <div
      ref={hostRef}
      className="flex h-full w-full items-center justify-center overflow-hidden bg-white [&>canvas]:h-full [&>canvas]:w-full [&>canvas]:object-contain"
    />
  );
};

export const PreviewFlipbook: React.FC<PreviewFlipbookProps> = ({ payload, onClose }) => {
  const bookRef = useRef<FlipBookHandle | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  // Un **canvas fixe par page** (jamais recréé) : react-pageflip garde des enfants stables.
  const canvasesRef = useRef<HTMLCanvasElement[]>([]);
  const docRef = useRef<pdfjsLib.PDFDocumentProxy | null>(null);
  const inFlightRef = useRef<Set<number>>(new Set());
  const renderedRef = useRef<Set<number>>(new Set());

  const [canvases, setCanvases] = useState<HTMLCanvasElement[]>([]);
  const [ready, setReady] = useState<boolean[]>([]);
  const [ratio, setRatio] = useState(0.72);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");
  const [error, setError] = useState("");
  const [current, setCurrent] = useState(0);
  const [size, setSize] = useState({ width: 360, height: 500, twoPage: true });

  // Rasterise **à la demande** une page — idempotent (jamais deux fois la même).
  const ensureRendered = useCallback(async (index: number) => {
    const doc = docRef.current;
    const canvas = canvasesRef.current[index];
    if (!doc || !canvas) return;
    if (renderedRef.current.has(index) || inFlightRef.current.has(index)) return;
    inFlightRef.current.add(index);
    try {
      const page = await doc.getPage(index + 1);
      const viewport = page.getViewport({ scale: RENDER_SCALE });
      canvas.width = Math.max(1, Math.floor(viewport.width));
      canvas.height = Math.max(1, Math.floor(viewport.height));
      const context = canvas.getContext("2d");
      if (!context) throw new Error("Contexte Canvas 2D indisponible.");
      await page.render({ canvas, canvasContext: context, viewport }).promise;
      renderedRef.current.add(index);
      setReady((previous) => {
        if (previous[index]) return previous;
        const next = previous.slice();
        next[index] = true;
        return next;
      });
    } catch {
      // Page illisible : elle reste en attente (aucun échec silencieux global).
    } finally {
      inFlightRef.current.delete(index);
    }
  }, []);

  // 1) PDF Typst **en mémoire** (render_pdf) → document pdf.js + **canvas 1×1** par
  //    page (aucune rasterisation massive : la mémoire reste minimale).
  useEffect(() => {
    let cancelled = false;
    // Réinitialise l'état de pré-chargement à chaque nouveau payload.
    docRef.current = null;
    canvasesRef.current = [];
    inFlightRef.current = new Set();
    renderedRef.current = new Set();
    (async () => {
      try {
        setStatus("loading");
        setError("");
        setCanvases([]);
        setReady([]);
        setCurrent(0);
        const bytes = await renderPdfBytes(payload);
        if (cancelled) return;
        const doc = await pdfjsLib.getDocument({ data: bytes }).promise;
        if (cancelled) return;
        const first = await doc.getPage(1);
        const viewport = first.getViewport({ scale: RENDER_SCALE });
        const list: HTMLCanvasElement[] = [];
        for (let index = 0; index < doc.numPages; index += 1) {
          const canvas = document.createElement("canvas");
          canvas.width = 1;
          canvas.height = 1;
          list.push(canvas);
        }
        docRef.current = doc;
        canvasesRef.current = list;
        setRatio(viewport.width / viewport.height);
        setCanvases(list);
        setReady(new Array(doc.numPages).fill(false));
        setStatus("ready");
        // Démarrage instantané : seules les 4 premières pages sont rasterisées.
        for (let index = 0; index < Math.min(INITIAL_PAGES, doc.numPages); index += 1) {
          if (cancelled) return;
          await ensureRendered(index);
        }
      } catch (cause) {
        if (cancelled) return;
        setError(cause instanceof Error ? cause.message : String(cause));
        setStatus("error");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [payload, ensureRendered]);

  // 2) Lazy load : buffer de ±2 pages autour de la page courante. La rasterisation
  //    est différée (macro-tâche) pour ne pas déclencher de rendus en cascade.
  useEffect(() => {
    if (status !== "ready" || canvases.length === 0) return;
    const handles: number[] = [];
    for (let index = current - LAZY_RADIUS; index <= current + LAZY_RADIUS; index += 1) {
      if (index >= 0 && index < canvases.length) {
        handles.push(
          window.setTimeout(() => {
            void ensureRendered(index);
          }, 0),
        );
      }
    }
    return () => handles.forEach((handle) => window.clearTimeout(handle));
  }, [current, status, canvases.length, ensureRendered]);

  // 3) Dimensionne le livre : double page (grand écran) ou simple page (petit écran).
  useEffect(() => {
    const element = containerRef.current;
    if (!element || canvases.length === 0) return;
    const compute = () => {
      const availableWidth = element.clientWidth;
      const availableHeight = element.clientHeight;
      if (availableWidth <= 0 || availableHeight <= 0) return;
      const twoPage = availableWidth >= TWO_PAGE_MIN_WIDTH;
      // Marge de sécurité : garantit le mode paysage (double) ou portrait (simple).
      let width = twoPage ? (availableWidth - 32) / 2 : availableWidth - 16;
      let height = width / ratio;
      if (height > availableHeight) {
        height = availableHeight;
        width = height * ratio;
      }
      setSize({
        width: Math.max(1, Math.floor(width)),
        height: Math.max(1, Math.floor(height)),
        twoPage,
      });
    };
    compute();
    const observer = new ResizeObserver(compute);
    observer.observe(element);
    return () => observer.disconnect();
  }, [canvases.length, ratio]);

  const goTo = useCallback((step: number) => {
    const api = bookRef.current?.pageFlip?.();
    if (!api) return;
    if (step > 0) api.flipNext();
    else api.flipPrev();
  }, []);

  // Saut direct (scrubber) : pilote l'API du flipbook sans animation.
  const goToPage = useCallback((index: number) => {
    const last = canvasesRef.current.length - 1;
    const clamped = Math.max(0, Math.min(last, index));
    bookRef.current?.pageFlip?.().turnToPage(clamped);
    setCurrent(clamped);
  }, []);

  const pageCount = Math.max(1, canvases.length);

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-black/80 backdrop-blur-sm">
      <header className="flex items-center justify-between gap-4 border-b border-stone-500/40 bg-desk px-6 py-3">
        <span className="flex items-center gap-3 font-mono text-[11px] uppercase tracking-[0.3em] text-parchment/90">
          Aperçu interactif
          {status === "ready" && (
            <span className="text-copper">
              page {Math.max(1, current + 1)} / {pageCount}
            </span>
          )}
        </span>
        <button
          type="button"
          onClick={onClose}
          title="Fermer l'aperçu"
          className="flex items-center gap-2 rounded-sm border border-stone-500/40 bg-desk-light/90 px-3 py-2 text-[11px] font-medium uppercase tracking-[0.2em] text-parchment/90 transition-colors hover:bg-copper hover:text-amber-50"
        >
          <X size={15} strokeWidth={1.75} /> Fermer
        </button>
      </header>

      <div className="relative flex min-h-0 flex-1 items-center justify-center p-6">
        {status === "loading" && (
          <div className="flex flex-col items-center gap-3 text-parchment/90">
            <Loader2 size={26} className="animate-spin text-copper" />
            <span className="font-mono text-xs uppercase tracking-widest">
              Génération du PDF Typst…
            </span>
          </div>
        )}

        {status === "error" && (
          <div className="flex max-w-lg flex-col items-center gap-3 rounded-sm border border-red-500/40 bg-red-950/40 p-6 text-center text-red-100">
            <AlertTriangle size={26} className="text-red-400" />
            <span className="text-sm">{error}</span>
          </div>
        )}

        {status === "ready" && (
          <div ref={containerRef} className="relative flex h-full w-full items-center justify-center">
            {/* Ombre de reliure centrale (double page uniquement). */}
            {size.twoPage && (
              <div
                aria-hidden
                className="pointer-events-none absolute inset-y-0 left-1/2 z-20 w-16 -translate-x-1/2"
                style={{
                  background: "linear-gradient(90deg, transparent, rgba(0,0,0,0.30), transparent)",
                }}
              />
            )}
            <HTMLFlipBook
              key={`${size.width}x${size.height}-${canvases.length}`}
              className="preview-flipbook"
              style={{}}
              width={size.width}
              height={size.height}
              size="fixed"
              minWidth={size.width}
              maxWidth={size.width}
              minHeight={size.height}
              maxHeight={size.height}
              startPage={0}
              drawShadow
              flippingTime={FLIPPING_TIME_MS}
              usePortrait
              startZIndex={1}
              autoSize
              maxShadowOpacity={0.4}
              showCover={false}
              mobileScrollSupport
              clickEventForward
              useMouseEvents
              swipeDistance={30}
              showPageCorners
              disableFlipByClick={false}
              onFlip={(event: { data: number }) => setCurrent(event.data)}
              ref={bookRef}
            >
              {canvases.map((canvas, index) => (
                <div
                  key={index}
                  className="relative h-full w-full overflow-hidden bg-white shadow-[inset_0_0_40px_rgba(0,0,0,0.06)]"
                >
                  <PdfPage canvas={canvas} />
                  {!ready[index] && (
                    <div className="absolute inset-0 flex flex-col items-center justify-center gap-2 bg-white/85 text-stone-400">
                      <Loader2 size={20} className="animate-spin" />
                      <span className="font-mono text-[10px] uppercase tracking-widest">
                        Chargement…
                      </span>
                    </div>
                  )}
                </div>
              ))}
            </HTMLFlipBook>
          </div>
        )}
      </div>

      {status === "ready" && (
        <footer className="flex items-center gap-4 border-t border-stone-500/40 bg-desk px-6 py-3">
          <button
            type="button"
            onClick={() => goTo(-1)}
            className="flex shrink-0 items-center gap-2 rounded-sm border border-stone-500/40 bg-desk-light/90 px-4 py-2 text-[11px] font-medium uppercase tracking-[0.2em] text-parchment/90 transition-colors hover:bg-copper hover:text-amber-50"
          >
            <ChevronLeft size={15} strokeWidth={1.75} /> Précédent
          </button>
          <input
            type="range"
            min={1}
            max={pageCount}
            value={Math.min(current + 1, pageCount)}
            onChange={(event) => goToPage(Number(event.target.value) - 1)}
            aria-label="Navigation dans le livre"
            className="h-1.5 flex-1 cursor-pointer appearance-none rounded-full bg-stone-500/40 accent-copper"
          />
          <span className="w-24 shrink-0 text-center font-mono text-[11px] tabular-nums text-parchment/80">
            {Math.min(current + 1, pageCount)} / {pageCount}
          </span>
          <button
            type="button"
            onClick={() => goTo(1)}
            className="flex shrink-0 items-center gap-2 rounded-sm border border-stone-500/40 bg-desk-light/90 px-4 py-2 text-[11px] font-medium uppercase tracking-[0.2em] text-parchment/90 transition-colors hover:bg-copper hover:text-amber-50"
          >
            Suivant <ChevronRight size={15} strokeWidth={1.75} />
          </button>
        </footer>
      )}
    </div>
  );
};

