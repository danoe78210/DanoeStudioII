import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import './index.css';
import { Layout } from "./components/Layout";
import { LeftPage } from "./components/LeftPage";
import { MenuPage } from "./components/MenuPage";
import { PreviewFlipbook } from "./components/PreviewFlipbook";
import { StudioProvider, type StudioContextValue } from "./components/StudioContext";
import { Toast, type ToastMessage } from "./components/Toast";
import type { ActiveMenu, BookInfoConfig, CoverStudioState, ErrorLogConfig, ImageColorMode, LogDiagnosticLevel, LogEntry, LogLevel, ManuscriptLayoutConfig, SourcesConfig, SpecialPageRole, StructureItem } from "./types";
import { findTrimPreset } from "./data/trimSizes";
import { INHERIT_FONT, numberFr } from "./data/fonts";
import { DEFAULT_SPECIAL_ROLE, specialRoleLabel } from "./data/specialPages";
import { openPathExternal } from "./utils/shell";
import { invokeCommand, isInvokeAvailable } from "./utils/tauri";
import {
  importFileIntoFolder,
  isFsAvailable,
  listDirectoryFiles,
  pickDirectory,
  pickDirectoryWithFiles,
  type FsDirectoryHandle,
} from "./utils/directory";
import {
  PROJECT_FILE_NAME,
  buildProjectFile,
  loadProject,
  parseProjectJson,
  saveProject,
  treeToFlat,
} from "./utils/projectFile";
import { isImageFile, splitSourceFiles } from "./utils/sources";
import { finalizeExit, onCloseRequested } from "./utils/shutdown";

const currentTime = () =>
  new Date().toLocaleTimeString("fr-FR", { hour12: false });

let logSequence = 0;
const createLog = (level: LogLevel, message: string): LogEntry => ({
  id: `${Date.now()}-${logSequence++}`,
  time: currentTime(),
  level,
  message,
});

let structureSequence = 0;
const nextStructureId = (type: string): string =>
  `${type}-${Date.now()}-${structureSequence++}`;

// Configuration de mise en page du manuscrit EXPORTÉ (Word / PDF conforme KDP).
// Portée stricte : ces valeurs ne concernent QUE le document final généré
// (métadonnées & règles de mise en page pour les moteurs d'export).
// Elles ne modifient en AUCUN cas les styles CSS de l'interface du studio.
const DEFAULT_MANUSCRIPT: ManuscriptLayoutConfig = {
  trimSize: "6x9",
  bodyFont: "Garamond",
  bodySize: 11,
  lineSpacing: 1.15,
  textAlignment: "justify",
  dropCap: true,
  chapterTitleFont: "body",
  chapterTitleSize: 16,
  subtitleFont: "body",
  subtitleSize: 14,
};

/** Construit le message de journal décrivant un changement de mise en page. */
const describeLayoutChange = (
  patch: Partial<ManuscriptLayoutConfig>,
  next: ManuscriptLayoutConfig,
): string => {
  const resolve = (font: string): string =>
    font === INHERIT_FONT ? next.bodyFont : font;
  const parts: string[] = [];

  if ("bodyFont" in patch || "bodySize" in patch) {
    parts.push(`${next.bodyFont} ${numberFr(next.bodySize)}pt`);
  }
  if ("lineSpacing" in patch) {
    parts.push(`interligne ${numberFr(next.lineSpacing)}`);
  }
  if ("textAlignment" in patch) {
    parts.push(next.textAlignment === "justify" ? "justifié" : "aligné à gauche");
  }
  if ("dropCap" in patch) {
    parts.push(next.dropCap ? "lettrine active" : "lettrine inactive");
  }
  if ("chapterTitleFont" in patch || "chapterTitleSize" in patch) {
    parts.push(
      `titre de chapitre ${resolve(next.chapterTitleFont)} ${numberFr(next.chapterTitleSize)}pt`,
    );
  }
  if ("subtitleFont" in patch || "subtitleSize" in patch) {
    parts.push(
      `sous-titres ${resolve(next.subtitleFont)} ${numberFr(next.subtitleSize)}pt`,
    );
  }
  return `Mise en page manuscrit mise à jour : ${parts.join(", ")}`;
};

