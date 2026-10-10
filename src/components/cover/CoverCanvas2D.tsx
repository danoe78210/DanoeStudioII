import React, { useState } from "react";
import type { CoverGeometry, SpineColorSuggestion } from "../../utils/coverApi";
import { Cover3DPreview } from "./Cover3DPreview";

/** Propriétés de la table de montage 2D (page de droite). */
export interface CoverCanvas2DProps {
  geometry: CoverGeometry | null;
  spineText: string;
  suggestion: SpineColorSuggestion | null;
  useGradient: boolean;
  busy: boolean;
  message: string | null;
  canExport: boolean;
  onExport: () => void;
}

type GuideId = "bleed" | "fold" | "safety" | "barcode";

const TOGGLES: { id: GuideId; label: string; activeClass: string }[] = [
  { id: "bleed", label: "Fond perdu (3,2 mm)", activeClass: "text-red-600" },
  { id: "fold", label: "Lignes de pliure", activeClass: "text-sky-600" },
  { id: "safety", label: "Zone de sécurité", activeClass: "text-emerald-600" },
  { id: "barcode", label: "Réserve code-barres", activeClass: "text-stone-800" },
];

/** Marge de sécurité interne des repères KDP (mm). */
const SAFETY_MM = 6.4;

/**
 * Rendu vectoriel de la planche complète étalée à plat (`plat 4 | tranche | plat 1`),
 * avec repères KDP commutables et export PDF 300 DPI.
 */
export const CoverCanvas2D: React.FC<CoverCanvas2DProps> = ({
  geometry,
  spineText,
  suggestion,
  useGradient,
  busy,
  message,
  canExport,
  onExport,
}) => {
  const [guides, setGuides] = useState<Record<GuideId, boolean>>({
    bleed: true,
    fold: true,
    safety: true,
    barcode: true,
  });
  const [view, setView] = useState<"2d" | "3d">("2d");

  const toggleGuide = (id: GuideId) =>
    setGuides((previous) => ({ ...previous, [id]: !previous[id] }));

  return (
    <div className="flex h-full min-h-0 flex-col">
      {/* Barre d'outils — repères KDP */}
      <div className="mb-2 flex flex-wrap items-center gap-2 border-b border-stone-400/30 pb-2">
        <div className="flex gap-1">
          {(["2d", "3d"] as const).map((mode) => (
            <button
              key={mode}
              type="button"
              onClick={() => setView(mode)}
              className={`rounded-sm border px-2 py-1 font-mono text-[10px] uppercase tracking-widest transition-colors ${
                view === mode
                  ? "border-brass/60 bg-copper text-amber-50"
                  : "border-stone-400/30 bg-transparent text-stone-500 hover:bg-stone-500/10"
              }`}
            >
              [ {mode === "2d" ? "Planche 2D" : "Modèle 3D"} ]
            </button>
          ))}
        </div>
        <span className="ml-2 font-mono text-[10px] uppercase tracking-widest text-stone-500">
          Repères
        </span>
        {TOGGLES.map((item) => {
          const active = guides[item.id];
          return (
            <button
              key={item.id}
              type="button"
              onClick={() => toggleGuide(item.id)}
              className={`rounded-sm border px-2 py-1 font-mono text-[10px] uppercase tracking-widest transition-colors ${
                active
                  ? `border-stone-400/60 bg-stone-500/10 ${item.activeClass}`
                  : "border-stone-400/30 bg-transparent text-stone-400"
              }`}
            >
              {item.label}
            </button>
          );
        })}
      </div>

      {/* Planche SVG */}
      <div className="flex min-h-0 flex-1 items-center justify-center rounded-sm border border-brass/30 bg-parchment-dark/40 p-2 shadow-[inset_0_0_30px_rgba(0,0,0,0.2)]">
        {!geometry ? (
          <p className="font-mono text-[11px] italic text-stone-400">
            Sélectionnez la pagination pour calculer la planche…
          </p>
        ) : view === "2d" ? (
          <CoverSvg
            geometry={geometry}
            spineText={spineText}
            suggestion={suggestion}
            useGradient={useGradient}
            guides={guides}
          />
        ) : (
          <Cover3DPreview
            trimWidthMm={geometry.trimWidthMm}
            trimHeightMm={geometry.trimHeightMm}
            spineWidthMm={geometry.spine.widthMm}
            spineText={spineText}
            spineColor={suggestion?.colorFront ?? null}
            spineColorAlt={suggestion?.colorBack ?? null}
            useGradient={useGradient}
            spineTextEligible={geometry.spine.textEligible}
          />
        )}
      </div>

      {/* Pied de page — export */}
      <div className="mt-2 flex shrink-0 items-center justify-between gap-3">
        <span className="min-w-0 flex-1 truncate font-mono text-[10px] text-stone-500" title={message ?? undefined}>
          {message}
        </span>
        <button
          type="button"
          onClick={onExport}
          disabled={!canExport || busy}
          className="shrink-0 rounded-sm border border-brass/50 bg-linear-to-b from-copper to-[#8a5426] px-4 py-2 font-mono text-[11px] uppercase tracking-[0.2em] text-amber-50 shadow-[inset_0_1px_0_rgba(255,255,255,0.3),0_2px_6px_rgba(0,0,0,0.5)] transition-all hover:brightness-110 active:translate-y-px disabled:cursor-not-allowed disabled:opacity-50"
        >
          [ {busy ? "Génération…" : "Exporter la couverture KDP (.pdf 300 DPI)"} &rsaquo; ]
        </button>
      </div>
    </div>
  );
};

