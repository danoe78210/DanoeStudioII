import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    // Le dossier de compilation Rust (`src-tauri/target`) est verrouillé par
    // `cargo`/le linker pendant les builds : l'exclure du watcher évite les
    // erreurs `EBUSY` (fichiers `.pdb` / `.dll` verrouillés sous Windows).
    watch: {
      ignored: ['**/src-tauri/target/**'],
    },
  },
})
