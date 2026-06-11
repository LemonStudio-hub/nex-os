/**
 * Authentication module for NexOS.
 *
 * Handles two distinct flows:
 *
 * 1. **First-time setup** — the user picks a username and password.
 *    The password is hashed with SHA-256 via the Web Crypto API and
 *    persisted to OPFS alongside the username.
 *
 * 2. **Returning login** — the user enters their password; the hash is
 *    compared against the stored value.
 *
 * Credentials are stored in `user_config.json` inside the browser's
 * Origin Private File System (OPFS), which is scoped to the page's
 * origin and never leaves the device.
 *
 * @module auth
 */

import type { Terminal } from '@xterm/xterm';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** The shape of the persisted user configuration file. */
interface UserConfig {
  /** The user's chosen display name. */
  username: string;
  /** Hex-encoded SHA-256 hash of the user's password. */
  passwordHash: string;
}

// ---------------------------------------------------------------------------
// OPFS helpers for user config
// ---------------------------------------------------------------------------

/** Filename used inside OPFS to store the user configuration. */
const CONFIG_FILE = 'user_config.json';

/**
 * Load the user configuration from OPFS.
 *
 * Returns `null` if the file does not exist or cannot be read (e.g.
 * first visit, or OPFS is unavailable).
 */
async function loadUserConfig(): Promise<UserConfig | null> {
  try {
    const root = await navigator.storage.getDirectory();
    const fh = await root.getFileHandle(CONFIG_FILE);
    const file = await fh.getFile();
    const text = await file.text();
    return JSON.parse(text) as UserConfig;
  } catch {
    // File missing or OPFS unavailable — treat as first-time user.
    return null;
  }
}

/**
 * Persist the user configuration to OPFS.
 *
 * Creates the file if it does not exist; overwrites it otherwise.
 */
async function saveUserConfig(config: UserConfig): Promise<void> {
  try {
    const root = await navigator.storage.getDirectory();
    const fh = await root.getFileHandle(CONFIG_FILE, { create: true });
    const writable = await fh.createWritable();
    await writable.write(JSON.stringify(config));
    await writable.close();
  } catch (e) {
    console.warn('[NexOS] Failed to save user config:', e);
  }
}

// ---------------------------------------------------------------------------
// SHA-256 hashing (Web Crypto API)
// ---------------------------------------------------------------------------

/**
 * Compute the SHA-256 hash of a UTF-8 string and return it as a lowercase
 * hex string.
 *
 * Uses the browser's native `crypto.subtle` implementation, which is
 * both fast and timing-safe.
 */
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

/** Write a newline followed by a prompt label to the terminal. */
function writePrompt(terminal: Terminal, label: string): void {
  terminal.write(`\r\n${label} `);
}

/**
 * Erase the currently displayed input from the terminal.
 *
 * Moves the cursor back, overwrites with a space, and moves back again
 * for each character in the buffer.
 */
function eraseInput(terminal: Terminal, buffer: string): void {
  for (let i = 0; i < buffer.length; i++) {
    terminal.write('\b \b');
  }
}

// ---------------------------------------------------------------------------
// Auth flow state machine
// ---------------------------------------------------------------------------

/**
 * Possible states of the authentication state machine.
 *
 * - `SETUP_USERNAME` — waiting for the user to type a new username.
 * - `SETUP_PASSWORD` — waiting for the user to choose a password.
 * - `SETUP_CONFIRM`  — waiting for the user to re-type the password.
 * - `LOGIN_PASSWORD` — waiting for a returning user to enter their password.
 */
type AuthMode =
  | 'SETUP_USERNAME'
  | 'SETUP_PASSWORD'
  | 'SETUP_CONFIRM'
  | 'LOGIN_PASSWORD';

/** Result returned by {@link runAuth} on successful authentication. */
export interface AuthResult {
  /** The authenticated (or newly created) username. */
  username: string;
}

