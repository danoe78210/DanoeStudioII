/**
 * Moteur de tri des sources (dossier unique « Mes sources »).
 *
 * Le projet ne stocke qu'**un seul dossier** (workspace) contenant l'intégralité
 * des fichiers. Les chapitres et les illustrations y cohabitent : ce module
 * répartit les fichiers détectés en deux tableaux distincts, en se basant
 * **strictement** sur leur extension.
 */

/** Extensions reconnues comme chapitres (textes). */
export const CHAPTER_EXTENSIONS = [".txt", ".md", ".docx"] as const;

/** Extensions reconnues comme images (illustrations). */
export const IMAGE_EXTENSIONS = [".png", ".jpg", ".jpeg", ".webp", ".tiff"] as const;

/** Retourne l'extension en minuscules d'un nom de fichier (avec le point). */
const extensionOf = (fileName: string): string => {
  const dot = fileName.lastIndexOf(".");
  return dot > 0 ? fileName.slice(dot).toLowerCase() : "";
};

/** Indique si un fichier est un chapitre (extension texte reconnue). */
export const isChapterFile = (fileName: string): boolean =>
  (CHAPTER_EXTENSIONS as readonly string[]).includes(extensionOf(fileName));

/** Indique si un fichier est une image (extension illustration reconnue). */
export const isImageFile = (fileName: string): boolean =>
  (IMAGE_EXTENSIONS as readonly string[]).includes(extensionOf(fileName));

/** Tri alphabétique insensible à la casse (locale-aware). */
const sortNames = (a: string, b: string) => a.localeCompare(b);

/**
 * Répartit une liste brute de fichiers (contenu du dossier « Mes sources »)
 * en deux tableaux triés : chapitres et images.
 *
 * Les fichiers aux extensions inconnues sont ignorés par les deux filtres.
 */
export const splitSourceFiles = (
  files: string[],
): { chapters: string[]; images: string[] } => {
  const chapters: string[] = [];
  const images: string[] = [];
  for (const file of files) {
    if (isChapterFile(file)) {
      chapters.push(file);
    } else if (isImageFile(file)) {
      images.push(file);
    }
  }
  chapters.sort(sortNames);
  images.sort(sortNames);
  return { chapters, images };
};
