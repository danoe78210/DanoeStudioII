import React, { useEffect, useState } from "react";
import { ShieldCheck, AlertTriangle } from "lucide-react";

/** Résultat d'un contrôle automatique de conformité KDP. */
export interface PreExportCheck {
  label: string;
  ok: boolean;
  detail: string;
}

/** Propriétés du dialogue de pré-export (modal bloquant). */
export interface CoverPreExportModalProps {
  open: boolean;
  checks: PreExportCheck[];
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

/**
 * Dialogue de conformité avant export : contrôles automatiques + validations
 * manuelles de l'auteur (modal bloquant).
 */
export const CoverPreExportModal: React.FC<CoverPreExportModalProps> = ({
  open,
  checks,
  busy,
  onCancel,
  onConfirm,
}) => {
  const [cutChecked, setCutChecked] = useState(false);
  const [spineChecked, setSpineChecked] = useState(false);

  // Réinitialise les cases manuelles à chaque ouverture.
  useEffect(() => {
    if (open) {
      setCutChecked(false);
      setSpineChecked(false);
    }
  }, [open]);

  if (!open) {
    return null;
  }

  const autoValid = checks.every((check) => check.ok);
  const canConfirm = autoValid && cutChecked && spineChecked && !busy;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
      <div className="w-full max-w-lg rounded-md border border-brass/40 bg-parchment shadow-[0_20px_60px_rgba(0,0,0,0.6)]">
        <header className="flex items-center gap-2 border-b border-stone-400/40 px-5 py-3">
          <ShieldCheck size={18} className="text-copper" />
          <h2 className="font-serif text-lg uppercase tracking-[0.2em] text-stone-700">
            Contrôle avant export
          </h2>
        </header>

        <div className="max-h-[55vh] overflow-y-auto px-5 py-4">
          <h3 className="mb-2 font-mono text-[10px] uppercase tracking-widest text-copper">
            Contrôles automatiques
          </h3>
          <ul className="mb-4 space-y-1">
            {checks.map((check) => (
              <li key={check.label} className="flex items-start gap-2 text-[12px]">
                <span className={check.ok ? "text-emerald-600" : "text-red-600"}>
                  {check.ok ? "✓" : "✗"}
                </span>
                <span className="flex-1">
                  <span className="text-stone-700">{check.label}</span>
                  {check.detail && (
                    <span className="ml-1 font-mono text-[10px] text-stone-500">
                      ({check.detail})
                    </span>
                  )}
                </span>
              </li>
            ))}
          </ul>

          <h3 className="mb-2 font-mono text-[10px] uppercase tracking-widest text-copper">
            Validation de l&rsquo;auteur
          </h3>
          <label className="mb-2 flex items-start gap-2 text-[12px] text-stone-700">
            <input
              type="checkbox"
              checked={cutChecked}
              onChange={(event) => setCutChecked(event.target.checked)}
            />
            Je certifie que les textes importants sont à l&rsquo;écart des zones de
            massicotage (fond perdu).
          </label>
          <label className="flex items-start gap-2 text-[12px] text-stone-700">
            <input
              type="checkbox"
              checked={spineChecked}
              onChange={(event) => setSpineChecked(event.target.checked)}
            />
            J&rsquo;ai vérifié la lisibilité et le centrage du texte de tranche.
          </label>

          {!autoValid && (
            <p className="mt-3 flex items-start gap-2 rounded-sm border border-amber-600/40 bg-amber-100/40 px-2 py-2 text-[11px] italic text-amber-900">
              <AlertTriangle size={14} className="mt-0.5 shrink-0" />
              Certains contrôles automatiques échouent. Corrigez les paramètres avant
              d&rsquo;exporter.
            </p>
          )}
        </div>

        <footer className="flex items-center justify-end gap-3 border-t border-stone-400/40 px-5 py-3">
          <button
            type="button"
            onClick={onCancel}
            className="rounded-sm border border-stone-400/50 bg-stone-500/5 px-4 py-2 font-mono text-[11px] uppercase tracking-widest text-stone-600 transition-colors hover:bg-stone-500/10"
          >
            [ Annuler ]
          </button>
          <button
            type="button"
            onClick={onConfirm}
            disabled={!canConfirm}
            className="rounded-sm border border-brass/50 bg-linear-to-b from-copper to-[#8a5426] px-4 py-2 font-mono text-[11px] uppercase tracking-widest text-amber-50 shadow-[inset_0_1px_0_rgba(255,255,255,0.3),0_2px_6px_rgba(0,0,0,0.5)] transition-all hover:brightness-110 active:translate-y-px disabled:cursor-not-allowed disabled:opacity-50"
          >
            [ {busy ? "Génération…" : "Confirmer & Enregistrer le PDF"} &rsaquo; ]
          </button>
        </footer>
      </div>
    </div>
  );
};
