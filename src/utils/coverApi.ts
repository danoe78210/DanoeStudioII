/**
 * API typée du **module Couverture** (commandes Tauri `cover::*`).
 *
 * Les noms d'arguments sont transmis en `camelCase` : Tauri v2 les convertit
 * automatiquement vers les paramètres `snake_case` des commandes Rust.
 */

import type { CoverPaperType } from "../types";
import { invokeCommand } from "./tauri";

/** Type de papier KDP (épaisseur de tranche) — défini dans `types.ts`. */
export type { CoverPaperType };

/** Bloc de tranche calculé (côté frontend). */
export interface SpineInfo {
  pagesUsed: number;
  widthMm: number;
  coefficientMmPerPage: number;
  textEligible: boolean;
}

/** Réserve code-barres (coin inférieur droit du plat 4). */
export interface BarcodeBox {
  xMm: number;
  yMm: number;
  widthMm: number;
  heightMm: number;
}

/** Géométrie complète du gabarit de couverture. */
export interface CoverGeometry {
  trimSize: string;
  paperType: string;
  bleedMm: number;
  trimWidthMm: number;
  trimHeightMm: number;
  spine: SpineInfo;
  totalWidthMm: number;
  totalHeightMm: number;
  barcode: BarcodeBox;
}

/** Statut de conformité DPI d'une image. */
export type InspectionStatus = "valid" | "warning" | "invalid";

/** Résultat d'inspection d'une image de couverture. */
export interface ImageInspection {
  path: string;
  widthPx: number;
  heightPx: number;
  hasAlpha: boolean;
  colorType: string;
  dpiHorizontal: number;
  dpiVertical: number;
  dpi: number;
  status: InspectionStatus;
}

/** Rapport d'inspection des deux plats. */
export interface ImageInspectionReport {
  trimSize: string;
  requiredDpi: number;
  front: ImageInspection;
  back: ImageInspection;
}

/** Couleur de tranche suggérée. */
export interface SpineColorSuggestion {
  colorFront: string;
  colorBack: string;
  gradientRecommended: boolean;
}

/** Paramètres de rendu d'une couverture complète. */
export interface CoverRenderParams {
  frontPath: string;
  backPath: string;
  pageCount: number;
  paperType: string;
  trimSize: string;
  spineText?: string | null;
  spineColorFront?: string | null;
  spineColorBack?: string | null;
}

/** Rapport d'export d'une couverture. */
export interface ExportReport {
  outputPath: string;
  bytes: number;
  dpi: number;
  pageCountUsed: number;
  totalWidthMm: number;
  totalHeightMm: number;
  spineWidthMm: number;
  spineTextEligible: boolean;
  flattenedImages: string[];
}

/** Ouvre le sélecteur natif d'image (`null` si annulé). */
export const pickCoverImage = (): Promise<string | null> =>
  invokeCommand<string | null>("pick_cover_image");

/** Ouvre le sélecteur natif d'enregistrement PDF (`null` si annulé). */
export const pickCoverOutputPath = (defaultName: string): Promise<string | null> =>
  invokeCommand<string | null>("pick_cover_output_path", { defaultName });

/** Calcule la géométrie complète de la couverture. */
export const calculateCoverGeometry = (
  pageCount: number,
  paperType: string,
  trimSize: string,
): Promise<CoverGeometry> =>
  invokeCommand<CoverGeometry>("calculate_cover_geometry", {
    pageCount,
    paperType,
    trimSize,
  });

/** Inspecte les deux plats (dimensions, alpha, DPI). */
export const inspectCoverImages = (
  frontPath: string,
  backPath: string,
  trimSize: string,
): Promise<ImageInspectionReport> =>
  invokeCommand<ImageInspectionReport>("inspect_cover_images", {
    frontPath,
    backPath,
    trimSize,
  });

/** Suggère une couleur de tranche (unie ou dégradé). */
export const extractSpineColor = (
  frontPath: string,
  backPath: string,
): Promise<SpineColorSuggestion> =>
  invokeCommand<SpineColorSuggestion>("extract_spine_color", {
    frontPath,
    backPath,
  });

/** Exporte le PDF complet de couverture (300 DPI). */
export const exportKdpCoverPdf = (
  params: CoverRenderParams,
  outputPdfPath: string,
): Promise<ExportReport> =>
  invokeCommand<ExportReport>("export_kdp_cover_pdf", { params, outputPdfPath });
