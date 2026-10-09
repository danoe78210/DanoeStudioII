import React from "react";

interface ProgressBarProps {
  /** Avancement global de 0 à 100. */
  progress: number;
  /** Libellé de l'opération en cours. */
  label: string;
  /** Indique qu'un traitement lourd est actif. */
  running?: boolean;
}

export const ProgressBar: React.FC<ProgressBarProps> = ({
  progress,
  label,
  running = false,
}) => {
  const clamped = Math.min(100, Math.max(0, Math.round(progress)));
  const scale = [0, 25, 50, 75, 100];

  return (
    <div className="w-full max-w-6xl">
      {/* Châssis rétro-industriel */}
      <div className="relative rounded-md border border-brass/40 bg-linear-to-b from-[#232a38] to-[#161b26] px-5 py-3 shadow-[inset_0_1px_0_rgba(255,255,255,0.08),inset_0_-4px_12px_rgba(0,0,0,0.85),0_6px_14px_rgba(0,0,0,0.5)]">
        {/* Vis de fixation aux quatre coins */}
        <span className="absolute left-2 top-2 h-1.5 w-1.5 rounded-full bg-brass/70 shadow-[inset_0_1px_1px_rgba(0,0,0,0.6)]" />
        <span className="absolute right-2 top-2 h-1.5 w-1.5 rounded-full bg-brass/70 shadow-[inset_0_1px_1px_rgba(0,0,0,0.6)]" />
        <span className="absolute bottom-2 left-2 h-1.5 w-1.5 rounded-full bg-brass/70 shadow-[inset_0_1px_1px_rgba(0,0,0,0.6)]" />
        <span className="absolute bottom-2 right-2 h-1.5 w-1.5 rounded-full bg-brass/70 shadow-[inset_0_1px_1px_rgba(0,0,0,0.6)]" />

        <div className="mb-2 flex items-end justify-between">
          <span className="font-mono text-[11px] uppercase tracking-[0.3em] text-amber-100/70">
            {label}
          </span>
          {/* Compteur mécanique à trois chiffres */}
          <span className="rounded-sm border border-brass/40 bg-[#0b0e14] px-2 py-0.5 font-mono text-sm tabular-nums tracking-widest text-gold-light shadow-[inset_0_2px_4px_rgba(0,0,0,0.9)]">
            {String(clamped).padStart(3, "0")}&#8202;%
          </span>
        </div>

        {/* Cadran lumineux segmenté */}
        <div className="relative h-3 w-full overflow-hidden rounded-sm border border-black/60 bg-[#0b0e14] shadow-[inset_0_2px_6px_rgba(0,0,0,0.9)]">
          <div
            className={`h-full bg-linear-to-r from-brass via-gold to-gold-light transition-all duration-200 ease-out ${
              running ? "animate-pulse" : ""
            }`}
            style={{ width: `${clamped}%` }}
          />
          {/* Graduations verticales du cadran */}
          <div
            className="pointer-events-none absolute inset-0 opacity-40"
            style={{
              backgroundImage:
                "repeating-linear-gradient(90deg, transparent 0 6px, rgba(0,0,0,0.9) 6px 7px)",
            }}
          />
        </div>

        {/* Échelle graduée */}
        <div className="mt-1 flex justify-between font-mono text-[9px] uppercase tracking-widest text-amber-100/35">
          {scale.map((tick) => (
            <span key={tick}>{tick}</span>
          ))}
        </div>
      </div>
    </div>
  );
};

