/**
 * Keyboard input handler for the NexOS terminal.
 *
 * This module translates raw keystrokes from xterm.js into shell actions:
 * character accumulation, command submission, history navigation (↑/↓),
 * tab completion, and control-key shortcuts (Ctrl+C, Ctrl+L).
 *
 * After each executed command the VFS state is persisted to OPFS via the
 * `onSaveState` callback provided at setup time.
 *
 * @module input
 */

import type { Terminal } from '@xterm/xterm';
import type { HostFsManager } from './host-fs';

/**
 * Minimal type surface for the WASM module.
 *
 * All methods accept the current shell state as a JSON string (the first
 * parameter).  State-mutating operations return updated state alongside
 * their result.
 */
export interface WasmApi {
  /** Initialize the service and return the initial shell state JSON. */
  init(stateJson: string): string;
  /** Initialize with a custom username and return the initial shell state JSON. */
  init_with_username(stateJson: string, username: string): string;
  /**
   * Execute a command string against the given state.
   * Returns JSON: `{"output": "...", "state": "..."}`.
   */
  execute_command(state: string, input: string): string;
  /** Get the current prompt string (with ANSI colour codes). */
  get_prompt(state: string): string;
  /** Get tab-completion candidates for the given partial input. */
  get_completions(state: string, input: string): string[];
  /** Get command history from the given state. */
  get_history(state: string): string[];
  /** Serialise the VFS from the given state to JSON for OPFS persistence. */
  get_state_json(state: string): string;
  /** Get dirty (modified/created) file paths as a JSON array. */
  get_dirty_files_json(state: string): string;
  /** Get deleted file paths as a JSON array. */
  get_deleted_files_json(state: string): string;
  /** Get the content of a single file. */
  get_file_content(state: string, path: string): string;
  /** Mark every file as dirty (for migration). */
  mark_all_dirty(state: string): string;
  /** Serialize the VFS tree structure with empty file contents. */
  get_tree_json(state: string): string;
  /** Register host FS callbacks for a mount. */
  register_host_fs(mountId: string, callbacks: object): string;
  /** Unregister host FS callbacks for a mount. */
  unregister_host_fs(mountId: string): string;
}

/**
 * Parsed result from `execute_command`.
 */
interface ExecuteResult {
  output: string;
  state: string;
}

/**
 * Attach the main keyboard input handler to the terminal.
 *
 * This function takes ownership of the terminal's `onData` event for the
 * lifetime of the session.  It manages:
 *
 * - **Input buffer** — characters typed since the last prompt.
 * - **Command history** — navigable with ↑/↓ arrow keys.
 * - **Tab completion** — queries the WASM shell for matching commands.
 * - **Command execution** — delegates to `wasm.execute_command()`.
 * - **State persistence** — calls `onSaveState` after every command.
 *
 * @param terminal      - The xterm.js terminal instance.
 * @param wasm          - The initialised WASM API bindings.
 * @param initialState  - The initial shell state JSON string.
 * @param initialPrompt - The prompt string to display initially.
 * @param onSaveState   - Callback to persist VFS JSON to OPFS.
 */
