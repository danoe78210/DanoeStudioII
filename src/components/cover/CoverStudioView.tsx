import React, { useEffect, useMemo, useRef, useState } from "react";
import { AlertTriangle } from "lucide-react";
import { useStudio } from "../StudioContext";
import { isInvokeAvailable } from "../../utils/tauri";
import {
  calculateCoverGeometry,
  exportKdpCoverPdf,
  extractSpineColor,
  inspectCoverImages,
  pickCoverImage,
  pickCoverOutputPath,
  type CoverGeometry,
  type CoverPaperType,
  type ImageInspectionReport,
  type SpineColorSuggestion,
} from "../../utils/coverApi";
import { CoverSettingsPanel } from "./CoverSettingsPanel";
import { CoverCanvas2D } from "./CoverCanvas2D";
import { CoverPreExportModal, type PreExportCheck } from "./CoverPreExportModal";

/**
 * Atelier de composition de la couverture : état persistant (contexte projet,
 * autosauvegardé dans `.danoe`), détection de dérive de pagination (§10) et
 * dialogue de conformité avant export (§9).
 */
export const CoverStudioView: React.FC = () => {
  const { manuscriptConfig, bookInfo, cover, onCoverChange, structureSignature } =
    useStudio();
  const trimSize = manuscriptConfig.trimSize;

  const [geometry, setGeometry] = useState<CoverGeometry | null>(null);
  const [inspection, setInspection] = useState<ImageInspectionReport | null>(null);
  const [suggestion, setSuggestion] = useState<SpineColorSuggestion | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const textInitialized = useRef(false);

  const { frontPath, backPath, pageCount, paperType, spineText, gradient } = cover;

  // Texte de tranche par défaut : « Titre — Auteur » (une seule fois).
  useEffect(() => {
    if (textInitialized.current) return;
    textInitialized.current = true;
    if (cover.spineText.trim() === "") {
      const value = [bookInfo.title, bookInfo.author].filter(Boolean).join(" — ");
      if (value) onCoverChange({ spineText: value });
    }
  }, [cover.spineText, bookInfo.title, bookInfo.author, onCoverChange]);

  // Géométrie recalculée à chaque changement de paramètre.
  useEffect(() => {
    if (!isInvokeAvailable()) {
      setGeometry(null);
      return;
    }
    let active = true;
    calculateCoverGeometry(pageCount, paperType, trimSize)
      .then((result) => {
        if (active) setGeometry(result);
      })
      .catch((error) => {
        if (active) setMessage(String(error));
      });
    return () => {
      active = false;
    };
  }, [pageCount, paperType, trimSize]);

  // Inspection DPI + couleur de tranche dès que les deux plats sont choisis.
  useEffect(() => {
    if (!frontPath || !backPath || !isInvokeAvailable()) {
      setInspection(null);
      setSuggestion(null);
      return;
    }
    let active = true;
    setBusy(true);
    Promise.all([
      inspectCoverImages(frontPath, backPath, trimSize),
      extractSpineColor(frontPath, backPath),
    ])
      .then(([report, color]) => {
        if (!active) return;
        setInspection(report);
        setSuggestion(color);
        onCoverChange({
          spineColorFront: color.colorFront,
          spineColorBack: color.colorBack,
          gradient: color.gradientRecommended,
        });
      })
      .catch((error) => {
        if (active) setMessage(String(error));
      })
      .finally(() => {
        if (active) setBusy(false);
      });
    return () => {
      active = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [frontPath, backPath, trimSize]);

  // Détection de dérive (§10) : pagination ou contenu modifiés depuis la validation.
  const drift =
    cover.pageCount !== cover.pageCountSnapshot ||
    structureSignature !== cover.structureSignature;

  const handlePick = async (target: "front" | "back") => {
    try {
      setMessage(null);
      const path = await pickCoverImage();
      if (!path) return;
      onCoverChange(target === "front" ? { frontPath: path } : { backPath: path });
    } catch (error) {
      setMessage(String(error));
    }
  };

  const handleRecalcSpine = () => {
    onCoverChange({ pageCountSnapshot: pageCount, structureSignature });
  };

  // Contrôles automatiques du dialogue de pré-export (§9).
  const checks: PreExportCheck[] = useMemo(() => {
    const frontDpi = inspection?.front.dpi ?? 0;
    const backDpi = inspection?.back.dpi ?? 0;
    return [
      {
        label: "Résolution ≥ 300 DPI",
        ok: Boolean(inspection) && frontDpi >= 300 && backDpi >= 300,
        detail: inspection
          ? `${Math.round(frontDpi)} / ${Math.round(backDpi)} DPI`
          : "images non inspectées",
      },
      {
        label: "Fond perdu 3,2 mm",
        ok: Boolean(geometry) && Math.abs((geometry?.bleedMm ?? 0) - 3.2) < 0.01,
        detail: geometry ? `${geometry.bleedMm} mm` : "—",
      },
      {
        label: "Épaisseur de tranche",
        ok: Boolean(geometry),
        detail: geometry
          ? `${geometry.spine.widthMm.toFixed(2)} mm · ${geometry.spine.pagesUsed} pages`
          : "—",
      },
      {
        label: "Réserve code-barres",
        ok: Boolean(geometry),
        detail: "50,8 × 30,5 mm",
      },
      {
        label: "Texte de tranche",
        ok:
          Boolean(geometry) &&
          ((geometry?.spine.textEligible ?? false) || spineText.trim() === ""),
        detail: geometry?.spine.textEligible ? "éligible" : "tranche < 80 pages",
      },
    ];
  }, [inspection, geometry, spineText]);

  const handleExportRequest = () => {
    if (!frontPath || !backPath) {
      setMessage("Sélectionnez les deux plats (Recto & Verso) avant l'export.");
      return;
    }
    setModalOpen(true);
  };

  const runExport = async () => {
    setModalOpen(false);
    if (!frontPath || !backPath) return;
    try {
      setMessage(null);
      const destination = await pickCoverOutputPath(`couverture-${trimSize}.pdf`);
      if (!destination) return;
      setBusy(true);
      const report = await exportKdpCoverPdf(
        {
          frontPath,
          backPath,
          pageCount,
          paperType,
          trimSize,
          spineText: geometry?.spine.textEligible ? spineText : null,
          spineColorFront: cover.spineColorFront ?? suggestion?.colorFront ?? null,
          // Dégradé désactivé → couleur unie (mêmes teintes des deux côtés).
          spineColorBack: gradient
            ? cover.spineColorBack ?? suggestion?.colorBack ?? null
            : cover.spineColorFront ?? suggestion?.colorFront ?? null,
        },
        destination,
      );
      setMessage(
        `Couverture exportée (${Math.round(report.bytes / 1024)} Ko) → ${report.outputPath}`,
      );
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  };

  const canExport = Boolean(frontPath && backPath && geometry) && !busy;

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      {drift && (
        <div className="flex shrink-0 items-center justify-between gap-3 rounded-sm border border-amber-600/40 bg-amber-100/50 px-3 py-2">
          <span className="flex items-center gap-2 text-[11px] italic text-amber-900">
            <AlertTriangle size={14} />
            Dérive détectée : la pagination ou le contenu a changé depuis la validation de
            la tranche.
          </span>
          <button
            type="button"
            onClick={handleRecalcSpine}
            className="shrink-0 rounded-sm border border-brass/50 bg-linear-to-b from-copper to-[#8a5426] px-3 py-1.5 font-mono text-[10px] uppercase tracking-widest text-amber-50 shadow-[inset_0_1px_0_rgba(255,255,255,0.3),0_2px_4px_rgba(0,0,0,0.5)] transition-all hover:brightness-110 active:translate-y-px"
          >
            [ Recalculer la tranche ]
          </button>
        </div>
      )}

      <div className="flex min-h-0 flex-1 gap-4">
        <div className="w-1/3 min-w-[16rem] shrink-0 border-r border-stone-400/25 pr-2">
          <CoverSettingsPanel
            trimSize={trimSize}
            pageCount={pageCount}
            onPageCountChange={(value) => onCoverChange({ pageCount: value })}
            paperType={paperType}
            onPaperTypeChange={(value: CoverPaperType) => onCoverChange({ paperType: value })}
            frontPath={frontPath}
            backPath={backPath}
            inspection={inspection}
            onPick={handlePick}
            spineText={spineText}
            onSpineTextChange={(value) => onCoverChange({ spineText: value })}
            geometry={geometry}
            suggestion={suggestion}
            useGradient={gradient}
            onUseGradientChange={(value) => onCoverChange({ gradient: value })}
          />
        </div>
        <div className="min-w-0 flex-1">
          <CoverCanvas2D
            geometry={geometry}
            spineText={spineText}
            suggestion={suggestion}
            useGradient={gradient}
            busy={busy}
            message={message}
            canExport={canExport}
            onExport={handleExportRequest}
          />
        </div>
      </div>

      <CoverPreExportModal
        open={modalOpen}
        checks={checks}
        busy={busy}
        onCancel={() => setModalOpen(false)}
        onConfirm={runExport}
      />
    </div>
  );
};