// Configuration des sources brutes du roman (dossier unique « Mes sources »).
const DEFAULT_SOURCES: SourcesConfig = {
  directory: null,
  colorMode: "grayscale",
};

const IMAGE_COLOR_LABELS: Record<ImageColorMode, string> = {
  color: "Couleur",
  grayscale: "Noir & Blanc",
};

// Configuration du journal des erreurs.
const DEFAULT_ERROR_LOG: ErrorLogConfig = {
  level: "standard",
  directory: null,
};

// État par défaut du module Couverture (Cover Studio).
const DEFAULT_COVER: CoverStudioState = {
  frontPath: null,
  backPath: null,
  pageCount: 120,
  paperType: "cream",
  spineText: "",
  spineColorFront: null,
  spineColorBack: null,
  gradient: false,
  pageCountSnapshot: 120,
  structureSignature: "",
};

// Ordre des pages du livre virtuel de droite (identique à l'ordre du ruban gauche).
const MENU_ORDER: ActiveMenu[] = [
  'reglages',
  'infos',
  'organisation',
  'couverture',
  'correcteur',
  'export',
];

// Pages du livre virtuel — identité **stable** (constante module) : leur contenu
// reste vivant via `StudioContext`, sans ré-initialiser le flipbook.
const BOOK_PAGES = MENU_ORDER.map((menu) => <MenuPage key={menu} menu={menu} />);

const LOG_LEVEL_LABELS: Record<LogDiagnosticLevel, string> = {
  standard: "Standard",
  diagnostic: "Mécanique",
};

// Métadonnées du roman (onglet « Informations »).
const DEFAULT_BOOK_INFO: BookInfoConfig = {
  title: "Les Schattenjägers",
  subtitle: "",
  sagaTitle: "",
  volumeNumber: "",
  author: "Danoë",
  contributor: "",
  publisher: "",
  isbn: "",
  year: "2026",
  printLocation: "",
  otherBooks: "",
  copyright: "",
  website: "",
};

