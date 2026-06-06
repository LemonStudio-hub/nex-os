/**
 * Authentication module for web-code.
 *
 * Handles first-time user setup (username + password) and returning-user
 * login (password only). Credentials are stored in OPFS with SHA-256
 * hashed passwords via the Web Crypto API.
 */

import type { Terminal } from '@xterm/xterm';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface UserConfig {
  username: string;
  passwordHash: string; // hex-encoded SHA-256
}

// ---------------------------------------------------------------------------
// OPFS helpers for user config
// ---------------------------------------------------------------------------

const CONFIG_FILE = 'user_config.json';

async function loadUserConfig(): Promise<UserConfig | null> {
  try {
    const root = await navigator.storage.getDirectory();
    const fh = await root.getFileHandle(CONFIG_FILE);
    const file = await fh.getFile();
    const text = await file.text();
    return JSON.parse(text) as UserConfig;
  } catch {
    return null;
  }
}

async function saveUserConfig(config: UserConfig): Promise<void> {
  try {
    const root = await navigator.storage.getDirectory();
    const fh = await root.getFileHandle(CONFIG_FILE, { create: true });
    const writable = await fh.createWritable();
    await writable.write(JSON.stringify(config));
    await writable.close();
  } catch (e) {
    console.warn('[web-code] Failed to save user config:', e);
  }
}

// ---------------------------------------------------------------------------
// SHA-256 hashing (Web Crypto API)
// ---------------------------------------------------------------------------

