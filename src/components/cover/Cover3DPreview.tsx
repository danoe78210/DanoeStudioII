import React, { useRef, useState } from "react";

/** Propriétés de l'aperçu 3D du livre fermé. */
export interface Cover3DPreviewProps {
  trimWidthMm: number;
  trimHeightMm: number;
  spineWidthMm: number;
  spineText: string;
  spineColor: string | null;
  spineColorAlt: string | null;
  useGradient: boolean;
  spineTextEligible: boolean;
}

/** Borne une valeur dans un intervalle. */
const clamp = (value: number, min: number, max: number): number =>
  Math.min(max, Math.max(min, value));

/** Hauteur cible du livre à l'écran (px) — sert d'échelle mm → px. */
const TARGET_HEIGHT_PX = 240;

/**
 * Aperçu 3D interactif du livre fermé (CSS 3D pur) : Plat 1, tranche et
 * tranche des pages, avec contrôle orbital à la souris.
 */
export const Cover3DPreview: React.FC<Cover3DPreviewProps> = ({
  trimWidthMm,
  trimHeightMm,
  spineWidthMm,
  spineText,
  spineColor,
  spineColorAlt,
  useGradient,
  spineTextEligible,
}) => {
  const [rotH, setRotH] = useState(-28);
  const [rotV, setRotV] = useState(-6);
  const drag = useRef<{ x: number; y: number } | null>(null);

  const scale = TARGET_HEIGHT_PX / Math.max(trimHeightMm, 1);
  const w = trimWidthMm * scale;
  const h = trimHeightMm * scale;
  const depth = Math.max(spineWidthMm * scale, 8);

  const spineBg =
    useGradient && spineColor && spineColorAlt
      ? `linear-gradient(90deg, ${spineColor}, ${spineColorAlt})`
      : spineColor ?? "#1c140e";

  const onPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    drag.current = { x: event.clientX, y: event.clientY };
    event.currentTarget.setPointerCapture(event.pointerId);
  };

  const onPointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!drag.current) return;
    const dx = event.clientX - drag.current.x;
    const dy = event.clientY - drag.current.y;
    drag.current = { x: event.clientX, y: event.clientY };
    setRotH((value) => clamp(value + dx * 0.4, -60, 60));
    setRotV((value) => clamp(value - dy * 0.4, -15, 15));
  };

  const onPointerUp = (event: React.PointerEvent<HTMLDivElement>) => {
    drag.current = null;
    event.currentTarget.releasePointerCapture(event.pointerId);
  };

  /** Feuille de style commune d'une face centrée (méthode cuboïde). */
  const face = (fw: number, fh: number, transform: string): React.CSSProperties => ({
    position: "absolute",
    left: "50%",
    top: "50%",
    width: fw,
    height: fh,
    marginLeft: -fw / 2,
    marginTop: -fh / 2,
    transform,
    backfaceVisibility: "hidden",
  });

  return (
    <div
      className="flex h-full w-full cursor-grab touch-none items-center justify-center active:cursor-grabbing"
      style={{ perspective: "1100px" }}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
    >
      <div
        style={{
          position: "relative",
          width: w,
          height: h,
          transformStyle: "preserve-3d",
          transform: `rotateX(${rotV}deg) rotateY(${rotH}deg)`,
          transition: drag.current ? "none" : "transform 120ms ease-out",
        }}
      >
        {/* Plat 1 (couverture) */}
        <div
          style={{
            ...face(w, h, `translateZ(${depth / 2}px)`),
            background: "linear-gradient(135deg, #6b4423 0%, #4a2c15 55%, #2e1a0c 100%)",
            boxShadow: "inset 0 0 40px rgba(0,0,0,0.45)",
            borderRadius: "2px 4px 4px 2px",
          }}
        >
          {/* Éclairage d'ambiance */}
          <div
            style={{
              position: "absolute",
              inset: 0,
              background:
                "radial-gradient(circle at 30% 20%, rgba(255,220,150,0.28) 0%, transparent 55%)",
            }}
          />
          <div
            style={{
              position: "absolute",
              inset: 12,
              border: "1px solid rgba(206,161,90,0.45)",
              borderRadius: "2px",
            }}
          />
        </div>

        {/* Tranche (dos) */}
        <div
          style={{
            ...face(depth, h, `rotateY(-90deg) translateZ(${w / 2}px)`),
            background: spineBg,
            boxShadow: "inset 0 0 24px rgba(0,0,0,0.55)",
          }}
        >
          {spineTextEligible && spineText.trim() !== "" && (
            <div className="flex h-full w-full items-center justify-center overflow-hidden">
              <span
                style={{
                  transform: "rotate(-90deg)",
                  whiteSpace: "nowrap",
                  color: "#ffffff",
                  fontFamily: "Georgia, 'Times New Roman', serif",
                  fontSize: 12,
                  letterSpacing: "0.12em",
                  textShadow: "0 1px 1px rgba(0,0,0,0.5)",
                }}
              >
                {spineText}
              </span>
            </div>
          )}
        </div>

        {/* Tranche des pages (papier ivoire, texture de feuillets) */}
        <div
          style={{
            ...face(depth, h, `rotateY(90deg) translateZ(${w / 2}px)`),
            background:
              "repeating-linear-gradient(0deg, #efe6d2 0px, #efe6d2 1px, #ddd2b6 2px, #efe6d2 3px)",
            boxShadow: "inset 0 0 20px rgba(0,0,0,0.3)",
          }}
        />
      </div>

      {/* Ombre portée douce au sol */}
      <div
        aria-hidden
        style={{
          position: "absolute",
          bottom: "6%",
          width: w * 1.1,
          height: h * 0.12,
          background: "radial-gradient(ellipse, rgba(0,0,0,0.45) 0%, transparent 70%)",
          filter: "blur(8px)",
        }}
      />
    </div>
  );
};
