import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  build: {
    chunkSizeWarningLimit: 1200,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes("node_modules")) {
            if (id.includes("pdfjs-dist")) {
              return "vendor-pdfjs";
            }
            if (id.includes("framer-motion")) {
              return "vendor-motion";
            }
            if (id.includes("@dnd-kit")) {
              return "vendor-dnd";
            }
            return "vendor";
          }
        },
      },
    },
  },
  server: {
    // Le dossier de compilation Rust (`src-tauri/target`) est verrouillé par
    // `cargo`/le linker pendant les builds : l'exclure du watcher évite les
    // erreurs `EBUSY` (fichiers `.pdb` / `.dll` verrouillés sous Windows).
    watch: {
      ignored: ['**/src-tauri/target/**'],
    },
  },
})
