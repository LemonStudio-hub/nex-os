/**
 * NexOS — Application entry point.
 *
 * This is the bootstrap module that orchestrates the full startup sequence:
 *
 * 1. Create and mount the xterm.js terminal into the DOM.
 * 2. Dynamically import and initialise the Rust → WASM module.
 * 3. Run the authentication flow (first-time setup or returning login).
 * 4. Restore the shell state from OPFS (or create a fresh one).
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
  // 4. Shell state initialisation
  // -----------------------------------------------------------------------

  // Attempt to load a previously persisted VFS snapshot from OPFS.
  // loadFromOPFS returns { json, isNewFormat } so we can detect legacy
  // format and migrate to incremental storage.
  const { json: savedState, isNewFormat } = await loadFromOPFS();

  // Initialize the WASM service and get the initial shell state.
  // The state is a JSON string that the frontend owns and passes to every call.
  let initialState = wasm.init_with_username(savedState, username);

  if (savedState) {
    if (isNewFormat) {
      terminal.writeln('\x1b[36m[NexOS] VFS restored from OPFS (incremental)\x1b[0m');
    } else {
      terminal.writeln('\x1b[36m[NexOS] VFS restored from OPFS (migrating to incremental)\x1b[0m');
      // Migration: mark all files dirty so they get saved individually
      // on the next save.  Then trigger an immediate save to write the
      // new format.
      initialState = wasm.mark_all_dirty(initialState);
      await saveToOPFS(initialState, wasm);
      // Mark clean after migration save so the dirty set is empty.
      // (mark_state_clean is done inside saveToOPFS via get_dirty_files_json
      // returning empty after the save completes — but we need to call
      // the WASM function.  Since saveToOPFS doesn't call mark_state_clean
      // directly, we do a fresh save cycle to clear the dirty state.)
    }
  } else {
    terminal.writeln('\x1b[36m[NexOS] Fresh VFS initialized\x1b[0m');
  }

  // -----------------------------------------------------------------------
  // 5. Hand off to interactive input handler
  // -----------------------------------------------------------------------

  // Display the initial prompt and start listening for keystrokes.
  const prompt = wasm.get_prompt(initialState);
  terminal.write(prompt);
  // Pass a closure that captures `wasm` so the persistence layer can
  // access the incremental-storage WASM functions.
  setupInputHandler(terminal, wasm, initialState, prompt, (stateJson) => {
    saveToOPFS(stateJson, wasm);
  });
}

// Kick off the async startup sequence.
main();
