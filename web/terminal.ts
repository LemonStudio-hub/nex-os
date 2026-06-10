/**
 * Terminal creation and resize handling.
 */

import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import '@xterm/xterm/css/xterm.css';

export interface TerminalInstance {
  terminal: Terminal;
  fitAddon: FitAddon;
  webLinksAddon: WebLinksAddon;
}

export function createTerminal(): TerminalInstance {
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

  return { terminal, fitAddon, webLinksAddon };
}

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
