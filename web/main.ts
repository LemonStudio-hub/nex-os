/**
 * NexOS — Application entry point.
 *
 * This is the bootstrap module that orchestrates the full startup sequence:
 *
 * 1. Create and mount the xterm.js terminal into the DOM.
 * 2. Dynamically import and initialise the Rust → WASM module.
 * 3. Run the authentication flow (first-time setup or returning login).
 * 4. Restore the virtual file system from OPFS (or create a fresh one).
 * 5. Hand off control to the keyboard input handler.
 *
 * All heavy lifting (command parsing, VFS operations) happens inside the
 * WASM module; this file is intentionally thin.
 */

import { createTerminal, setupResize } from './terminal';
import { loadFromOPFS, saveToOPFS } from './persistence';
import { runAuth } from './auth';
import { setupInputHandler, type WasmApi } from './input';

/**
 * Main entry point — called once at the bottom of this file.
 *
 * Every step that can fail is wrapped in a try/catch so the user sees a
 * meaningful error in the loading overlay rather than a blank screen.
 */
async function main() {
  // Grab DOM elements for the loading overlay and the terminal container.
  const loadingEl = document.getElementById('loading')!;
  const container = document.getElementById('terminal')!;

  // -----------------------------------------------------------------------
  // 1. Terminal setup
  // -----------------------------------------------------------------------

  // Create the xterm.js instance with fit and web-links addons, then mount
  // it into the #terminal container and perform the initial size fit.
  const { terminal, fitAddon, webLinksAddon } = createTerminal();
  terminal.open(container);
  terminal.loadAddon(fitAddon);
  terminal.loadAddon(webLinksAddon);
  fitAddon.fit();

  // Keep the terminal size synchronised with the browser window.
  setupResize(terminal, fitAddon, container);

  // -----------------------------------------------------------------------
  // 2. WASM module loading
  // -----------------------------------------------------------------------

  // Dynamically import the wasm-bindgen generated JS glue.  The `default()`
  // call initialises the WASM memory and must complete before any exported
  // function can be called.
  let wasm: WasmApi | null = null;
  try {
    const mod = await import('../pkg/nexos');
    await (mod as unknown as { default: () => Promise<void> }).default();
    wasm = mod as unknown as WasmApi;
  } catch (e) {
    loadingEl.textContent = 'Failed to load WASM module.';
    console.error(e);
    return;
  }

  // WASM loaded successfully — hide the "Loading…" overlay.
  loadingEl.classList.add('hidden');

  // -----------------------------------------------------------------------
  // 3. Authentication
  // -----------------------------------------------------------------------

  // The auth flow blocks until the user provides valid credentials.
  // It takes over the terminal's onData handler for the duration of the flow.
  const { username } = await runAuth(terminal);

  // -----------------------------------------------------------------------
  // 4. VFS initialisation
  // -----------------------------------------------------------------------

  // Attempt to load a previously persisted VFS snapshot from OPFS.
  // If nothing is stored (first visit), `savedState` will be an empty string.
  const savedState = await loadFromOPFS();

  // Pass the saved state and the authenticated username to the WASM shell.
  // `init_with_username` returns `true` when the VFS was restored from JSON.
  const restored = (
    wasm as unknown as {
      init_with_username(state: string, user: string): boolean;
    }
  ).init_with_username(savedState, username);

  if (restored) {
    terminal.writeln('\x1b[36m[NexOS] VFS restored from OPFS\x1b[0m');
  } else {
    terminal.writeln('\x1b[36m[NexOS] Fresh VFS initialized\x1b[0m');
  }

  // -----------------------------------------------------------------------
  // 5. Hand off to interactive input handler
  // -----------------------------------------------------------------------

  // Display the initial prompt and start listening for keystrokes.
  const prompt = wasm.get_prompt();
  terminal.write(prompt);
  setupInputHandler(terminal, wasm, prompt, saveToOPFS);
}

// Kick off the async startup sequence.
main();
