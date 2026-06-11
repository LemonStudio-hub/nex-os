/**
 * Terminal creation and resize handling.
 *
 * This module owns the xterm.js {@link Terminal} instance and ensures it
 * stays correctly sized when the browser window or its container element
 * is resized.
 *
 * @module terminal
 */

import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
// Import the xterm CSS so Vite bundles it into the final stylesheet.
import '@xterm/xterm/css/xterm.css';

/**
 * Bundle returned by {@link createTerminal}.
 *
 * Contains the terminal itself plus the two addons that are loaded into it.
 */
export interface TerminalInstance {
  /** The xterm.js terminal emulator instance. */
  terminal: Terminal;
  /** Addon that automatically fits the terminal to its container size. */
  fitAddon: FitAddon;
  /** Addon that makes URL-like text clickable. */
  webLinksAddon: WebLinksAddon;
}

/**
 * Create a fully configured xterm.js terminal with sensible defaults.
 *
 * The terminal is **not** opened or mounted here — the caller is
 * responsible for calling `terminal.open(container)` and loading the
 * addons via `terminal.loadAddon(...)`.
 *
 * @returns A {@link TerminalInstance} ready to be mounted.
 */
export function createTerminal(): TerminalInstance {
  const terminal = new Terminal({
    cursorBlink: true,
    fontSize: 14,
    fontFamily: '"Cascadia Code", Menlo, "Courier New", monospace',
    theme: {
      background: '#1e1e1e',   // dark background
      foreground: '#cccccc',   // light grey text
      cursor: '#ffffff',       // white cursor
      selectionBackground: '#264f78', // blue selection highlight
    },
    allowProposedApi: true,
  });

  const fitAddon = new FitAddon();
  const webLinksAddon = new WebLinksAddon();

  return { terminal, fitAddon, webLinksAddon };
}

/**
 * Observe the given `container` element and re-fit the terminal whenever
 * its size changes.
 *
 * This uses a {@link ResizeObserver} so the terminal stays responsive
 * across window resizes, sidebar toggles, and other layout shifts.
 *
 * @param terminal  - The xterm.js instance.
 * @param fitAddon  - The FitAddon loaded into the terminal.
 * @param container - The DOM element that contains the terminal.
 * @returns The ResizeObserver instance (caller can disconnect it later).
 */
export function setupResize(
  terminal: Terminal,
  fitAddon: FitAddon,
  container: HTMLElement,
): ResizeObserver {
  const resizeObserver = new ResizeObserver(() => {
    fitAddon.fit();
  });
  resizeObserver.observe(container);
  return resizeObserver;
}
