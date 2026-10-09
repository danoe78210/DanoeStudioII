import React, { type ReactNode } from "react";
import { SAMPLE_TEXT } from "../../data/fonts";

interface PillButtonProps {
  active: boolean;
  onClick: () => void;
  children: ReactNode;
  title?: string;
  /** Classes additionnelles (largeur, alignement…). */
  className?: string;
}

/** Bouton « pastille » pour les valeurs (tailles, interlignes, oui/non…). */
export const PillButton: React.FC<PillButtonProps> = ({
  active,
  onClick,
  children,
  title,
  className = "",
}) => (
  <button
    type="button"
    onClick={onClick}
    title={title}
    className={`rounded-sm border px-3 py-1.5 font-mono text-xs transition-colors ${
      active
        ? "border-copper bg-copper text-amber-50"
        : "border-stone-400/40 bg-stone-500/5 text-stone-600 hover:bg-stone-500/10"
    } ${className}`}
  >
    {children}
  </button>
);

interface RadioRowProps {
  active: boolean;
  onClick: () => void;
  children: ReactNode;
}

/** Ligne de registre cliquable avec marqueur radio cuivré. */
export const RadioRow: React.FC<RadioRowProps> = ({
  active,
  onClick,
  children,
}) => (
  <button
    type="button"
    onClick={onClick}
    className={`flex w-full items-center gap-4 rounded-sm border px-3 py-2 text-left transition-colors ${
      active
        ? "border-copper bg-copper/10"
        : "border-stone-400/30 bg-stone-500/5 hover:bg-stone-500/10"
    }`}
  >
    <span
      className={`flex h-4 w-4 shrink-0 items-center justify-center rounded-full border ${
        active ? "border-copper" : "border-stone-400"
      }`}
    >
      {active && <span className="h-2 w-2 rounded-full bg-copper" />}
    </span>
    {children}
  </button>
);

interface FontRadioRowProps {
  active: boolean;
  onClick: () => void;
  /** Libellé affiché (nom de la police ou « Identique au corps du texte »). */
  name: string;
  /** Pile CSS d'aperçu ; absente pour l'option d'héritage. */
  stack?: string;
}

/** Ligne de sélection d'une police, avec aperçu visuel contextuel. */
export const FontRadioRow: React.FC<FontRadioRowProps> = ({
  active,
  onClick,
  name,
  stack,
}) => (
  <RadioRow active={active} onClick={onClick}>
    <span className="w-44 shrink-0 font-medium text-stone-700">{name}</span>
    {stack ? (
      <span className="truncate text-stone-600" style={{ fontFamily: stack }}>
        {SAMPLE_TEXT}
      </span>
    ) : (
      <span className="truncate text-xs italic text-stone-400">
        hérite du corps de texte
      </span>
    )}
  </RadioRow>
);

interface FontCardProps {
  active: boolean;
  onClick: () => void;
  name: string;
  /** Pile CSS d'aperçu. */
  stack: string;
  /** Court échantillon affiché dans la police. */
  preview?: string;
}

/** Carte compacte de sélection d'une police (grille 2 colonnes, sans ascenseur). */
export const FontCard: React.FC<FontCardProps> = ({
  active,
  onClick,
  name,
  stack,
  preview = "Abc 123 — Danoë",
}) => (
  <button
    type="button"
    onClick={onClick}
    className={`flex flex-col gap-1 rounded-sm border px-3 py-2 text-left transition-colors ${
      active
        ? "border-copper bg-copper/10"
        : "border-stone-400/30 bg-stone-500/5 hover:bg-stone-500/10"
    }`}
  >
    <span className="flex items-center gap-2">
      <span
        className={`flex h-3.5 w-3.5 shrink-0 items-center justify-center rounded-full border ${
          active ? "border-copper" : "border-stone-400"
        }`}
      >
        {active && <span className="h-1.5 w-1.5 rounded-full bg-copper" />}
      </span>
      <span className="truncate text-xs font-medium text-stone-700">{name}</span>
    </span>
    <span
      className="truncate pl-6 text-sm text-stone-600"
      style={{ fontFamily: stack }}
    >
      {preview}
    </span>
  </button>
);
