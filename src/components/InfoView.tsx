import React, { type ReactNode } from "react";
import type { BookInfoConfig } from "../types";

interface InfoViewProps {
  /** Métadonnées du roman. */
  config: BookInfoConfig;
  /** Enregistre la modification d'un champ (validation à la perte de focus). */
  onFieldChange: (
    section: string,
    label: string,
    field: keyof BookInfoConfig,
    value: string,
  ) => void;
}

/** Style « registre » d'un champ : soulignement cuivre uniquement. */
const inputClass =
  "w-full bg-transparent border-b border-copper/50 font-serif text-sm text-stone-800 placeholder:italic placeholder:text-stone-400 focus:border-copper focus:outline-none";

interface InfoInputProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  type?: "text" | "number";
  /** Champ pleine largeur (occupe les 2 colonnes). */
  full?: boolean;
}

/** Champ texte court : libellé en petites majuscules au-dessus, saisie soulignée. */
const InfoInput: React.FC<InfoInputProps> = ({
  label,
  value,
  onChange,
  placeholder,
  type = "text",
  full = false,
}) => (
  <div className={`flex flex-col gap-1 ${full ? "col-span-2" : ""}`}>
    <label className="font-mono text-[10px] uppercase tracking-widest text-stone-500">
      {label}
    </label>
    <input
      type={type}
      defaultValue={value}
      onBlur={(event) => onChange(event.target.value)}
      placeholder={placeholder}
      className={inputClass}
    />
  </div>
);

interface InfoTextAreaProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}

/** Zone multiligne pleine largeur (éléments annexes). */
const InfoTextArea: React.FC<InfoTextAreaProps> = ({
  label,
  value,
  onChange,
  placeholder,
}) => (
  <div className="col-span-2 flex flex-col gap-1">
    <label className="font-mono text-[10px] uppercase tracking-widest text-stone-500">
      {label}
    </label>
    <textarea
      defaultValue={value}
      onBlur={(event) => onChange(event.target.value)}
      placeholder={placeholder}
      rows={4}
      className={`${inputClass} resize-none leading-relaxed`}
    />
  </div>
);

interface SectionProps {
  title: string;
  children: ReactNode;
}

/** Section logique : titre en petites capitales + grille à 2 colonnes. */
const Section: React.FC<SectionProps> = ({ title, children }) => (
  <section className="mb-6">
    <h3 className="mb-3 font-mono text-[10px] uppercase tracking-widest text-copper">
      {title}
    </h3>
    <div className="grid grid-cols-2 gap-x-8 gap-y-4">{children}</div>
  </section>
);

/**
 * Onglet « Informations » — formulaire unique organisé en sections
 * (Identité, Auteurs, Édition, Éléments annexes). Écriture liée à l'état global,
 * validée à la perte de focus (autosauvegarde `.danoe`).
 */
export const InfoView: React.FC<InfoViewProps> = ({ config, onFieldChange }) => {
  const set =
    (section: string, label: string, field: keyof BookInfoConfig) =>
    (value: string) =>
      onFieldChange(section, label, field, value);

  return (
    <div className="flex h-full flex-col">
      <h2 className="mb-4 border-b border-stone-400/40 pb-3 text-center font-serif text-2xl uppercase tracking-[0.25em] text-stone-700">
        Informations
      </h2>

      <div className="mx-auto w-full max-w-2xl overflow-y-auto pr-1">
        <Section title="Identité de l'œuvre">
          <InfoInput
            label="Titre du livre"
            full
            value={config.title}
            placeholder="ex. Nunael"
            onChange={set("Identité de l'œuvre", "Titre du livre", "title")}
          />
          <InfoInput
            label="Sous-titre"
            full
            value={config.subtitle}
            onChange={set("Identité de l'œuvre", "Sous-titre", "subtitle")}
          />
          <InfoInput
            label="Nom de la série / Saga"
            value={config.sagaTitle}
            placeholder="ex. Les Schattenjägers"
            onChange={set("Identité de l'œuvre", "Nom de la série / Saga", "sagaTitle")}
          />
          <InfoInput
            label="Numéro de tome"
            type="number"
            value={config.volumeNumber}
            placeholder="ex. 1"
            onChange={set("Identité de l'œuvre", "Numéro de tome", "volumeNumber")}
          />
        </Section>

        <Section title="Auteurs et contributeurs">
          <InfoInput
            label="Nom de l'auteur principal"
            value={config.author}
            placeholder="ex. Danoë"
            onChange={set("Auteurs et contributeurs", "Nom de l'auteur principal", "author")}
          />
          <InfoInput
            label="Traducteur ou illustrateur"
            value={config.contributor}
            placeholder="Optionnel"
            onChange={set("Auteurs et contributeurs", "Traducteur ou illustrateur", "contributor")}
          />
        </Section>

        <Section title="Édition et mentions légales">
          <InfoInput
            label="Nom de l'éditeur"
            value={config.publisher}
            placeholder="ex. Auto-édition ou nom de la maison"
            onChange={set("Édition et mentions légales", "Nom de l'éditeur", "publisher")}
          />
          <InfoInput
            label="Numéro ISBN"
            value={config.isbn}
            placeholder="ex. 978-2-XXXXXX-XX-X"
            onChange={set("Édition et mentions légales", "Numéro ISBN", "isbn")}
          />
          <InfoInput
            label="Date de parution"
            value={config.year}
            placeholder="ex. Octobre 2026"
            onChange={set("Édition et mentions légales", "Date de parution", "year")}
          />
          <InfoInput
            label="Lieu d'impression"
            value={config.printLocation}
            placeholder="ex. France"
            onChange={set("Édition et mentions légales", "Lieu d'impression", "printLocation")}
          />
        </Section>

        <Section title="Éléments annexes">
          <InfoTextArea
            label="Autres œuvres"
            value={config.otherBooks}
            placeholder="ex. Titre 1, Titre 2, Titre 3"
            onChange={set("Éléments annexes", "Autres œuvres", "otherBooks")}
          />
        </Section>
      </div>
    </div>
  );
};