export function setupInputHandler(
  terminal: Terminal,
  wasm: WasmApi,
  initialState: string,
  initialPrompt: string,
  onSaveState: (stateJson: string) => void,
  hostFsManager?: HostFsManager,
): void {
  // The current shell state — updated after every mutating operation.
  let stateJson = initialState;
  // The current line being edited (not yet submitted).
  let inputBuffer = '';
  // Index into `history` for arrow-key navigation; -1 = not browsing.
  // `history.length` means "at the empty line below the last entry".
  let historyIndex = -1;
  // Chronological list of previously submitted commands.
  const history: string[] = [];
  // The current prompt text (may change after `cd` or `export`).
  let prompt = initialPrompt;

  terminal.onData((data: string) => {
    const code = data.charCodeAt(0);

    // -----------------------------------------------------------------
    // Enter — submit the current input buffer for execution
    // -----------------------------------------------------------------
    if (data === '\r') {
      terminal.writeln('');

      if (inputBuffer.trim().length > 0) {
        // Record the command in history and reset the history browser.
        history.push(inputBuffer);
        historyIndex = history.length;

        // Delegate to the WASM shell — returns {output, state}.
        const raw = wasm.execute_command(stateJson, inputBuffer);
        let result: ExecuteResult;
        try {
          result = JSON.parse(raw);
        } catch {
          result = { output: raw, state: stateJson };
        }

        // Update the stored state.
        stateJson = result.state;

        // Detect mount request: the `mount` command returns a special
        // error marker that signals the frontend to open the directory picker.
        if (result.output.startsWith('__MOUNT_REQUEST__')) {
          const vfsPath = result.output.replace('__MOUNT_REQUEST__', '').trim();
          handleMountRequest(terminal, wasm, vfsPath, hostFsManager)
            .then((newState) => {
              if (newState) {
                stateJson = newState;
                onSaveState(stateJson);
              }
              prompt = wasm.get_prompt(stateJson);
              terminal.write(prompt);
            });
          // Don't write prompt here — the async handler will do it
          inputBuffer = '';
          return;
        }

        // The `clear` command returns a special ANSI escape sequence;
        // detect it and clear the terminal instead of printing it.
        if (result.output === '\x1b[2J\x1b[H') {
          terminal.clear();
        } else if (result.output.length > 0) {
          // Strip the trailing newline so the prompt sits flush with the
          // last line of output.
          const trimmed = result.output.endsWith('\n')
            ? result.output.slice(0, -1)
            : result.output;
          terminal.writeln(trimmed);
        }

        // Flush any pending host FS writes.
        if (hostFsManager) {
          hostFsManager.flushWrites();
        }

        // Persist the VFS snapshot so the user's work survives reloads.
        // Pass the full state JSON so the persistence layer can access
        // dirty-tracking info for incremental saves.
        if (stateJson) {
          onSaveState(stateJson);
        }
      }

      // Reset buffer, refresh the prompt (cwd may have changed), and display it.
      inputBuffer = '';
      prompt = wasm.get_prompt(stateJson);
      terminal.write(prompt);
      return;
    }

    // -----------------------------------------------------------------
    // Ctrl+C — cancel the current input
    // -----------------------------------------------------------------
    if (data === '\x03') {
      terminal.writeln('^C');
      inputBuffer = '';
      terminal.write(prompt);
      return;
    }

    // -----------------------------------------------------------------
    // Ctrl+L — clear the screen (keep input buffer intact)
    // -----------------------------------------------------------------
    if (data === '\x0c') {
      terminal.clear();
      terminal.write(prompt);
      return;
    }

    // -----------------------------------------------------------------
    // Backspace — delete the last character
    // -----------------------------------------------------------------
    if (data === '\x7f' || code === 8) {
      if (inputBuffer.length > 0) {
        inputBuffer = inputBuffer.slice(0, -1);
        // Erase the character visually: move back, overwrite with space,
        // move back again.
        terminal.write('\b \b');
      }
      return;
    }

    // -----------------------------------------------------------------
    // Arrow Up — navigate backwards through command history
    // -----------------------------------------------------------------
    if (data === '\x1b[A') {
      if (historyIndex > 0) {
        historyIndex--;
        clearCurrentInput(terminal, inputBuffer);
        inputBuffer = history[historyIndex];
        terminal.write(inputBuffer);
      }
      return;
    }

    // -----------------------------------------------------------------
    // Arrow Down — navigate forwards through command history
    // -----------------------------------------------------------------
    if (data === '\x1b[B') {
      if (historyIndex < history.length - 1) {
        // Move to the next (newer) history entry.
        historyIndex++;
        clearCurrentInput(terminal, inputBuffer);
        inputBuffer = history[historyIndex];
        terminal.write(inputBuffer);
      } else if (historyIndex === history.length - 1) {
        // Past the last entry — show an empty input line.
        historyIndex = history.length;
        clearCurrentInput(terminal, inputBuffer);
        inputBuffer = '';
      }
      return;
    }

    // -----------------------------------------------------------------
    // Tab — trigger tab completion
    // -----------------------------------------------------------------
    if (data === '\t') {
      const completions = wasm.get_completions(stateJson, inputBuffer);
      if (completions.length === 1) {
        // Single match: auto-complete and add a trailing space.
        const rest = completions[0].slice(inputBuffer.length);
        inputBuffer += rest;
        terminal.write(rest + ' ');
      } else if (completions.length > 1) {
        // Multiple matches: print them on a new line, then re-draw the
        // current prompt + input so the user can continue typing.
        terminal.writeln('');
        terminal.writeln(completions.join('  '));
        terminal.write(prompt + inputBuffer);
      }
      return;
    }

    // -----------------------------------------------------------------
    // Escape sequences — ignore everything except the arrows handled above
    // -----------------------------------------------------------------
    if (code === 27) return;

    // -----------------------------------------------------------------
    // Printable character — append to the input buffer
    // -----------------------------------------------------------------
    if (code >= 32) {
      inputBuffer += data;
      terminal.write(data);
    }
  });
}

