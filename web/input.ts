/**
 * Keyboard input handler for the terminal.
 */

import type { Terminal } from '@xterm/xterm';

/** Type of the WASM module (partial — only the functions we call). */
export interface WasmApi {
  execute_command(input: string): string;
  get_prompt(): string;
  get_completions(input: string): string[];
  get_state_json(): string;
}

/**
 * Set up the main keyboard input handler on the terminal.
 *
 * Manages the input buffer, command history, tab completion, and delegates
 * command execution to the WASM shell.
 */
export function setupInputHandler(
  terminal: Terminal,
  wasm: WasmApi,
  initialPrompt: string,
  onSaveState: (stateJson: string) => void,
): void {
  let inputBuffer = '';
  let historyIndex = -1;
  const history: string[] = [];
  let prompt = initialPrompt;

  terminal.onData((data: string) => {
    const code = data.charCodeAt(0);

    // Enter
    if (data === '\r') {
      terminal.writeln('');

      if (inputBuffer.trim().length > 0) {
        history.push(inputBuffer);
        historyIndex = history.length;

        const output = wasm.execute_command(inputBuffer);

        // Check for clear command (ANSI clear sequence)
        if (output === '\x1b[2J\x1b[H') {
          terminal.clear();
        } else if (output.length > 0) {
          // Write output, removing trailing newline to align with prompt
          const trimmed = output.endsWith('\n') ? output.slice(0, -1) : output;
          terminal.writeln(trimmed);
        }

        // Save state to OPFS after each command
        const stateJson = wasm.get_state_json();
        if (stateJson) {
          onSaveState(stateJson);
        }
      }

      inputBuffer = '';
      prompt = wasm.get_prompt();
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
      const completions = wasm.get_completions(inputBuffer);
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

function clearCurrentInput(terminal: Terminal, input: string): void {
  for (let i = 0; i < input.length; i++) {
    terminal.write('\b \b');
  }
}