/** Planche SVG vectorielle de la couverture étalée. */
const CoverSvg: React.FC<{
  geometry: CoverGeometry;
  spineText: string;
  suggestion: SpineColorSuggestion | null;
  useGradient: boolean;
  guides: Record<GuideId, boolean>;
}> = ({ geometry, spineText, suggestion, useGradient, guides }) => {
  const b = geometry.bleedMm;
  const t = geometry.trimWidthMm;
  const w = geometry.totalWidthMm;
  const h = geometry.totalHeightMm;
  const spineW = geometry.spine.widthMm;
  const barcode = geometry.barcode;

  const plat4W = b + t;
  const spineX = b + t;
  const plat1X = b + t + spineW;
  const plat1W = t + b;
  const spineCenterX = spineX + spineW / 2;

  const spineFill =
    useGradient && suggestion ? "url(#spineGrad)" : suggestion?.colorFront ?? "#1c140e";
  const fontSize = Math.max(h * 0.018, 3);
  const guide = 1.4;
  const safetyInset = b + SAFETY_MM;

  return (
    <svg
      viewBox={`0 0 ${w} ${h}`}
      className="h-full w-full"
      preserveAspectRatio="xMidYMid meet"
      role="img"
      aria-label="Planche de couverture KDP"
    >
      <defs>
        <linearGradient id="spineGrad" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stopColor={suggestion?.colorFront ?? "#1c140e"} />
          <stop offset="100%" stopColor={suggestion?.colorBack ?? "#1c140e"} />
        </linearGradient>
      </defs>

      {/* Plats 4 et 1 */}
      <rect x={0} y={0} width={plat4W} height={h} fill="#e7e0d2" />
      <rect x={plat1X} y={0} width={plat1W} height={h} fill="#e7e0d2" />

      {/* Tranche (fond uni ou dégradé) */}
      <rect x={spineX} y={0} width={spineW} height={h} fill={spineFill} />

      {/* Contour général */}
      <rect
        x={0}
        y={0}
        width={w}
        height={h}
        fill="none"
        stroke="#57534e"
        strokeWidth={guide}
        vectorEffect="non-scaling-stroke"
      />

      {/* Fond perdu (3,2 mm) — pointillés rouges */}
      {guides.bleed && (
        <rect
          x={b}
          y={b}
          width={Math.max(w - 2 * b, 0)}
          height={Math.max(h - 2 * b, 0)}
          fill="none"
          stroke="#dc2626"
          strokeWidth={guide}
          strokeDasharray="6 4"
          vectorEffect="non-scaling-stroke"
        />
      )}

      {/* Lignes de pliure — pointillés bleus */}
      {guides.fold && (
        <>
          <line
            x1={spineX}
            y1={0}
            x2={spineX}
            y2={h}
            stroke="#0284c7"
            strokeWidth={guide}
            strokeDasharray="6 4"
            vectorEffect="non-scaling-stroke"
          />
          <line
            x1={plat1X}
            y1={0}
            x2={plat1X}
            y2={h}
            stroke="#0284c7"
            strokeWidth={guide}
            strokeDasharray="6 4"
            vectorEffect="non-scaling-stroke"
          />
        </>
      )}

      {/* Zone de sécurité — cadre vert */}
      {guides.safety && (
        <rect
          x={safetyInset}
          y={safetyInset}
          width={Math.max(w - 2 * safetyInset, 0)}
          height={Math.max(h - 2 * safetyInset, 0)}
          fill="none"
          stroke="#16a34a"
          strokeWidth={guide}
          vectorEffect="non-scaling-stroke"
        />
      )}

      {/* Réserve code-barres (coin inférieur droit du plat 4) */}
      {guides.barcode && (
        <rect
          x={barcode.xMm}
          y={barcode.yMm}
          width={barcode.widthMm}
          height={barcode.heightMm}
          fill="#ffffff"
          stroke="#57534e"
          strokeWidth={guide}
          vectorEffect="non-scaling-stroke"
        />
      )}

      {/* Libellés des plats */}
      <text x={plat4W / 2} y={h * 0.05} fill="#78716c" fontSize={fontSize} textAnchor="middle">
        Plat 4 · Verso
      </text>
      <text
        x={plat1X + plat1W / 2}
        y={h * 0.05}
        fill="#78716c"
        fontSize={fontSize}
        textAnchor="middle"
      >
        Plat 1 · Recto
      </text>

      {/* Texte de tranche (tourné à -90°) */}
      {geometry.spine.textEligible && spineText.trim() !== "" && (
        <text
          x={spineCenterX}
          y={h / 2}
          fill="#ffffff"
          fontSize={fontSize}
          textAnchor="middle"
          dominantBaseline="middle"
          transform={`rotate(-90 ${spineCenterX} ${h / 2})`}
        >
          {spineText}
        </text>
      )}
    </svg>
  );
};