/**
 * Erase the currently displayed input from the terminal.
 *
 * Moves the cursor back one character at a time, overwrites each character
 * with a space, and moves back again — effectively clearing the line
 * without affecting the scrollback buffer.
 *
 * @param terminal - The xterm.js instance.
 * @param input    - The string currently displayed on the input line.
 */
function clearCurrentInput(terminal: Terminal, input: string): void {
  for (let i = 0; i < input.length; i++) {
    terminal.write('\b \b');
  }
}

/**
 * Handle a mount request from the `mount` command.
 *
 * Opens the browser's directory picker, caches the selected directory,
 * registers the host FS callbacks with WASM, and updates the VFS state.
 *
 * @returns The updated state JSON, or null if the mount was cancelled.
 */
async function handleMountRequest(
  terminal: Terminal,
  wasm: WasmApi,
  vfsPath: string,
  hostFsManager?: HostFsManager,
): Promise<string | null> {
  if (!hostFsManager) {
    terminal.writeln('\x1b[31mMount not supported in this context.\x1b[0m');
    return null;
  }

  try {
    // @ts-expect-error: showDirectoryPicker may not be in all type defs
    const handle: FileSystemDirectoryHandle = await window.showDirectoryPicker({
      mode: 'readwrite',
    });

    terminal.writeln(`\x1b[33mMounting ${handle.name} at ${vfsPath}...\x1b[0m`);

    // Pre-cache the directory contents and register callbacks
    const mountId = await hostFsManager.mount(handle, vfsPath);
    const callbacks = hostFsManager.getCallbacks(mountId);
    if (callbacks) {
      wasm.register_host_fs(mountId, callbacks);
    }

    // Update the mount metadata in VFS state via a shell command
    // The mount command already created the VFS entry; we just need to
    // update the host name. Use a no-op command to get the state back.
    const updateResult = wasm.execute_command(
      wasm.get_state_json(wasm.init_with_username('', 'user')),
      `export NEXOS_MOUNT_${mountId}=${vfsPath}`,
    );

    terminal.writeln(
      `\x1b[32mMounted ${handle.name} at ${vfsPath}\x1b[0m`,
    );

    // Return the current state (which already has the mount metadata)
    return null; // The state was already updated by the mount command
  } catch (e) {
    if (e instanceof DOMException && e.name === 'AbortError') {
      terminal.writeln('\x1b[33mMount cancelled.\x1b[0m');
    } else {
      terminal.writeln(`\x1b[31mMount failed: ${e}\x1b[0m`);
    }
    return null;
  }
}
