import React from "react";
import { BookOpen, ChevronLeft, ChevronRight, Heading1, Heading2, PenLine } from "lucide-react";
import type { ManuscriptLayoutConfig } from "../../types";
import { trimSubtitleFor } from "../../data/trimSizes";
import { fontLabel, numberFr } from "../../data/fonts";

interface SettingsGeneralPageProps {
  /** Configuration de mise en page du manuscrit (export). */
  config: ManuscriptLayoutConfig;
  /** Ouvre la sous-page de sélection du format de coupe. */
  onOpenFormat: () => void;
  onOpenBody: () => void;
  onOpenChapterTitle: () => void;
  onOpenSubtitle: () => void;
  onBack: () => void;
}

interface NavRowProps {
  icon: React.ComponentType<{ size?: number; strokeWidth?: number; className?: string }>;
  title: string;
  summary: string;
  onClick: () => void;
}

const NavRow: React.FC<NavRowProps> = ({ icon: Icon, title, summary, onClick }) => (
  <button
    type="button"
    onClick={onClick}
    className="flex w-full items-center gap-4 rounded-sm border border-stone-400/30 bg-stone-500/5 px-4 py-3 text-left transition-colors hover:bg-stone-500/10"
  >
    <Icon size={20} strokeWidth={1.5} className="text-stone-600" />
    <span className="flex-1">
      <span className="block font-medium text-stone-700">{title}</span>
      <span className="block font-mono text-[10px] uppercase tracking-widest text-stone-500">
        {summary}
      </span>
    </span>
    <ChevronRight size={18} className="text-stone-400" />
  </button>
);

/** Étape 2 — paramètres généraux du manuscrit (format + accès typographie). */
export const SettingsGeneralPage: React.FC<SettingsGeneralPageProps> = ({
  config,
  onOpenFormat,
  onOpenBody,
  onOpenChapterTitle,
  onOpenSubtitle,
  onBack,
}) => {
  return (
    <div className="h-full">
      <button
        type="button"
        onClick={onBack}
        className="mb-4 flex items-center gap-1 font-mono text-[11px] uppercase tracking-widest text-stone-500 transition hover:text-copper"
      >
        <ChevronLeft size={14} />
        Retour
      </button>

      <h2 className="mb-6 border-b border-stone-400/40 pb-4 text-center font-serif text-2xl uppercase tracking-[0.25em] text-stone-700">
        Paramètres du livre
      </h2>

      {/* Format du livre — carte de navigation vers la sous-page */}
      <h3 className="mb-2 font-mono text-[10px] uppercase tracking-widest text-stone-500">
        Format du livre
      </h3>
      <div className="mb-6">
        <NavRow
          icon={BookOpen}
          title="Format du livre"
          summary={trimSubtitleFor(config.trimSize)}
          onClick={onOpenFormat}
        />
      </div>

      {/* Sous-sections de typographie */}
      <h3 className="mb-2 font-mono text-[10px] uppercase tracking-widest text-stone-500">
        Mise en page
      </h3>
      <div className="space-y-2">
        <NavRow
          icon={PenLine}
          title="Corps du texte"
          summary={`${config.bodyFont} ${numberFr(config.bodySize)} pt · interligne ${numberFr(config.lineSpacing)}`}
          onClick={onOpenBody}
        />
        <NavRow
          icon={Heading1}
          title="Titre du chapitre"
          summary={`${fontLabel(config.chapterTitleFont)} ${numberFr(config.chapterTitleSize)} pt`}
          onClick={onOpenChapterTitle}
        />
        <NavRow
          icon={Heading2}
          title="Sous-titres"
          summary={`${fontLabel(config.subtitleFont)} ${numberFr(config.subtitleSize)} pt`}
          onClick={onOpenSubtitle}
        />
      </div>
    </div>
  );
};
