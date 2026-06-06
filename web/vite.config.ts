import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

export default defineConfig({
  plugins: [
    wasm(),
    topLevelAwait(),
  ],
  server: {
    // Allow serving files from the parent directory (for ../pkg/)
    fs: {
      allow: ['..'],
    },
  },
  build: {
    target: 'esnext',
    outDir: 'dist',
  },
});
