import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import '@xterm/xterm/css/xterm.css';
import { runAuth } from './auth';

// ---------------------------------------------------------------------------
// OPFS persistence helpers (VFS state)
// ---------------------------------------------------------------------------

const OPFS_FILE = 'vfs_state.json';

async function loadFromOPFS(): Promise<string> {
  try {
    const root = await navigator.storage.getDirectory();
    const fileHandle = await root.getFileHandle(OPFS_FILE);
    const file = await fileHandle.getFile();
    return await file.text();
  } catch {
    return '';
  }
}

async function saveToOPFS(data: string): Promise<void> {
  try {
    const root = await navigator.storage.getDirectory();
    const fileHandle = await root.getFileHandle(OPFS_FILE, { create: true });
    const writable = await fileHandle.createWritable();
    await writable.write(data);
    await writable.close();
  } catch (e) {
    console.warn('[web-code] OPFS save failed:', e);
  }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  const loadingEl = document.getElementById('loading')!;

  // Create terminal
  const terminal = new Terminal({
    cursorBlink: true,
    fontSize: 14,
    fontFamily: '"Cascadia Code", Menlo, "Courier New", monospace',
    theme: {
      background: '#1e1e1e',
      foreground: '#cccccc',
      cursor: '#ffffff',
      selectionBackground: '#264f78',
    },
    allowProposedApi: true,
  });

  const fitAddon = new FitAddon();
  const webLinksAddon = new WebLinksAddon();

  const container = document.getElementById('terminal')!;
  terminal.open(container);

  // Load addons AFTER open
  terminal.loadAddon(fitAddon);
  terminal.loadAddon(webLinksAddon);

  // Fit terminal to container
  fitAddon.fit();

  // Auto-resize
  const resizeObserver = new ResizeObserver(() => {
    fitAddon.fit();
  });
  resizeObserver.observe(container);

  // Load WASM module
  let wasm: typeof import('../pkg/web_code') | null = null;
  try {
    wasm = await import('../pkg/web_code');
    await wasm.default();
  } catch (e) {
    loadingEl.textContent = 'Failed to load WASM module.';
    console.error(e);
    return;
  }

  // Hide loading indicator
  loadingEl.classList.add('hidden');

  // ---------------------------------------------------------------
  // Auth gate – must pass before the shell becomes usable
  // ---------------------------------------------------------------
  const { username } = await runAuth(terminal);

  // Initialize VFS from OPFS and create the shell with the logged-in user
  const savedState = await loadFromOPFS();
  const restored = wasm.init_with_username(savedState, username);

  if (restored) {
    terminal.writeln('\x1b[36m[web-code] VFS restored from OPFS\x1b[0m');
  } else {
    terminal.writeln('\x1b[36m[web-code] Fresh VFS initialized\x1b[0m');
  }

  // Show prompt
  let prompt = wasm.get_prompt();
  terminal.write(prompt);

  // Input buffer
  let inputBuffer = '';
  let historyIndex = -1;
  const history: string[] = [];

  // Handle keyboard input
  terminal.onData((data) => {
    const code = data.charCodeAt(0);

    // Enter
    if (data === '\r') {
      terminal.writeln('');

      if (inputBuffer.trim().length > 0) {
        history.push(inputBuffer);
        historyIndex = history.length;

        const output = wasm!.execute_command(inputBuffer);

        // Check for clear command (ANSI clear sequence)
        if (output === '\x1b[2J\x1b[H') {
          terminal.clear();
        } else if (output.length > 0) {
          // Write output, removing trailing newline to align with prompt
          const trimmed = output.endsWith('\n') ? output.slice(0, -1) : output;
          terminal.writeln(trimmed);
        }

        // Save state to OPFS after each command
        const stateJson = wasm!.get_state_json();
        if (stateJson) {
          saveToOPFS(stateJson);
        }
      }

      inputBuffer = '';
      prompt = wasm!.get_prompt();
      terminal.write(prompt);
      return;
    }

    // Ctrl+C
    if (data === '\x03') {
      terminal.writeln('^C');
      inputBuffer = '';
      terminal.write(prompt);
      return;
    }

    // Ctrl+L (clear)
    if (data === '\x0c') {
      terminal.clear();
      terminal.write(prompt);
      return;
    }

    // Backspace
    if (data === '\x7f' || code === 8) {
      if (inputBuffer.length > 0) {
        inputBuffer = inputBuffer.slice(0, -1);
        terminal.write('\b \b');
      }
      return;
    }

    // Arrow Up
    if (data === '\x1b[A') {
      if (historyIndex > 0) {
        historyIndex--;
        clearCurrentInput(terminal, inputBuffer);
        inputBuffer = history[historyIndex];
        terminal.write(inputBuffer);
      }
      return;
    }

    // Arrow Down
    if (data === '\x1b[B') {
      if (historyIndex < history.length - 1) {
        historyIndex++;
        clearCurrentInput(terminal, inputBuffer);
        inputBuffer = history[historyIndex];
        terminal.write(inputBuffer);
      } else if (historyIndex === history.length - 1) {
        historyIndex = history.length;
        clearCurrentInput(terminal, inputBuffer);
        inputBuffer = '';
      }
      return;
    }

    // Tab (completion)
    if (data === '\t') {
      const completions = wasm!.get_completions(inputBuffer);
      if (completions.length === 1) {
        const rest = completions[0].slice(inputBuffer.length);
        inputBuffer += rest;
        terminal.write(rest + ' ');
      } else if (completions.length > 1) {
        terminal.writeln('');
        terminal.writeln(completions.join('  '));
        terminal.write(prompt + inputBuffer);
      }
      return;
    }

    // Escape sequences (ignore other arrow keys etc.)
    if (code === 27) return;

    // Printable character
    if (code >= 32) {
      inputBuffer += data;
      terminal.write(data);
    }
  });
}

function clearCurrentInput(terminal: Terminal, input: string) {
  for (let i = 0; i < input.length; i++) {
    terminal.write('\b \b');
  }
}

main();
