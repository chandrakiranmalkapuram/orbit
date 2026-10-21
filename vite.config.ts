import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({ plugins: [react()], clearScreen: false, server: { strictPort: true, port: 1420 }, envPrefix: ['VITE_', 'TAURI_'], build: { target: ['safari13'], minify: true },
  // Tauri expects the dev server to remain on a predictable local port.
});
