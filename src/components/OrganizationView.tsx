import React, { useEffect, useRef, useState } from "react";
import {
  DndContext,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragMoveEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  arrayMove,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import {
  AlertTriangle,
  Bookmark,
  FileText,
  FolderOpen,
  GripVertical,
  Image as ImageIcon,
  Plus,
  Trash2,
  Upload,
} from "lucide-react";
import type { SpecialPageRole, StructureItem } from "../types";
import {
  DEFAULT_SPECIAL_ROLE,
  SPECIAL_PAGE_ROLES,
  specialRoleHasSource,
  specialRoleHasTitle,
} from "../data/specialPages";

interface OrganizationViewProps {
  /** Structure du roman (tableau plat ordonné). */
  items: StructureItem[];
  /** Fichiers chapitres détectés dans « Mes sources » (peut être vide). */
  chapterFiles: string[];
  /** Fichiers images détectés dans « Mes sources » (peut être vide). */
  imageFiles: string[];
  /** Le dossier unique « Mes sources » est-il configuré (Réglages) ? */
  sourcesConfigured: boolean;
  onAddAct: () => void;
  onAddChapter: () => void;
  onAddImage: () => void;
  /** Ajoute une page spéciale (page modulaire glisser-déposer). */
  onAddSpecial: () => void;
  onRemove: (id: string) => void;
  /** Renommage (validé à la perte de focus). */
  onRename: (id: string, value: string) => void;
  /** Change le rôle éditorial d'une page spéciale. */
  onRoleChange: (id: string, role: SpecialPageRole) => void;
  /** Liaison à un fichier source. */
  onSourceChange: (id: string, fileName: string) => void;
  /** Nouvel ordre (tableau complet réordonné). */
  onReorder: (items: StructureItem[]) => void;
  /** Re-sélectionne le dossier unique « Mes sources » (avec fichiers détectés). */
  onPickSourcesDirectory: () => void;
  /** Importe un fichier externe pour un chapitre donné (id). */
  onImportChapterSource: (id: string) => void;
  /** Importe un fichier externe pour une image donnée (id). */
  onImportImageSource: (id: string) => void;
  /** Importe un fichier externe pour une page spéciale donnée (id). */
  onImportSpecialSource: (id: string) => void;
}

/** Options d'un menu source : les fichiers détectés, plus la valeur courante. */
const sourceOptions = (files: string[], current?: string): string[] => {
  const base = [...files];
  if (current && !base.includes(current)) {
    base.unshift(current);
  }
  return base;
};

interface ToolbarButtonProps {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  title?: string;
}

/** Bouton d'action rétro (bakélite/cuivre) pour la barre d'outils. */
const ToolbarButton: React.FC<ToolbarButtonProps> = ({
  label,
  onClick,
  disabled = false,
  title,
}) => (
  <button
    type="button"
    onClick={onClick}
    disabled={disabled}
    title={title}
    className="flex w-full items-center justify-center gap-1 whitespace-nowrap rounded-sm border border-brass/50 bg-linear-to-b from-copper to-[#8a5426] px-2 py-2 font-mono text-[10px] uppercase tracking-normal text-amber-50 shadow-[inset_0_1px_0_rgba(255,255,255,0.3),0_2px_4px_rgba(0,0,0,0.5)] transition-all hover:brightness-110 active:translate-y-px active:shadow-[inset_0_2px_4px_rgba(0,0,0,0.6)] disabled:cursor-not-allowed disabled:opacity-40"
  >
    <Plus size={12} strokeWidth={2} />
    {label}
  </button>
);

const inputClass =
  "flex-1 bg-transparent border-b border-copper/40 font-serif text-sm text-stone-800 focus:border-copper focus:outline-none placeholder:italic placeholder:text-stone-400";

const selectClass =
  "shrink-0 rounded-sm border border-stone-400/40 bg-transparent font-mono text-xs text-stone-700 focus:border-copper focus:outline-none";

/** Nombre maximum d'éléments affichés par page (évite tout défilement). */
const ITEMS_PER_PAGE = 8;

/** Indique si un point (x, y) est à l'intérieur d'un élément. */
const isInside = (element: HTMLElement | null, x: number, y: number): boolean => {
  if (!element) {
    return false;
  }
  const rect = element.getBoundingClientRect();
  return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
};

/** Bouton secondaire discret pour re-sélectionner un dossier source. */
const PickButton: React.FC<{ onClick: () => void }> = ({ onClick }) => (
  <button
    type="button"
    onClick={onClick}
    className="flex shrink-0 items-center gap-1 rounded-sm border border-stone-400/50 bg-stone-500/5 px-2 py-1 font-mono text-[10px] uppercase tracking-widest text-stone-600 transition-colors hover:bg-stone-500/10 hover:text-copper"
  >
    <FolderOpen size={12} />
    Parcourir…
  </button>
);

/** Bouton « Importer » : copie un fichier externe dans le dossier officiel. */
const ImportButton: React.FC<{ onClick: () => void }> = ({ onClick }) => (
  <button
    type="button"
    onClick={onClick}
    title="Importer un fichier externe dans le dossier du projet"
    className="flex shrink-0 items-center gap-1 rounded-sm border border-brass/50 bg-linear-to-b from-copper to-[#8a5426] px-2 py-1 font-mono text-[10px] uppercase tracking-widest text-amber-50 shadow-[inset_0_1px_0_rgba(255,255,255,0.3),0_2px_4px_rgba(0,0,0,0.5)] transition-all hover:brightness-110 active:translate-y-px active:shadow-[inset_0_2px_4px_rgba(0,0,0,0.6)]"
  >
    <Upload size={12} />
    Importer
  </button>
);

/** Séparateur filigrané très discret pour baliser la zone de structure. */
const StructureSeparator: React.FC<{ text: string }> = ({ text }) => (
  <p className="select-none py-1 text-center font-mono text-[9px] uppercase tracking-[0.25em] text-stone-400/60">
    {text}
  </p>
);

interface SortableRowProps {
  item: StructureItem;
  /** Indente la ligne (enfant d'un Acte). */
  indent: boolean;
  chapterFiles: string[];
  imageFiles: string[];
  onRemove: (id: string) => void;
  onRename: (id: string, value: string) => void;
  onRoleChange: (id: string, role: SpecialPageRole) => void;
  onSourceChange: (id: string, fileName: string) => void;
  /** Re-sélection du dossier source (affiché si aucun fichier détecté). */
  onPickSource?: () => void;
  /** Importe un fichier externe dans le dossier officiel (« bac à sable »). */
  onImport?: () => void;
}

/** Ligne de structure déplaçable (glisser-déposer via la poignée). */
const SortableRow: React.FC<SortableRowProps> = ({
  item,
  indent,
  chapterFiles,
  imageFiles,
  onRemove,
  onRename,
  onRoleChange,
  onSourceChange,
  onPickSource,
  onImport,
}) => {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: item.id });

  return (
    <div
      ref={setNodeRef}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      className={`flex items-stretch gap-2 rounded-sm border border-stone-400/25 bg-stone-500/5 px-2 py-2 transition-shadow ${
        indent ? "ml-8" : ""
      } ${isDragging ? "bg-parchment-dark/80 shadow-[0_6px_16px_rgba(0,0,0,0.25)]" : ""}`}
    >
      {/* Poignée de déplacement — déplace le bloc entier */}
      <button
        type="button"
        {...attributes}
        {...listeners}
        aria-label="Déplacer"
        className="flex cursor-grab items-center rounded-sm px-1 text-stone-400 transition-colors hover:bg-black/5 hover:text-copper active:cursor-grabbing"
      >
        <GripVertical size={16} />
      </button>

      {item.type === "act" && (
        <div className="flex flex-1 items-center gap-3">
          <span className="w-12 shrink-0 font-mono text-[10px] font-bold uppercase tracking-widest text-copper">
            [ACTE]
          </span>
          <input
            defaultValue={item.displayName ?? ""}
            onBlur={(event) => onRename(item.id, event.target.value)}
            placeholder="ex: ACTE I — La Découverte"
            className={`${inputClass} font-semibold`}
          />
          <button
            type="button"
            onClick={() => onRemove(item.id)}
            aria-label="Supprimer"
            className="ml-auto shrink-0 rounded-sm p-1.5 text-stone-500 transition-colors hover:bg-black/5 hover:text-red-600"
          >
            <Trash2 size={16} />
          </button>
        </div>
      )}

      {item.type === "chapter" && (
        <div className="flex flex-1 flex-col gap-1.5">
          {/* Ligne 1 — sélection / import de la source */}
          <div className="flex items-center gap-3">
            <FileText size={16} className="shrink-0 text-stone-500" />
            <select
              value={item.sourceFileName ?? ""}
              onChange={(event) => onSourceChange(item.id, event.target.value)}
              className={`${selectClass} w-44`}
            >
              {(sourceOptions(chapterFiles, item.sourceFileName).length === 0 ||
                !item.sourceFileName) && (
                <option value="">
                  — Sélectionner ou importer un fichier source —
                </option>
              )}
              {sourceOptions(chapterFiles, item.sourceFileName).map((file) => (
                <option key={file} value={file}>
                  {file}
                </option>
              ))}
            </select>
            {sourceOptions(chapterFiles, item.sourceFileName).length === 0 &&
              onPickSource && <PickButton onClick={onPickSource} />}
            {onImport && <ImportButton onClick={onImport} />}
            <button
              type="button"
              onClick={() => onRemove(item.id)}
              aria-label="Supprimer"
              className="ml-auto shrink-0 rounded-sm p-1.5 text-stone-500 transition-colors hover:bg-black/5 hover:text-red-600"
            >
              <Trash2 size={16} />
            </button>
          </div>

          {/* Ligne 2 — renommage pour le livre (indentée) */}
          <div className="flex items-center gap-2 pl-10">
            <span className="shrink-0 font-mono text-[10px] uppercase tracking-widest text-copper">
              Nom dans le livre :
            </span>
            <input
              defaultValue={item.displayName ?? ""}
              onBlur={(event) => onRename(item.id, event.target.value)}
              placeholder="Saisissez le titre du chapitre tel qu'il apparaîtra..."
              className={inputClass}
            />
          </div>
        </div>
      )}

      {item.type === "image" && (
        <div className="flex flex-1 items-center gap-3">
          <ImageIcon size={16} className="shrink-0 text-stone-500" />
          <select
            value={item.sourceFileName ?? ""}
            onChange={(event) => onSourceChange(item.id, event.target.value)}
            className="min-w-0 flex-1 rounded-sm border border-stone-400/40 bg-transparent font-mono text-xs text-stone-700 focus:border-copper focus:outline-none"
          >
            {(sourceOptions(imageFiles, item.sourceFileName).length === 0 ||
              !item.sourceFileName) && (
              <option value="">— Sélectionner une image —</option>
            )}
            {sourceOptions(imageFiles, item.sourceFileName).map((file) => (
              <option key={file} value={file}>
                {file}
              </option>
            ))}
          </select>
          {sourceOptions(imageFiles, item.sourceFileName).length === 0 &&
            onPickSource && <PickButton onClick={onPickSource} />}
          {onImport && <ImportButton onClick={onImport} />}
          <button
            type="button"
            onClick={() => onRemove(item.id)}
            aria-label="Supprimer"
            className="ml-auto shrink-0 rounded-sm p-1.5 text-stone-500 transition-colors hover:bg-black/5 hover:text-red-600"
          >
            <Trash2 size={16} />
          </button>
        </div>
      )}

      {item.type === "special" && (
        <div className="flex flex-1 flex-col gap-1.5">
          {/* Ligne 1 — rôle éditorial et source */}
          <div className="flex items-center gap-3">
            <Bookmark size={16} className="shrink-0 text-copper" />
            <select
              value={item.role ?? DEFAULT_SPECIAL_ROLE}
              onChange={(event) =>
                onRoleChange(item.id, event.target.value as SpecialPageRole)
              }
              aria-label="Rôle de la page spéciale"
              className={`${selectClass} w-36`}
            >
              {SPECIAL_PAGE_ROLES.map((option) => (
                <option key={option.id} value={option.id}>
                  {option.label}
                </option>
              ))}
            </select>
            {specialRoleHasSource(item.role ?? DEFAULT_SPECIAL_ROLE) && (
              <>
                <select
                  value={item.sourceFileName ?? ""}
                  onChange={(event) =>
                    onSourceChange(item.id, event.target.value)
                  }
                  className={`${selectClass} w-44`}
                >
                  {(sourceOptions(chapterFiles, item.sourceFileName).length ===
                    0 ||
                    !item.sourceFileName) && (
                    <option value="">
                      — Sélectionner ou importer un fichier source —
                    </option>
                  )}
                  {sourceOptions(chapterFiles, item.sourceFileName).map(
                    (file) => (
                      <option key={file} value={file}>
                        {file}
                      </option>
                    ),
                  )}
                </select>
                {sourceOptions(chapterFiles, item.sourceFileName).length === 0 &&
                  onPickSource && <PickButton onClick={onPickSource} />}
                {onImport && <ImportButton onClick={onImport} />}
              </>
            )}
            <button
              type="button"
              onClick={() => onRemove(item.id)}
              aria-label="Supprimer"
              className="ml-auto shrink-0 rounded-sm p-1.5 text-stone-500 transition-colors hover:bg-black/5 hover:text-red-600"
            >
              <Trash2 size={16} />
            </button>
          </div>

          {/* Ligne 2 — titre affiché (grisé si le rôle n'admet pas de titre) */}
          <div className="flex items-center gap-2 pl-10">
            <span
              className={`shrink-0 font-mono text-[10px] uppercase tracking-widest ${
                specialRoleHasTitle(item.role ?? DEFAULT_SPECIAL_ROLE)
                  ? "text-copper"
                  : "text-stone-400"
              }`}
            >
              Titre dans le livre :
            </span>
            <input
              defaultValue={item.displayName ?? ""}
              onBlur={(event) => onRename(item.id, event.target.value)}
              disabled={!specialRoleHasTitle(item.role ?? DEFAULT_SPECIAL_ROLE)}
              placeholder={
                specialRoleHasTitle(item.role ?? DEFAULT_SPECIAL_ROLE)
                  ? "Saisissez le titre tel qu'il apparaîtra..."
                  : "Sans objet pour ce rôle"
              }
              className={`${inputClass} disabled:cursor-not-allowed disabled:border-stone-300 disabled:text-stone-400 disabled:placeholder:text-stone-300`}
            />
          </div>
        </div>
      )}
    </div>
  );
};

