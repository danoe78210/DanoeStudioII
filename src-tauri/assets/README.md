# Assets du backend Tauri — `src-tauri/assets/`

Ce dossier est **réservé aux ressources binaires** empaquetées avec l'exécutable Tauri.

## Couverture cuir (export Word)

Déposer ici le fichier fourni par l'utilisateur, nommé **exactement** :

```
Couverture_Cuir_01.jpg
```

> ⚠️ Ce binaire **n'est pas versionné** dans le dépôt (il est transmis séparément par
> l'utilisateur). Le moteur d'export Rust (`docx-rs`) le charge en mémoire et l'utilise comme
> **image de fond de la première page** du `.docx` — voir `SPECIFICATIONS_DANOE_STUDIO.md`,
> section « Moteur d'export — Couverture cuir (Word) ».

## `tauri.conf.json` — empaquetage de la ressource

```json
{
  "bundle": {
    "resources": ["assets/Couverture_Cuir_01.jpg"]
  }
}
```

(Le chemin est relatif à la racine `src-tauri/`.)

## Chargement côté Rust

Compilation (ressource embarquée) :

```rust
const COVER_BYTES: &[u8] = include_bytes!("../assets/Couverture_Cuir_01.jpg");
```

Ou à l'exécution (chemin résolu depuis les ressources du bundle) :

```rust
let path = app
    .path()
    .resolve("assets/Couverture_Cuir_01.jpg", tauri::path::BaseDirectory::Resource)?;
let cover_bytes = std::fs::read(path)?;
```