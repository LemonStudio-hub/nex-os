/**
 * NexOS — Application entry point.
 *
 * Bootstraps the terminal, authenticates the user, initializes the WASM
 * shell, and wires up keyboard input.
 */

import { createTerminal, setupResize } from './terminal';
import { loadFromOPFS, saveToOPFS } from './persistence';
import { runAuth } from './auth';
import { setupInputHandler, type WasmApi } from './input';

async function main() {
  const loadingEl = document.getElementById('loading')!;
  const container = document.getElementById('terminal')!;

  // Create and mount terminal
  const { terminal, fitAddon, webLinksAddon } = createTerminal();
  terminal.open(container);
  terminal.loadAddon(fitAddon);
  terminal.loadAddon(webLinksAddon);
  fitAddon.fit();
  setupResize(terminal, fitAddon, container);

  // Load WASM module
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

  // Hide loading indicator
  loadingEl.classList.add('hidden');

  // Auth gate — must pass before the shell becomes usable
  const { username } = await runAuth(terminal);

  // Initialize VFS from OPFS and create the shell with the logged-in user
  const savedState = await loadFromOPFS();
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

  // Show prompt and hand off to input handler
  const prompt = wasm.get_prompt();
  terminal.write(prompt);
  setupInputHandler(terminal, wasm, prompt, saveToOPFS);
}

main();