// ---------------------------------------------------------------------------
// Main auth entry point
// ---------------------------------------------------------------------------

/**
 * Run the interactive authentication flow.
 *
 * - If no user config exists in OPFS → first-time setup (pick username +
 *   password, confirm password).
 * - If user config exists → password-only login.
 *
 * The function takes ownership of the terminal's `onData` handler for the
 * duration of the flow and restores it (via `handler.dispose()`) before
 * resolving.
 *
 * @param terminal - The xterm.js instance to use for I/O.
 * @returns A promise that resolves with the {@link AuthResult} on success.
 */
export async function runAuth(terminal: Terminal): Promise<AuthResult> {
  const config = await loadUserConfig();

  return new Promise<AuthResult>((resolve) => {
    let mode: AuthMode;
    let inputBuffer = '';
    let setupUsername = '';
    let setupPassword = '';

    if (config) {
      // Returning user — prompt for password only.
      terminal.writeln('');
      terminal.writeln('\x1b[1;36mNexOS\x1b[0m — login required');
      writePrompt(terminal, 'Password:');
      mode = 'LOGIN_PASSWORD';
    } else {
      // First-time user — start the account creation wizard.
      terminal.writeln('');
      terminal.writeln('\x1b[1;36mNexOS\x1b[0m — first-time setup');
      terminal.writeln('Create your account to get started.');
      writePrompt(terminal, 'Username:');
      mode = 'SETUP_USERNAME';
    }

    // Register a temporary onData handler that drives the state machine.
    const handler = terminal.onData((data: string) => {
      const code = data.charCodeAt(0);

      // ---- Ctrl+C: cancel and restart from the beginning ----
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

      // ---- Backspace: delete the last character ----
      if (data === '\x7f' || code === 8) {
        if (inputBuffer.length > 0) {
          inputBuffer = inputBuffer.slice(0, -1);
          // Erase the visual character (password mode still shows *).
          terminal.write('\b \b');
        }
        return;
      }

      // ---- Ignore other control / escape sequences ----
      // \r (Enter) and \n are handled below, so allow them through.
      if (code < 32 && data !== '\r' && data !== '\n') return;

      // ---- Printable character (echo before checking Enter) ----
      if (data !== '\r' && data !== '\n') {
        inputBuffer += data;
        // Echo * for password fields; the character itself for username.
        if (mode === 'SETUP_PASSWORD' || mode === 'SETUP_CONFIRM' || mode === 'LOGIN_PASSWORD') {
          terminal.write('*');
        } else {
          terminal.write(data);
        }
        return;
      }

      // ---- Enter pressed — process the accumulated input ----
      // Strip trailing \r from buffer (defensive; should be empty).
      const value = inputBuffer.replace(/\r$/, '');
      inputBuffer = '';

      switch (mode) {
        // ============================================================
        // SETUP_USERNAME — validate and store the chosen username
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
        // SETUP_PASSWORD — accept and store the chosen password
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
        // SETUP_CONFIRM — verify the password matches
        // ============================================================
        case 'SETUP_CONFIRM': {
          if (value !== setupPassword) {
            terminal.writeln('');
            terminal.writeln('\x1b[31mPasswords do not match. Try again.\x1b[0m');
            // Send the user back to the password step.
            writePrompt(terminal, 'Password:');
            mode = 'SETUP_PASSWORD';
            setupPassword = '';
            break;
          }

          // Passwords match — hash, persist, and resolve.
          sha256(setupPassword).then((hash) => {
            saveUserConfig({ username: setupUsername, passwordHash: hash });
            terminal.writeln('');
            terminal.writeln(
              `\x1b[32mAccount created for \x1b[1m${setupUsername}\x1b[0m`,
            );
            // Release the terminal handler so the main input handler can take over.
            handler.dispose();
            resolve({ username: setupUsername });
          });
          break;
        }

        // ============================================================
        // LOGIN_PASSWORD — verify the entered password against the hash
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
