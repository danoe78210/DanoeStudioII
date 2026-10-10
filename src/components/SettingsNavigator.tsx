import React, { useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import type {
  ErrorLogConfig,
  ImageColorMode,
  LogDiagnosticLevel,
  ManuscriptLayoutConfig,
  SourcesConfig,
} from "../types";
import { trimLabelFor } from "../data/trimSizes";
import type { FsDirectoryHandle } from "../utils/directory";
import { SettingsView } from "./SettingsView";
import { SettingsGeneralPage } from "./settings/SettingsGeneralPage";
import { FormatPage } from "./settings/FormatPage";
import { BodyTextPage } from "./settings/BodyTextPage";
import { ChapterTitlePage } from "./settings/ChapterTitlePage";
import { SubtitlePage } from "./settings/SubtitlePage";
import { SourcesPage } from "./settings/SourcesPage";
import { ErrorLogPage } from "./settings/ErrorLogPage";
import { CorrectorPage } from "./settings/CorrectorPage";
import { WelcomeCover } from "./WelcomeCover";

type SettingsLevel = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9;

interface SettingsNavigatorProps {
  /** Configuration de mise en page du manuscrit exporté (jamais l'UI). */
  manuscriptConfig: ManuscriptLayoutConfig;
  /** Configuration des sources brutes (dossier unique « Mes sources »). */
  sourceConfig: SourcesConfig;
  /** Change le format de coupe KDP. */
  onTrimChange: (presetId: string) => void;
  /** Applique un changement de mise en page (typographie, structure). */
  onLayoutChange: (patch: Partial<ManuscriptLayoutConfig>) => void;
  /** Change le dossier unique des sources (avec les fichiers détectés). */
  onSourcesDirectoryChange: (
    path: string,
    files: string[],
    handle: FsDirectoryHandle | null,
  ) => void;
  /** Change le traitement colorimétrique des images. */
  onImageColorModeChange: (mode: ImageColorMode) => void;
  /** Configuration du journal des erreurs. */
  errorLogConfig: ErrorLogConfig;
  /** Change le niveau de diagnostic. */
  onLogLevelChange: (level: LogDiagnosticLevel) => void;
  /** Change le dossier du journal. */
  onLogDirectoryChange: (path: string) => void;
  /** Demande à l'OS d'ouvrir le fichier journal. */
  onOpenLog: () => void;
  /** Ferme proprement l'application (cinématique de fermeture + purge des caches). */
  onQuit: () => void;
}

// Fil d'Ariane selon le niveau courant.
const BREADCRUMBS: Record<SettingsLevel, string[]> = {
  0: [],
  1: ["Réglages"],
  2: ["Réglages", "Paramètres du livre"],
  3: ["Réglages", "Paramètres du livre", "Format du livre"],
  4: ["Réglages", "Paramètres du livre", "Corps du texte"],
  5: ["Réglages", "Paramètres du livre", "Titre du chapitre"],
  6: ["Réglages", "Paramètres du livre", "Sous-titres"],
  7: ["Réglages", "Mes sources"],
  8: ["Réglages", "Journal des erreurs"],
  9: ["Réglages", "Correcteur linguistique"],
};

// Variantes d'animation : simule une page de parchemin qui se tourne
// autour de la reliure (bord gauche) de la page de droite.
const pageVariants = {
  enter: (direction: number) => ({
    rotateY: direction > 0 ? 75 : -75,
    opacity: 0,
  }),
  center: { rotateY: 0, opacity: 1 },
  exit: (direction: number) => ({
    rotateY: direction > 0 ? -75 : 75,
    opacity: 0,
  }),
};

export const SettingsNavigator: React.FC<SettingsNavigatorProps> = ({
  manuscriptConfig,
  sourceConfig,
  errorLogConfig,
  onTrimChange,
  onLayoutChange,
  onSourcesDirectoryChange,
  onImageColorModeChange,
  onLogLevelChange,
  onLogDirectoryChange,
  onOpenLog,
  onQuit,
}) => {
  const [level, setLevel] = useState<SettingsLevel>(0);
  // 1 = tour vers l'avant, -1 = retour.
  const [direction, setDirection] = useState(1);

  const goTo = (next: SettingsLevel, dir: number) => {
    setDirection(dir);
    setLevel(next);
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      {/* Fil d'Ariane */}
      <nav
        className={`${level === 0 ? "hidden " : ""}mb-5 flex flex-wrap items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-stone-500`}
      >
        {BREADCRUMBS[level].map((crumb, index, all) => (
          <React.Fragment key={crumb}>
            {index > 0 && <span className="text-stone-400">&rsaquo;</span>}
            <span className={index === all.length - 1 ? "text-copper" : ""}>
              {crumb}
            </span>
          </React.Fragment>
        ))}
      </nav>

      {/* Zone de tour de page */}
      <div className="relative flex-1 [perspective:2000px]">
        <AnimatePresence mode="wait" custom={direction}>
          <motion.div
            key={level}
            custom={direction}
            variants={pageVariants}
            initial="enter"
            animate="center"
            exit="exit"
            transition={{ duration: 0.45, ease: "easeInOut" }}
            style={{
              transformOrigin: "left center",
              transformStyle: "preserve-3d",
              backfaceVisibility: "hidden",
            }}
            className="h-full"
          >
            {level === 0 && (
              <WelcomeCover onOpenSettings={() => goTo(1, 1)} onQuit={onQuit} />
            )}

            {level === 1 && (
              <SettingsView
                onOpenGeneralSettings={() => goTo(2, 1)}
                onOpenSources={() => goTo(7, 1)}
                onOpenErrorLog={() => goTo(8, 1)}
                onOpenCorrector={() => goTo(9, 1)}
                trimLabel={trimLabelFor(manuscriptConfig.trimSize)}
              />
            )}

            {level === 2 && (
              <SettingsGeneralPage
                config={manuscriptConfig}
                onOpenFormat={() => goTo(3, 1)}
                onOpenBody={() => goTo(4, 1)}
                onOpenChapterTitle={() => goTo(5, 1)}
                onOpenSubtitle={() => goTo(6, 1)}
                onBack={() => goTo(1, -1)}
              />
            )}

            {level === 3 && (
              <FormatPage
                config={manuscriptConfig}
                onTrimChange={onTrimChange}
                onBack={() => goTo(2, -1)}
              />
            )}

            {level === 4 && (
              <BodyTextPage
                config={manuscriptConfig}
                onChange={onLayoutChange}
                onBack={() => goTo(2, -1)}
              />
            )}

            {level === 5 && (
              <ChapterTitlePage
                config={manuscriptConfig}
                onChange={onLayoutChange}
                onBack={() => goTo(2, -1)}
              />
            )}

            {level === 6 && (
              <SubtitlePage
                config={manuscriptConfig}
                onChange={onLayoutChange}
                onBack={() => goTo(2, -1)}
              />
            )}

            {level === 7 && (
              <SourcesPage
                config={sourceConfig}
                onColorModeChange={onImageColorModeChange}
                onDirectoryChange={onSourcesDirectoryChange}
                onBack={() => goTo(1, -1)}
              />
            )}

            {level === 8 && (
              <ErrorLogPage
                config={errorLogConfig}
                onLevelChange={onLogLevelChange}
                onDirectoryChange={onLogDirectoryChange}
                onOpenLog={onOpenLog}
                onBack={() => goTo(1, -1)}
              />
            )}

            {level === 9 && <CorrectorPage onBack={() => goTo(1, -1)} />}
          </motion.div>
        </AnimatePresence>
      </div>
    </div>
  );
};
