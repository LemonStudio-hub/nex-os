/**
 * Vite configuration for the NexOS frontend.
 *
 * The frontend is a vanilla TypeScript app that imports the Rust→WASM
 * bindings from `../pkg/`.  Two Vite plugins are required:
 *
 * - `vite-plugin-wasm` — enables `import` of `.wasm` files.
 * - `vite-plugin-top-level-await` — allows top-level `await` in ES modules
 *   (needed because the WASM initialisation is async).
 */

import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

export default defineConfig({
  plugins: [
    wasm(),
    topLevelAwait(),
  ],
  server: {
    // Allow the dev server to serve files outside the web/ directory.
    // This is required because the WASM bindings live in ../pkg/.
    fs: {
      allow: ['..'],
    },
  },
  build: {
    // Use the latest ES target so the output can use top-level await natively.
    target: 'esnext',
    // Output directory relative to this config file (web/dist/).
    outDir: 'dist',
  },
});