/** Onglet « Organisation » — construction de la structure du roman (drag & drop). */
export const OrganizationView: React.FC<OrganizationViewProps> = ({
  items,
  chapterFiles,
  imageFiles,
  sourcesConfigured,
  onAddAct,
  onAddChapter,
  onAddImage,
  onAddSpecial,
  onRemove,
  onRename,
  onRoleChange,
  onSourceChange,
  onReorder,
  onPickSourcesDirectory,
  onImportChapterSource,
  onImportImageSource,
  onImportSpecialSource,
}) => {
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
  );

  // --- Pagination : la vue n'est qu'une tranche du tableau plat global ---
  const [page, setPage] = useState(1);
  const [activeId, setActiveId] = useState<string | null>(null);
  const prevRef = useRef<HTMLButtonElement>(null);
  const nextRef = useRef<HTMLButtonElement>(null);
  const hoverTimerRef = useRef<number | null>(null);
  const hoverTargetRef = useRef<"prev" | "next" | null>(null);

  const pageCount = Math.max(1, Math.ceil(items.length / ITEMS_PER_PAGE));
  const safePage = Math.min(page, pageCount);
  const startIndex = (safePage - 1) * ITEMS_PER_PAGE;
  const pageItems = items.slice(startIndex, startIndex + ITEMS_PER_PAGE);

  // Maintient l'élément en cours de déplacement monté après un changement de page.
  const visibleItems =
    activeId && !pageItems.some((item) => item.id === activeId)
      ? [...pageItems, ...items.filter((item) => item.id === activeId)]
      : pageItems;

  useEffect(() => {
    if (page > pageCount) {
      setPage(pageCount);
    }
  }, [page, pageCount]);

  // Indentation calculée sur le tableau global (correcte d'une page à l'autre).
  const indentById = new Map<string, boolean>();
  let seenAct = false;
  for (const item of items) {
    if (item.type === "act") {
      indentById.set(item.id, false);
      seenAct = true;
    } else {
      indentById.set(item.id, seenAct);
    }
  }

  const clearHoverTimer = () => {
    if (hoverTimerRef.current !== null) {
      window.clearTimeout(hoverTimerRef.current);
      hoverTimerRef.current = null;
    }
    hoverTargetRef.current = null;
  };

  const handleDragStart = (event: DragStartEvent) => {
    setActiveId(String(event.active.id));
  };

  const handleDragEnd = (event: DragEndEvent) => {
    clearHoverTimer();
    setActiveId(null);
    const { active, over } = event;
    if (!over || active.id === over.id) {
      return;
    }
    const oldIndex = items.findIndex((item) => item.id === active.id);
    const newIndex = items.findIndex((item) => item.id === over.id);
    if (oldIndex < 0 || newIndex < 0) {
      return;
    }
    onReorder(arrayMove(items, oldIndex, newIndex));
  };

  const handleDragCancel = () => {
    clearHoverTimer();
    setActiveId(null);
  };

  // Option A : survoler « Page précédente / suivante » pendant 800 ms change de
  // page en plein glisser-déposer (permet le déplacement multi-pages).
  const handleDragMove = (event: DragMoveEvent) => {
    const activator = event.activatorEvent as PointerEvent;
    const x = activator.clientX + event.delta.x;
    const y = activator.clientY + event.delta.y;
    const overPrev = safePage > 1 && isInside(prevRef.current, x, y);
    const overNext = safePage < pageCount && isInside(nextRef.current, x, y);
    const target: "prev" | "next" | null = overPrev
      ? "prev"
      : overNext
        ? "next"
        : null;

    if (target === hoverTargetRef.current) {
      return;
    }
    if (hoverTimerRef.current !== null) {
      window.clearTimeout(hoverTimerRef.current);
      hoverTimerRef.current = null;
    }
    hoverTargetRef.current = target;
    if (target) {
      hoverTimerRef.current = window.setTimeout(() => {
        hoverTimerRef.current = null;
        setPage((current) =>
          target === "prev"
            ? Math.max(1, current - 1)
            : Math.min(pageCount, current + 1),
        );
      }, 800);
    }
  };

  const missing: string[] = [];
  if (!sourcesConfigured) {
    missing.push("Mes sources");
  }

  return (
    <div className="flex h-full grow flex-col">
      <h2 className="mb-4 border-b border-stone-400/40 pb-3 text-center font-serif text-2xl uppercase tracking-[0.25em] text-stone-700">
        Organisation
      </h2>

      {/* Zone de contenu — s'étire (flex-1) pour plaquer la pagination en bas */}
      <div className="flex flex-1 flex-col">
        {/* Barre d'outils d'ajout — 4 colonnes, sur une seule ligne */}
        <div className="mb-3 grid w-full grid-cols-4 gap-3">
          <ToolbarButton label="Ajouter un Acte" onClick={onAddAct} />
          <ToolbarButton
            label="Ajouter un Chapitre"
            onClick={onAddChapter}
            disabled={!sourcesConfigured}
            title={
              !sourcesConfigured
                ? "Configurez d'abord Mes sources (Réglages)"
                : undefined
            }
          />
          <ToolbarButton
            label="Ajouter une Image"
            onClick={onAddImage}
            disabled={!sourcesConfigured}
            title={
              !sourcesConfigured
                ? "Configurez d'abord Mes sources (Réglages)"
                : undefined
            }
          />
          <ToolbarButton
            label="Page spéciale"
            onClick={onAddSpecial}
            disabled={!sourcesConfigured}
            title={
              !sourcesConfigured
                ? "Configurez d'abord Mes sources (Réglages)"
                : undefined
            }
          />
        </div>

        {missing.length > 0 && (
          <p className="mb-4 flex items-center gap-2 font-serif text-xs italic text-[#8a5a2b]">
            <AlertTriangle size={14} />
            Configurez {missing.join(" et ")} dans les Réglages pour lier des
            fichiers sources.
          </p>
        )}

        {/* Zone de structure (glisser-déposer) — vue paginée */}
        {items.length === 0 ? (
          <p className="mt-8 text-center font-serif text-sm italic text-stone-400">
            Structure vide. Ajoutez un Acte, un Chapitre, une Image ou une Page
            spéciale.
          </p>
        ) : (
          <DndContext
            sensors={sensors}
            collisionDetection={closestCenter}
            onDragStart={handleDragStart}
            onDragMove={handleDragMove}
            onDragEnd={handleDragEnd}
            onDragCancel={handleDragCancel}
          >
            <SortableContext
              items={visibleItems.map((item) => item.id)}
              strategy={verticalListSortingStrategy}
            >
              <div className="space-y-1.5">
                {safePage === 1 && (
                  <StructureSeparator text="--- Début de l'ouvrage (pages liminaires auto-générées) ---" />
                )}
                {visibleItems.map((item) => (
                  <SortableRow
                    key={item.id}
                    item={item}
                    indent={indentById.get(item.id) ?? false}
                    chapterFiles={chapterFiles}
                    imageFiles={imageFiles}
                    onRemove={onRemove}
                    onRename={onRename}
                    onRoleChange={onRoleChange}
                    onSourceChange={onSourceChange}
                    onPickSource={
                      item.type === "chapter" ||
                      item.type === "image" ||
                      item.type === "special"
                        ? onPickSourcesDirectory
                        : undefined
                    }
                    onImport={
                      item.type === "chapter"
                        ? () => onImportChapterSource(item.id)
                        : item.type === "image"
                          ? () => onImportImageSource(item.id)
                          : item.type === "special"
                            ? () => onImportSpecialSource(item.id)
                            : undefined
                    }
                  />
                ))}
                {safePage === pageCount && (
                  <StructureSeparator text="--- Fin de l'ouvrage (table des matières auto-générée) ---" />
                )}
              </div>
            </SortableContext>
          </DndContext>
        )}
      </div>

      {/* Navigation de pagination — ancrée tout en bas de la page (mt-auto) */}
      <div className="mt-auto flex items-center justify-center gap-6 pt-5 font-mono text-[10px] uppercase tracking-widest">
        <button
          ref={prevRef}
          type="button"
          onClick={() => setPage((current) => Math.max(1, current - 1))}
          disabled={safePage <= 1}
          className="text-stone-500 transition-colors hover:text-copper disabled:cursor-not-allowed disabled:opacity-40"
        >
          ← Page précédente
        </button>
        <span className="text-copper">
          Page {safePage} / {pageCount}
        </span>
        <button
          ref={nextRef}
          type="button"
          onClick={() => setPage((current) => Math.min(pageCount, current + 1))}
          disabled={safePage >= pageCount}
          className="text-stone-500 transition-colors hover:text-copper disabled:cursor-not-allowed disabled:opacity-40"
        >
          Page suivante →
        </button>
      </div>
    </div>
  );
};


