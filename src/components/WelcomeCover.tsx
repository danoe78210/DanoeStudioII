import React, { useEffect, useState } from "react";
import coverImg from "../assets/ds_cover.png";

interface WelcomeCoverProps {
  /** Ouvre le sommaire des reglages (tour de page vers le niveau 1). */
  onOpenSettings: () => void;
  /** Ferme proprement l'application : cinematique 3D du livre puis sortie native. */
  onQuit: () => void;
}

/** Micro-particules de poussière dorée : position dans le faisceau + cadence. */
const DUST: { left: string; top: string; delay: string; duration: string }[] = [
  { left: "24%", top: "30%", delay: "0s", duration: "11s" },
  { left: "31%", top: "38%", delay: "1.4s", duration: "13s" },
  { left: "18%", top: "52%", delay: "2.6s", duration: "12s" },
  { left: "40%", top: "45%", delay: "0.8s", duration: "14s" },
  { left: "55%", top: "62%", delay: "3.4s", duration: "10s" },
  { left: "12%", top: "72%", delay: "4.2s", duration: "15s" },
  { left: "68%", top: "78%", delay: "1.9s", duration: "12s" },
  { left: "47%", top: "88%", delay: "5.1s", duration: "13s" },
  { left: "28%", top: "66%", delay: "6.3s", duration: "11s" },
  { left: "60%", top: "40%", delay: "2.2s", duration: "16s" },
];

/**
 * Page de garde d'accueil « L'Atelier » : illustration encadree d'un
 * passe-partout parchemin, allumage progressif (lampe d'atelier) et pied de
 * page a une action — entree dans les reglages (la sortie est sur la tranche).
 * Aucun debordement : le cadre s'adapte a la hauteur (`flex-1 min-h-0`).
 */
export const WelcomeCover: React.FC<WelcomeCoverProps> = ({ onOpenSettings }) => {
  // Allumage : l'illustration demarre assombrie puis gagne la pleine lumiere.
  const [lit, setLit] = useState(false);

  useEffect(() => {
    const raf = requestAnimationFrame(() => setLit(true));
    return () => cancelAnimationFrame(raf);
  }, []);

  return (
    <div className="flex h-full max-h-full flex-col gap-3 overflow-hidden">
      {/* Passe-partout parchemin : filet fin laiton + ombre de reliure a gauche. */}
      <div className="relative flex min-h-0 flex-1 items-center justify-center rounded-sm border border-brass/40 bg-parchment-dark/50 p-3 shadow-[inset_25px_0_25px_-20px_rgba(0,0,0,0.25),0_12px_30px_rgba(0,0,0,0.3)]">
        {/* Coins renforces (equerres laiton) */}
        <span className="pointer-events-none absolute top-1.5 left-1.5 z-10 h-5 w-5 border-t-2 border-l-2 border-brass/60" />
        <span className="pointer-events-none absolute top-1.5 right-1.5 z-10 h-5 w-5 border-t-2 border-r-2 border-brass/60" />
        <span className="pointer-events-none absolute bottom-1.5 left-1.5 z-10 h-5 w-5 border-b-2 border-l-2 border-brass/60" />
        <span className="pointer-events-none absolute right-1.5 bottom-1.5 z-10 h-5 w-5 border-r-2 border-b-2 border-brass/60" />

        {/* Fenetre de l'illustration — s'adapte a l'espace, jamais de debordement */}
        <div className="relative h-full min-h-0 w-full overflow-hidden rounded-sm border border-brass/25">
          <img
            src={coverImg}
            alt="L'Atelier typographique"
            draggable={false}
            className={`h-full w-full rounded-sm object-cover select-none transition-all duration-700 ease-out ${
              lit ? "brightness-100" : "brightness-75"
            }`}
          />

          {/* Lueur radiale de l'abat-jour : centre 26 % / 22 %, pulsation lente */}
          <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_26%_22%,rgba(245,215,130,0.35)_0%,transparent_50%)] animate-pulse" />

          {/* Cône volumétrique de la lampe : polygone dégradé + flou matériel */}
          <div className="pointer-events-none absolute inset-0 overflow-hidden">
            <div className="absolute inset-0 animate-lamp-cone bg-linear-to-b from-amber-200/35 via-amber-400/15 to-transparent backdrop-blur-[1px] [clip-path:polygon(26%_22%,5%_95%,85%_95%)]" />

            {/* Poussière dorée en suspension dans le faisceau */}
            {DUST.map((dust, index) => (
              <span
                key={index}
                className="absolute h-1 w-1 animate-dust rounded-full bg-amber-200/60 blur-[0.5px]"
                style={{
                  left: dust.left,
                  top: dust.top,
                  animationDelay: dust.delay,
                  animationDuration: dust.duration,
                }}
              />
            ))}
          </div>

          {/* Glint d'orfèvrerie : balayage spéculaire traversant l'illustration */}
          <div className="pointer-events-none absolute inset-0 overflow-hidden">
            <div className="absolute top-0 -left-1/3 h-full w-1/3 animate-glint skew-x-[-25deg] bg-linear-to-r from-transparent via-white/10 to-transparent" />
          </div>

          {/* Continuite de la reliure : assombrissement du bord gauche */}
          <div className="pointer-events-none absolute inset-y-0 left-0 w-16 bg-linear-to-r from-black/25 to-transparent" />
        </div>

      </div>

      {/* Pied de page : action principale unique, centree — la sortie reste sur la tranche gauche */}
      <div className="flex shrink-0 items-center justify-center">
        <button
          type="button"
          onClick={onOpenSettings}
          className="rounded-sm border border-brass/50 bg-linear-to-b from-copper to-[#8a5426] px-4 py-2 font-mono text-[11px] tracking-[0.2em] text-amber-50 uppercase shadow-[inset_0_1px_0_rgba(255,255,255,0.3),0_2px_6px_rgba(0,0,0,0.5)] transition-all hover:brightness-110 active:translate-y-px active:shadow-[inset_0_2px_4px_rgba(0,0,0,0.6)]"
        >
          [ Entrer dans l&rsquo;atelier&nbsp;&rsaquo; ]
        </button>
      </div>
    </div>
  );
};