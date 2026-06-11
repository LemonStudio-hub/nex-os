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

/**
 * Minimal type surface for the WASM module.
 *
 * Only the functions actually called from TypeScript are listed here;
 * the full WASM API has additional exports used by other modules.
 */
export interface WasmApi {
  /** Execute a command string and return its stdout/stderr output. */
  execute_command(input: string): string;
  /** Get the current prompt string (with ANSI colour codes). */
  get_prompt(): string;
  /** Get tab-completion candidates for the given partial input. */
  get_completions(input: string): string[];
  /** Serialise the current VFS state to JSON for OPFS persistence. */
  get_state_json(): string;
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
 * @param initialPrompt - The prompt string to display initially.
 * @param onSaveState   - Callback to persist VFS JSON to OPFS.
 */
export function setupInputHandler(
  terminal: Terminal,
  wasm: WasmApi,
  initialPrompt: string,
  onSaveState: (stateJson: string) => void,
): void {
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

        // Delegate to the WASM shell.
        const output = wasm.execute_command(inputBuffer);

        // The `clear` command returns a special ANSI escape sequence;
        // detect it and clear the terminal instead of printing it.
        if (output === '\x1b[2J\x1b[H') {
          terminal.clear();
        } else if (output.length > 0) {
          // Strip the trailing newline so the prompt sits flush with the
          // last line of output.
          const trimmed = output.endsWith('\n') ? output.slice(0, -1) : output;
          terminal.writeln(trimmed);
        }

        // Persist the VFS snapshot so the user's work survives reloads.
        const stateJson = wasm.get_state_json();
        if (stateJson) {
          onSaveState(stateJson);
        }
      }

      // Reset buffer, refresh the prompt (cwd may have changed), and display it.
      inputBuffer = '';
      prompt = wasm.get_prompt();
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
      const completions = wasm.get_completions(inputBuffer);
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