async function sha256(input: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(input);
  const hashBuffer = await crypto.subtle.digest('SHA-256', data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map((b) => b.toString(16).padStart(2, '0')).join('');
}

// ---------------------------------------------------------------------------
// Terminal helpers
// ---------------------------------------------------------------------------

/** Write a line to the terminal, then write a prompt with the given label. */
function writePrompt(terminal: Terminal, label: string): void {
  terminal.write(`\r\n${label} `);
}

/** Erase the current input from the terminal display. */
function eraseInput(terminal: Terminal, buffer: string): void {
  for (let i = 0; i < buffer.length; i++) {
    terminal.write('\b \b');
  }
}

// ---------------------------------------------------------------------------
// Auth flow types
// ---------------------------------------------------------------------------

type AuthMode =
  | 'SETUP_USERNAME'
  | 'SETUP_PASSWORD'
  | 'SETUP_CONFIRM'
  | 'LOGIN_PASSWORD';

export interface AuthResult {
  username: string;
}

// ---------------------------------------------------------------------------
// Main auth entry point
// ---------------------------------------------------------------------------

/**
 * Run the authentication flow.
 *
 * - If no user config exists in OPFS → first-time setup (pick username + password).
 * - If user config exists → password-only login.
 *
 * Resolves with the authenticated username on success. The function takes
 * ownership of the terminal's `onData` handler for the duration of the flow.
 */
export async function runAuth(terminal: Terminal): Promise<AuthResult> {
  const config = await loadUserConfig();

  return new Promise<AuthResult>((resolve) => {
    let mode: AuthMode;
    let inputBuffer = '';
    let setupUsername = '';
    let setupPassword = '';

    if (config) {
      // Returning user – ask for password only
      terminal.writeln('');
      terminal.writeln('\x1b[1;36mweb-code\x1b[0m — login required');
      writePrompt(terminal, 'Password:');
      mode = 'LOGIN_PASSWORD';
    } else {
      // First-time setup
      terminal.writeln('');
      terminal.writeln('\x1b[1;36mweb-code\x1b[0m — first-time setup');
      terminal.writeln('Create your account to get started.');
      writePrompt(terminal, 'Username:');
      mode = 'SETUP_USERNAME';
    }

    const handler = terminal.onData((data: string) => {
      const code = data.charCodeAt(0);

      // ---- Ctrl+C: cancel and restart ----
      if (data === '\x03') {
        terminal.writeln('^C');
        inputBuffer = '';
        if (config) {
          writePrompt(terminal, 'Password:');
          mode = 'LOGIN_PASSWORD';
        } else {
          writePrompt(terminal, 'Username:');
          mode = 'SETUP_USERNAME';
        }
        return;
      }

      // ---- Backspace ----
      if (data === '\x7f' || code === 8) {
        if (inputBuffer.length > 0) {
          inputBuffer = inputBuffer.slice(0, -1);
          // Erase the visual character (password mode still shows *)
          terminal.write('\b \b');
        }
        return;
      }

      // ---- Ignore other control / escape sequences ----
      // Note: \r (code 13) and \n (code 10) are handled below, so skip them here
      if (code < 32 && data !== '\r' && data !== '\n') return;

      // ---- Printable character (echo before checking Enter) ----
      if (data !== '\r' && data !== '\n') {
        inputBuffer += data;
        // Echo * for password fields, the character itself for username
        if (mode === 'SETUP_PASSWORD' || mode === 'SETUP_CONFIRM' || mode === 'LOGIN_PASSWORD') {
          terminal.write('*');
        } else {
          terminal.write(data);
        }
        return;
      }

      // ---- Enter pressed ----
      // Strip trailing \r from buffer (should be empty now, but defensive)
      const value = inputBuffer.replace(/\r$/, '');
      inputBuffer = '';

      switch (mode) {
        // ============================================================
        // SETUP_USERNAME
        // ============================================================
        case 'SETUP_USERNAME': {
          const name = value.trim();
          if (name.length === 0) {
            terminal.writeln('');
            terminal.writeln('\x1b[31mUsername cannot be empty.\x1b[0m');
            writePrompt(terminal, 'Username:');
            break;
          }
          if (name.length > 32) {
            terminal.writeln('');
            terminal.writeln('\x1b[31mUsername too long (max 32 chars).\x1b[0m');
            writePrompt(terminal, 'Username:');
            break;
          }
          setupUsername = name;
          terminal.writeln('');
          writePrompt(terminal, 'Password:');
          mode = 'SETUP_PASSWORD';
          break;
        }

        // ============================================================
        // SETUP_PASSWORD
        // ============================================================
        case 'SETUP_PASSWORD': {
          if (value.length < 1) {
            terminal.writeln('');
            terminal.writeln('\x1b[31mPassword cannot be empty.\x1b[0m');
            writePrompt(terminal, 'Password:');
            break;
          }
          setupPassword = value;
          terminal.writeln('');
          writePrompt(terminal, 'Confirm password:');
          mode = 'SETUP_CONFIRM';
          break;
        }

        // ============================================================
        // SETUP_CONFIRM
        // ============================================================
        case 'SETUP_CONFIRM': {
          if (value !== setupPassword) {
            terminal.writeln('');
            terminal.writeln('\x1b[31mPasswords do not match. Try again.\x1b[0m');
            writePrompt(terminal, 'Password:');
            mode = 'SETUP_PASSWORD';
            setupPassword = '';
            break;
          }

          // Hash and persist
          sha256(setupPassword).then((hash) => {
            saveUserConfig({ username: setupUsername, passwordHash: hash });
            terminal.writeln('');
            terminal.writeln(
              `\x1b[32mAccount created for \x1b[1m${setupUsername}\x1b[0m`,
            );
            handler.dispose();
            resolve({ username: setupUsername });
          });
          break;
        }

        // ============================================================
        // LOGIN_PASSWORD
        // ============================================================
        case 'LOGIN_PASSWORD': {
          sha256(value).then((hash) => {
            if (hash === config!.passwordHash) {
              terminal.writeln('');
              terminal.writeln(`\x1b[32mWelcome back, ${config!.username}!\x1b[0m`);
              handler.dispose();
              resolve({ username: config!.username });
            } else {
              terminal.writeln('');
              terminal.writeln('\x1b[31mIncorrect password.\x1b[0m');
              writePrompt(terminal, 'Password:');
            }
          });
          break;
        }
      }
    });
  });
}
