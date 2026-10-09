import React, { useEffect, useMemo, useRef, useState } from "react";
import HTMLFlipBook from "react-pageflip";

interface MenuFlipBookProps {
  /** Contenu de chaque page (une vue de menu par page). */
  pages: React.ReactNode[];
  /** Index de la page à afficher (piloté par le ruban de navigation gauche). */
  activeIndex: number;
}

/** Sous-ensemble de l'API `page-flip` exposée par la référence du composant. */
interface FlipBookApi {
  /** Tourne vers la page demandée **avec animation** (`flipToPage`). */
  flip: (page: number, corner?: 0 | 1) => void;
  /** Affiche la page demandée **sans animation** (repli de sécurité). */
  turnToPage: (page: number) => void;
  getCurrentPageIndex: () => number;
  getPageCount: () => number;
}

interface FlipBookHandle {
  pageFlip: () => FlipBookApi;
}

/** Rapport largeur / hauteur d'une page (format livre portrait). */
const PAGE_RATIO = 0.72;
/** Durée (ms) de l'animation de page tournée. */
const FLIPPING_TIME_MS = 600;

/**
 * Livre virtuel dont chaque **page est une vue de menu** (Réglages, Informations,
 * Organisation, Export).
 *
 * Le changement de page est **piloté par le ruban de navigation gauche** : lorsque
 * `activeIndex` change, le composant déclenche `flip()` qui provoque la
 * **rotation animée de la page** (`react-pageflip`) au lieu d'un basculement
 * instantané de vue.
 *
 * La page est **auto-dimensionnée** (mesure du conteneur via `ResizeObserver`) en
 * mode `fixed`, ce qui garantit un rendu **simple page** (portrait) stable quelle
 * que soit la taille de la fenêtre. Les interactions de formulaire restent
 * actives : `useMouseEvents` est désactivé et le flip au clic neutralisé
 * (navigation uniquement par le ruban).
 */
export const MenuFlipBook: React.FC<MenuFlipBookProps> = ({
  pages,
  activeIndex,
}) => {
  const bookRef = useRef<FlipBookHandle | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [size, setSize] = useState<{ width: number; height: number }>({
    width: 360,
    height: 500,
  });

  // Mesure le conteneur et calcule une page portrait qui l'occupe au mieux.
  useEffect(() => {
    const element = containerRef.current;
    if (!element) {
      return;
    }
    const compute = () => {
      const availableWidth = element.clientWidth;
      const availableHeight = element.clientHeight;
      if (availableWidth <= 0 || availableHeight <= 0) {
        return;
      }
      let height = availableHeight;
      let width = height * PAGE_RATIO;
      if (width > availableWidth) {
        width = availableWidth;
        height = width / PAGE_RATIO;
      }
      setSize({ width: Math.floor(width), height: Math.floor(height) });
    };
    compute();
    const observer = new ResizeObserver(compute);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  // Bascule animée vers la page du menu actif, avec **filet de sécurité** :
  // si l'animation n'a pas abouti (API indisponible, animation interrompue…),
  // la page cible est affichée sans animation — le changement de vue est garanti.
  useEffect(() => {
    const api = bookRef.current?.pageFlip?.();
    if (api && api.getCurrentPageIndex() !== activeIndex) {
      api.flip(activeIndex);
    }
    const timer = window.setTimeout(() => {
      const current = bookRef.current?.pageFlip?.();
      if (current && current.getCurrentPageIndex() !== activeIndex) {
        current.turnToPage(activeIndex);
      }
    }, FLIPPING_TIME_MS + 300);
    return () => window.clearTimeout(timer);
  }, [activeIndex]);

  // Pages enveloppées, mémorisées : leur identité ne change que si la liste des
  // pages change, ce qui évite toute ré-initialisation du flipbook (et donc
  // l'annulation de l'animation de page tournée) lors des re-rendus.
  const pageElements = useMemo(
    () =>
      pages.map((content, index) => (
        <div
          key={index}
          className="menu-flipbook__page h-full w-full overflow-y-auto bg-parchment p-6 shadow-[inset_0_0_40px_rgba(0,0,0,0.08)]"
        >
          {content}
        </div>
      )),
    [pages],
  );

  return (
    <div ref={containerRef} className="flex h-full w-full justify-center">
      <HTMLFlipBook
        key={`${size.width}x${size.height}`}
        className="menu-flipbook"
        style={{}}
        width={size.width}
        height={size.height}
        size="fixed"
        minWidth={size.width}
        maxWidth={size.width}
        minHeight={size.height}
        maxHeight={size.height}
        startPage={activeIndex}
        drawShadow
        flippingTime={FLIPPING_TIME_MS}
        usePortrait
        startZIndex={1}
        autoSize
        maxShadowOpacity={0.4}
        showCover={false}
        mobileScrollSupport
        clickEventForward
        useMouseEvents={false}
        swipeDistance={30}
        showPageCorners={false}
        disableFlipByClick
        ref={bookRef}
      >
        {pageElements}
      </HTMLFlipBook>
    </div>
  );
};