const App: React.FC = () => {
  const [activeMenu, setActiveMenu] = useState<ActiveMenu>('reglages');
  const [logs, setLogs] = useState<LogEntry[]>(() => [
    createLog('info', 'Studio initialisé. Prêt à écrire.'),
    createLog('success', 'Projet « Les Schattenjägers » chargé.'),
  ]);
  const [progress, setProgress] = useState(0);
  const [progressLabel, setProgressLabel] = useState('Aucune opération en cours');
  const [running, setRunning] = useState(false);
  const [toast, setToast] = useState<ToastMessage | null>(null);
  const [manuscriptConfig, setManuscriptConfig] =
    useState<ManuscriptLayoutConfig>(DEFAULT_MANUSCRIPT);
  const [sourceConfig, setSourceConfig] =
    useState<SourcesConfig>(DEFAULT_SOURCES);
  const [errorLogConfig, setErrorLogConfig] =
    useState<ErrorLogConfig>(DEFAULT_ERROR_LOG);
  const [bookInfo, setBookInfo] = useState<BookInfoConfig>(DEFAULT_BOOK_INFO);
  const [structure, setStructure] = useState<StructureItem[]>([]);
  const [chapterFiles, setChapterFiles] = useState<string[]>([]);
  const [imageFiles, setImageFiles] = useState<string[]>([]);
  // État persistant du module Couverture (synchronisé dans le fichier `.danoe`).
  const [cover, setCover] = useState<CoverStudioState>(DEFAULT_COVER);
  // Payload figé de l'aperçu interactif (null = fermé) : capturé à l'ouverture
  // pour une identité stable (évite toute re-génération au re-rendu).
  const [previewPayload, setPreviewPayload] = useState<unknown | null>(null);
  // Cinématique de fermeture du livre (déclenchée par le bouton ou la croix native).
  const [isClosing, setIsClosing] = useState(false);
  const [projectFilePath, setProjectFilePath] = useState<string | null>(null);
  const [saveState, setSaveState] = useState<'idle' | 'saving' | 'saved'>('idle');
  const [hydrated, setHydrated] = useState(false);
  const timerRef = useRef<number | null>(null);
  const sourceHandleRef = useRef<FsDirectoryHandle | null>(null);

  const addLog = useCallback((level: LogLevel, message: string) => {
    setLogs((previous) => [...previous, createLog(level, message)]);
  }, []);

  // Mise à jour partielle de l'état de la couverture (autosauvegardé).
  const handleCoverChange = useCallback((patch: Partial<CoverStudioState>) => {
    setCover((previous) => ({ ...previous, ...patch }));
  }, []);

  // Signature de la structure courante (détection de dérive de pagination — §10).
  const structureSignature = useMemo(
    () => structure.map((item) => item.type).join("|"),
    [structure],
  );

  // Fermeture animée : le volet 3D du livre se rabat, puis la sortie est finalisée.
  const handleQuit = useCallback(() => {
    setIsClosing(true);
  }, []);

  // Croix native de la fenêtre → déclenche la même cinématique de fermeture.
  useEffect(() => {
    onCloseRequested(() => setIsClosing(true));
  }, []);

  // Extinction du glow (~1 000 ms) puis fermeture native définitive.
  useEffect(() => {
    if (!isClosing) {
      return;
    }
    const handle = window.setTimeout(() => {
      void finalizeExit();
    }, 1150);
    return () => window.clearTimeout(handle);
  }, [isClosing]);

  // Nettoyage de l'intervalle en cas de démontage.
  useEffect(() => {
    return () => {
      if (timerRef.current !== null) {
        window.clearInterval(timerRef.current);
      }
    };
  }, []);

  // Détection des fichiers du dossier unique (watcher — Tauri uniquement). Hors
  // Tauri, les fichiers sont fournis par le sélecteur de dossier lui-même.
  // Le moteur de tri répartit ensuite les fichiers par extension.
  useEffect(() => {
    if (!isFsAvailable()) {
      return;
    }
    let active = true;
    listDirectoryFiles(sourceConfig.directory).then((files) => {
      console.log("[sources] énumération du dossier :", {
        directory: sourceConfig.directory,
        files,
      });
      if (active) {
        const { chapters, images } = splitSourceFiles(files);
        setChapterFiles(chapters);
        setImageFiles(images);
      }
    });
    return () => {
      active = false;
    };
  }, [sourceConfig.directory]);

  // Restauration du projet au démarrage (fichier Tauri ou repli localStorage).
  useEffect(() => {
    let active = true;
    loadProject(null).then((text) => {
      if (!active) {
        return;
      }
      const data = text ? parseProjectJson(text) : null;
      if (data) {
        if (data.metadata) {
          const metadata = data.metadata;
          setBookInfo({
            title: metadata.bookTitle ?? "",
            subtitle: metadata.subtitle ?? "",
            sagaTitle: metadata.sagaTitle ?? "",
            volumeNumber: metadata.volumeNumber ?? "",
            author: metadata.authorName ?? "",
            contributor: metadata.contributor ?? "",
            publisher: metadata.publisher ?? "",
            isbn: metadata.isbn ?? "",
            year: metadata.year ?? "",
            printLocation: metadata.printLocation ?? "",
            otherBooks: metadata.otherBooks ?? "",
            copyright: metadata.copyrightText ?? "",
            website: metadata.website ?? "",
          });
        }
        if (data.layoutConfig) {
          setManuscriptConfig({ ...DEFAULT_MANUSCRIPT, ...data.layoutConfig });
        }
        if (data.directories) {
          const directories = data.directories;
          // Migration : le dossier unique remplace les anciens dossiers séparés.
          const sourcesDirectory =
            directories.sources ??
            directories.chapters ??
            directories.images ??
            null;
          setSourceConfig((previous) => ({
            ...previous,
            directory: sourcesDirectory,
          }));
          setErrorLogConfig((previous) => ({
            ...previous,
            directory: directories.logs ?? null,
          }));
        }
        if (data.options) {
          const options = data.options;
          setSourceConfig((previous) => ({
            ...previous,
            colorMode: options.imageColorMode ?? previous.colorMode,
          }));
          setErrorLogConfig((previous) => ({
            ...previous,
            level: options.logLevel ?? previous.level,
          }));
        }
        if (data.organization) {
          setStructure(treeToFlat(data.organization));
        }
        if (data.fileCache) {
          setChapterFiles(data.fileCache.chapters);
          setImageFiles(data.fileCache.images);
        }
        if (data.cover) {
          setCover((previous) => ({ ...previous, ...data.cover }));
        }
      }
      setHydrated(true);
    });
    return () => {
      active = false;
    };
  }, []);

  // Compile l'état courant en fichier de projet (réutilisé par la sauvegarde et l'export).
  const buildCurrentProject = useCallback(
    () =>
      buildProjectFile({
        bookInfo,
        manuscriptConfig,
        sourceConfig,
        errorLogConfig,
        structure,
        chapterFiles,
        imageFiles,
        cover,
      }),
    [
      bookInfo,
      manuscriptConfig,
      sourceConfig,
      errorLogConfig,
      structure,
      chapterFiles,
      imageFiles,
      cover,
    ],
  );

  // Ouvre l'aperçu interactif : fige le payload courant (généré en mémoire par Typst).
  const handlePreview = useCallback(() => {
    setPreviewPayload(buildCurrentProject());
  }, [buildCurrentProject]);

  // Sauvegarde automatique (debounce ~800 ms) — aucun bouton « Enregistrer ».
  useEffect(() => {
    if (!hydrated) {
      return;
    }
    const timer = window.setTimeout(() => {
      setSaveState('saving');
      const json = JSON.stringify(buildCurrentProject(), null, 2);
      saveProject(json, projectFilePath).then((method) => {
        if (method === 'error') {
          setSaveState('idle');
          return;
        }
        setSaveState('saved');
        window.setTimeout(() => setSaveState('idle'), 1500);
      });
    }, 800);
    return () => window.clearTimeout(timer);
  }, [
    hydrated,
    buildCurrentProject,
    projectFilePath,
  ]);

  const runExport = useCallback(
    async (format: string) => {
      if (running) {
        return;
      }
      setRunning(true);
      setProgress(0);
      setProgressLabel(`Export ${format} en cours…`);
      addLog('info', `Démarrage de l'export — ${format}`);

      // Payload complet du projet (mêmes données que le fichier `.danoe`).
      const payload = buildCurrentProject();

      // **Garde-fou UI** : le moteur Rust applique une politique **stricte** — toute
      // illustration sans fichier lié (`sourceFileName` vide), introuvable ou non
      // décodable **bloque l'export** avec un message explicite remonté au frontend.
      // On prévient donc l'utilisateur AVANT l'export pour qu'il corrige dans
      // l'onglet « Organisation ».
      const unlinkedImages = structure.filter(
        (item) => item.type === "image" && !item.sourceFileName,
      );
      if (unlinkedImages.length > 0) {
        addLog(
          'warning',
          `${unlinkedImages.length} illustration(s) sans fichier lié : l'export sera BLOQUÉ tant qu'elles ne seront pas reliées à un fichier (onglet « Organisation »).`,
        );
      }

      // Débogage : vérifier que l'arborescence (chapitres ajoutés/liaisons
      // sources) est bien transmise au moteur Rust.
      console.log("[export] payload transmis au backend :", {
        format,
        directories: payload.directories,
        organization: payload.organization,
      });

      // Application empaquetée : appel réel du moteur Rust via la commande Tauri.
      if (isInvokeAvailable()) {
        try {
          // Routage par format : EPUB → scaffold, PDF → moteur Typst, sinon Word (.docx).
          const lowered = format.toLowerCase();
          const isEpub = lowered.includes('epub');
          const isPdf = lowered.includes('pdf');
          const message = isEpub
            ? await invokeCommand<string>('generate_epub', { payload })
            : isPdf
              ? await invokeCommand<string>('export_pdf', { payload })
              : await invokeCommand<string>('generate_docx', { format, payload });
          setProgress(100);
          setProgressLabel(`Export ${format} terminé`);
          addLog('success', message || `Export ${format} généré avec succès.`);
          const cancelled = message.startsWith('Export annulé');
          setToast({
            kind: cancelled ? 'info' : 'success',
            message: message || `Export ${format} généré avec succès.`,
          });
        } catch (error) {
          addLog('error', `Échec de l'export ${format} : ${String(error)}`);
          setToast({
            kind: 'error',
            message: `Échec de l'export : ${String(error)}`,
          });
        } finally {
          window.setTimeout(() => {
            setProgress(0);
            setProgressLabel('Aucune opération en cours');
            setRunning(false);
          }, 1600);
        }
        return;
      }

      // Repli navigateur (SPA sans Tauri) : export simulé.
      addLog(
        'warning',
        "Moteur Rust indisponible (application non empaquetée) : export simulé.",
      );
      let value = 0;
      timerRef.current = window.setInterval(() => {
        value += 10;
        setProgress(value);

        if (value >= 100 && timerRef.current !== null) {
          window.clearInterval(timerRef.current);
          timerRef.current = null;
          setProgressLabel(`Export ${format} terminé`);
          addLog('success', `Export ${format} généré avec succès.`);
          setToast({
            kind: 'success',
            message: `Export ${format} généré avec succès (simulation navigateur).`,
          });
          setRunning(false);

          window.setTimeout(() => {
            setProgress(0);
            setProgressLabel('Aucune opération en cours');
          }, 1600);
        }
      }, 180);
    },
    [addLog, buildCurrentProject, running, structure],
  );

  const handleTrimChange = useCallback(
    (presetId: string) => {
      const preset = findTrimPreset(presetId);
      if (!preset) {
        return;
      }
      setManuscriptConfig((previous) => ({ ...previous, trimSize: presetId }));
      addLog('success', `Format de coupe configuré sur ${preset.label} (${preset.cm})`);
    },
    [addLog],
  );

  const handleLayoutChange = useCallback(
    (patch: Partial<ManuscriptLayoutConfig>) => {
      const next = { ...manuscriptConfig, ...patch };
      setManuscriptConfig(next);
      addLog('info', describeLayoutChange(patch, next));
    },
    [manuscriptConfig, addLog],
  );

  const handleSourcesDirectoryChange = useCallback(
    (path: string, files: string[], handle: FsDirectoryHandle | null) => {
      sourceHandleRef.current = handle;
      setSourceConfig((previous) => ({ ...previous, directory: path }));
      const { chapters, images } = splitSourceFiles(files);
      setChapterFiles(chapters);
      setImageFiles(images);
      addLog('success', `Dossier des sources mis à jour : ${path}`);
    },
    [addLog],
  );

  const handleImageColorModeChange = useCallback(
    (mode: ImageColorMode) => {
      setSourceConfig((previous) => ({ ...previous, colorMode: mode }));
      addLog('info', `Images PDF configurées sur : ${IMAGE_COLOR_LABELS[mode]}`);
    },
    [addLog],
  );

  const handleLogLevelChange = useCallback(
    (level: LogDiagnosticLevel) => {
      setErrorLogConfig((previous) => ({ ...previous, level }));
      addLog(
        'info',
        `Niveau de diagnostic défini sur : ${LOG_LEVEL_LABELS[level]}`,
      );
    },
    [addLog],
  );

  const handleLogDirectoryChange = useCallback(
    (path: string) => {
      setErrorLogConfig((previous) => ({ ...previous, directory: path }));
      addLog('success', `Emplacement du journal mis à jour : ${path}`);
    },
    [addLog],
  );

  const handleOpenLog = useCallback(async () => {
    const path = errorLogConfig.directory
      ? `${errorLogConfig.directory}/danoe-studio.log`
      : "danoe-studio.log";
    const opened = await openPathExternal(path);
    if (opened) {
      addLog('success', `Ouverture du fichier de bord : ${path}`);
    } else {
      addLog(
        'warning',
        "Ouverture du fichier de bord indisponible (nécessite l'application empaquetée).",
      );
    }
  }, [errorLogConfig, addLog]);

  const handleInfoFieldChange = useCallback(
    (section: string, label: string, field: keyof BookInfoConfig, value: string) => {
      setBookInfo((previous) => ({ ...previous, [field]: value }));
      addLog('info', `[Informations - ${section}] ${label} mis à jour.`);
    },
    [addLog],
  );

  const handleAddAct = useCallback(() => {
    const count = structure.filter((item) => item.type === "act").length;
    setStructure((previous) => [
      ...previous,
      { id: nextStructureId("act"), type: "act", displayName: `Acte ${count + 1}` },
    ]);
    addLog('info', `Structure : acte ${count + 1} ajouté.`);
  }, [structure, addLog]);

  const handleAddChapter = useCallback(() => {
    const count = structure.filter((item) => item.type === "chapter").length;
    setStructure((previous) => [
      ...previous,
      {
        id: nextStructureId("chapter"),
        type: "chapter",
        displayName: `Chapitre ${count + 1}`,
        sourceFileName: "",
      },
    ]);
    addLog('info', `Structure : chapitre ${count + 1} ajouté.`);
  }, [structure, addLog]);

  const handleAddImage = useCallback(() => {
    setStructure((previous) => [
      ...previous,
      { id: nextStructureId("image"), type: "image", sourceFileName: "" },
    ]);
    addLog('info', "Structure : illustration ajoutée.");
  }, [addLog]);

  const handleAddSpecialPage = useCallback(() => {
    setStructure((previous) => [
      ...previous,
      {
        id: nextStructureId("special"),
        type: "special",
        role: DEFAULT_SPECIAL_ROLE,
        sourceFileName: "",
      },
    ]);
    addLog('info', "Structure : page spéciale ajoutée.");
  }, [addLog]);

  const handleSpecialRoleChange = useCallback(
    (id: string, role: SpecialPageRole) => {
      setStructure((previous) =>
        previous.map((item) => (item.id === id ? { ...item, role } : item)),
      );
      addLog(
        'info',
        `Structure : rôle de la page spéciale défini sur « ${specialRoleLabel(role)} ».`,
      );
    },
    [addLog],
  );

  const handleRemoveStructureItem = useCallback(
    (id: string) => {
      setStructure((previous) => previous.filter((item) => item.id !== id));
      addLog('info', "Structure : élément supprimé.");
    },
    [addLog],
  );

  const handleRenameStructureItem = useCallback(
    (id: string, value: string) => {
      setStructure((previous) =>
        previous.map((item) =>
          item.id === id ? { ...item, displayName: value } : item,
        ),
      );
      addLog('info', `Structure : libellé mis à jour (« ${value} »).`);
    },
    [addLog],
  );

  const handleStructureSourceChange = useCallback(
    (id: string, fileName: string) => {
      setStructure((previous) =>
        previous.map((item) =>
          item.id === id ? { ...item, sourceFileName: fileName } : item,
        ),
      );
      addLog(
        'info',
        fileName
          ? `Structure : source liée « ${fileName} ».`
          : "Structure : source retirée.",
      );
    },
    [addLog],
  );

  const handleStructureReorder = useCallback(
    (next: StructureItem[]) => {
      setStructure(next);
      addLog('info', "Structure : ordre réorganisé.");
    },
    [addLog],
  );

  const handlePickProjectFile = useCallback(async () => {
    const directory = await pickDirectory();
    if (directory) {
      setProjectFilePath(`${directory}/${PROJECT_FILE_NAME}`);
      addLog('info', `Fichier de projet défini dans : ${directory}`);
    }
  }, [addLog]);

  const pickSourcesDirectory = useCallback(async () => {
    const { path, files, handle } = await pickDirectoryWithFiles();
    console.log("[sources] dossier « Mes sources » sélectionné :", { path, files });
    if (path) {
      sourceHandleRef.current = handle;
      setSourceConfig((previous) => ({ ...previous, directory: path }));
      const { chapters, images } = splitSourceFiles(files);
      setChapterFiles(chapters);
      setImageFiles(images);
      addLog('success', `Dossier des sources mis à jour : ${path}`);
    }
  }, [addLog]);

  const importIntoFolder = useCallback(
    async (itemId: string) => {
      const result = await importFileIntoFolder(
        sourceHandleRef.current,
        sourceConfig.directory,
      );
      if (!result) {
        return;
      }
      if (result.copied) {
        // Le fichier rejoint le dossier unique ; le moteur de tri décide, selon
        // son extension, du tableau dans lequel il apparaît (chapitres/images).
        const isImage = isImageFile(result.fileName);
        addLog(
          'success',
          `${isImage ? "Illustration" : "Chapitre"} importé dans le projet : ${result.fileName}`,
        );
        const setFiles = isImage ? setImageFiles : setChapterFiles;
        setFiles((previous) =>
          previous.includes(result.fileName)
            ? previous
            : [...previous, result.fileName].sort((a, b) => a.localeCompare(b)),
        );
      } else {
        addLog(
          'warning',
          `Import sans copie (application empaquetée requise) : ${result.fileName}`,
        );
      }
      setStructure((previous) =>
        previous.map((item) =>
          item.id === itemId
            ? { ...item, sourceFileName: result.fileName }
            : item,
        ),
      );
    },
    [sourceConfig.directory, addLog],
  );

  // État applicatif exposé aux pages du livre virtuel (via `StudioContext`).
  const studioValue: StudioContextValue = {
    manuscriptConfig,
    sourceConfig,
    errorLogConfig,
    onTrimChange: handleTrimChange,
    onLayoutChange: handleLayoutChange,
    onSourcesDirectoryChange: handleSourcesDirectoryChange,
    onImageColorModeChange: handleImageColorModeChange,
    onLogLevelChange: handleLogLevelChange,
    onLogDirectoryChange: handleLogDirectoryChange,
    onOpenLog: handleOpenLog,
    bookInfo,
    onInfoFieldChange: handleInfoFieldChange,
    structure,
    chapterFiles,
    imageFiles,
    sourcesConfigured: sourceConfig.directory !== null,
    onAddAct: handleAddAct,
    onAddChapter: handleAddChapter,
    onAddImage: handleAddImage,
    onAddSpecial: handleAddSpecialPage,
    onRemoveStructureItem: handleRemoveStructureItem,
    onRenameStructureItem: handleRenameStructureItem,
    onSpecialRoleChange: handleSpecialRoleChange,
    onStructureSourceChange: handleStructureSourceChange,
    onStructureReorder: handleStructureReorder,
    onPickSourcesDirectory: pickSourcesDirectory,
    onImportIntoFolder: (id: string) => {
      void importIntoFolder(id);
    },
    onExport: runExport,
    onPreview: handlePreview,
    cover,
    onCoverChange: handleCoverChange,
    structureSignature,
    isExporting: running,
    onQuit: handleQuit,
  };

  const activeIndex = Math.max(0, MENU_ORDER.indexOf(activeMenu));

  return (
    <>
      <StudioProvider value={studioValue}>
        <Layout
          leftContent={
            <LeftPage
              activeMenu={activeMenu}
              setActiveMenu={setActiveMenu}
              onExitApp={handleQuit}
            />
          }
          bookPages={BOOK_PAGES}
          activeIndex={activeIndex}
          progress={progress}
          progressLabel={progressLabel}
          running={running}
          logs={logs}
          saveState={saveState}
          projectFilePath={projectFilePath}
          onPickProjectFile={handlePickProjectFile}
          closing={isClosing}
        />
      </StudioProvider>
      {toast && <Toast toast={toast} onClose={() => setToast(null)} />}
      {previewPayload !== null && (
        <PreviewFlipbook
          payload={previewPayload}
          onClose={() => setPreviewPayload(null)}
        />
      )}
    </>
  );
};

export default App;

