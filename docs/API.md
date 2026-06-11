# WASM API Reference

This document describes the WebAssembly API exported by the NexOS Rust core. These functions are the interface between the TypeScript frontend and the Rust shell engine.

## Overview

The WASM module exports 7 functions via `wasm-bindgen`. The global shell state is held in a `thread_local! { RefCell<Option<Shell>> }` — a single mutable instance accessible from all exported functions.

### Initialization

Before calling any other function, you must initialize the module:

```javascript
// Import and initialize the WASM module
const wasm = await import('./pkg/nexos.js');
await wasm.default(); // Initialize WASM memory

// Then initialize the shell
const restored = wasm.init_with_username(savedState, username);
```

The `default()` export initializes the WASM memory and must be called once before using any other function.

---

## Functions

### `init`

```typescript
init(state_json: string): boolean
```

Initialize the VFS and shell with the default username `"user"`.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `state_json` | `string` | Previously saved VFS state as JSON. Pass `""` for a fresh VFS. |

**Returns:** `boolean` — `true` if the VFS was successfully restored from the provided JSON, `false` if a fresh VFS was created.

**Behavior:**
- If `state_json` is non-empty, attempts to parse it as a VFS JSON string
- If parsing fails, falls back to a fresh default VFS
- Sets the username to `"user"`
- Creates the shell with default environment variables

**Example:**
```javascript
const restored = wasm.init(savedVfsState);
if (restored) {
    console.log('VFS restored from saved state');
} else {
    console.log('Fresh VFS initialized');
}
```

---

### `init_with_username`

```typescript
init_with_username(state_json: string, username: string): boolean
```

Initialize the VFS and shell with a custom username. Used after authentication.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `state_json` | `string` | Previously saved VFS state as JSON. Pass `""` for a fresh VFS. |
| `username` | `string` | The authenticated username (e.g., `"alice"`). |

**Returns:** `boolean` — `true` if restored from persisted state, `false` if fresh.

**Behavior:**
- Same as `init()`, but sets the shell username to the provided value
- The username appears in the prompt: `alice@nexos:/home/user$ `
- Also sets the `USER` environment variable

**Example:**
```javascript
const { username } = await runAuth(terminal);
const savedState = await loadFromOPFS();
const restored = wasm.init_with_username(savedState, username);
```

---

### `execute_command`

```typescript
execute_command(input: string): string
```

Parse and execute a shell command.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `input` | `string` | The complete command string as typed by the user. |

**Returns:** `string` — The command output text. Empty string for commands with no output.

**Behavior:**
- Parses the input for `&&` chaining, `|` pipes, and `>` / `>>` redirection
- Dispatches to the appropriate command handler via the Registry
- Adds the input to command history
- Updates the `PWD` environment variable
- Special case: `clear` returns `\x1b[2J\x1b[H` (ANSI clear screen sequence)
- Errors from `&&` chains stop execution and return the error message

**Examples:**
```javascript
// Simple command
const output = wasm.execute_command('ls -l');

// Pipe chain
const result = wasm.execute_command('cat file.txt | grep error | wc -l');

// Command chaining
const result = wasm.execute_command('mkdir project && cd project && touch README.md');

// Redirection (no terminal output)
wasm.execute_command('echo Hello World > greeting.txt');
// Returns "" — content was written to the VFS file

// Clear screen
const clear = wasm.execute_command('clear');
// Returns "\x1b[2J\x1b[H"
```

---

### `get_prompt`

```typescript
get_prompt(): string
```

Return the current shell prompt string with ANSI color codes.

**Parameters:** None

**Returns:** `string` — The formatted prompt string.

**Format:**
```
\x1b[1;32m{username}@{hostname}:\x1b[1;34m{cwd}\x1b[0m$
```

**ANSI codes:**
- `\x1b[1;32m` — Bold green (for `user@host`)
- `\x1b[1;34m` — Bold blue (for working directory)
- `\x1b[0m` — Reset

**Example:**
```javascript
const prompt = wasm.get_prompt();
terminal.write(prompt);
// Displays: user@nexos:/home/user$  (in color)
```

---

### `get_completions`

```typescript
get_completions(partial: string): string[]
```

Return tab completion candidates matching a prefix.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `partial` | `string` | The partial command name to complete. |

**Returns:** `string[]` — Array of matching command names. Empty array if no matches.

**Behavior:**
- Matches against registered command names
- Returns all commands that start with the given prefix
- Used for tab completion in the input handler

**Examples:**
```javascript
wasm.get_completions('gr');
// → ["grep"]

wasm.get_completions('c');
// → ["cat", "cd", "chmod", "chown", "clear", "cp", "cut"]

wasm.get_completions('xyz');
// → []

wasm.get_completions('ls');
// → ["ls"]  (exact match)
```

---

### `get_history`

```typescript
get_history(): string[]
```

Return the command history as an array of previously entered commands.

**Parameters:** None

**Returns:** `string[]` — Array of command strings in chronological order. Empty array if no history.

**Behavior:**
- Returns all commands entered since shell initialization
- Commands are stored in order of execution
- History is not persisted across sessions (not saved to OPFS)

**Example:**
```javascript
wasm.execute_command('ls');
wasm.execute_command('cd /tmp');
wasm.execute_command('pwd');

wasm.get_history();
// → ["ls", "cd /tmp", "pwd"]
```

---

### `get_state_json`

```typescript
get_state_json(): string
```

Serialize the current VFS state to a JSON string for persistence.

**Parameters:** None

**Returns:** `string` — JSON string representing the complete VFS state. Empty string if the shell is not initialized.

**Behavior:**
- Serializes the entire VFS tree (all files and directories)
- Includes the current working directory (`cwd`)
- Uses serde for JSON serialization
- The resulting JSON can be passed back to `init()` or `init_with_username()` to restore the state

**JSON structure:**
```json
{
  "root": {
    "name": "",
    "children": {
      "home": {
        "Directory": {
          "name": "home",
          "children": {
            "user": {
              "Directory": {
                "name": "user",
                "children": {
                  "greeting.txt": {
                    "File": {
                      "name": "greeting.txt",
                      "content": "Hello World\n"
                    }
                  }
                }
              }
            }
          }
        }
      },
      "tmp": { "Directory": { "name": "tmp", "children": {} } },
      "etc": { "Directory": { "name": "etc", "children": {} } },
      "var": { "Directory": { "name": "var", "children": {} } }
    }
  },
  "cwd": "/home/user"
}
```

**Example:**
```javascript
// Save after each command
const output = wasm.execute_command(input);
const state = wasm.get_state_json();
await saveToOPFS(state);

// Restore on next visit
const savedState = await loadFromOPFS();
wasm.init_with_username(savedState, username);
```

---

## TypeScript Type Declarations

The `wasm-bindgen` CLI generates TypeScript declarations in `pkg/nexos.d.ts`:

```typescript
/**
 * Initialize the VFS and shell with the default username "user".
 * Returns true if restored from persisted state.
 */
export function init(state_json: string): boolean;

/**
 * Initialize with a custom username. Returns true if restored.
 */
export function init_with_username(state_json: string, username: string): boolean;

/**
 * Execute a shell command and return the output.
 */
export function execute_command(input: string): string;

/**
 * Get the current prompt string with ANSI color codes.
 */
export function get_prompt(): string;

/**
 * Get tab completion candidates for a partial input.
 */
export function get_completions(partial: string): string[];

/**
 * Get the command history.
 */
export function get_history(): string[];

/**
 * Serialize the VFS state to JSON.
 */
export function get_state_json(): string;
```

## Integration Example

Complete example showing the full lifecycle:

```typescript
import type { Terminal } from '@xterm/xterm';

interface WasmApi {
    default(): Promise<void>;
    init(state_json: string): boolean;
    init_with_username(state_json: string, username: string): boolean;
    execute_command(input: string): string;
    get_prompt(): string;
    get_completions(partial: string): string[];
    get_history(): string[];
    get_state_json(): string;
}

async function bootstrap(terminal: Terminal) {
    // 1. Load and initialize WASM
    const wasm = await import('../pkg/nexos') as unknown as WasmApi;
    await wasm.default();

    // 2. Authenticate user
    const username = await authenticateUser(terminal);

    // 3. Load saved state from OPFS
    const savedState = await loadFromOPFS();

    // 4. Initialize shell
    const restored = wasm.init_with_username(savedState, username);
    terminal.writeln(restored
        ? '\x1b[36m[NexOS] VFS restored\x1b[0m'
        : '\x1b[36m[NexOS] Fresh VFS initialized\x1b[0m'
    );

    // 5. Show prompt and handle input
    let prompt = wasm.get_prompt();
    terminal.write(prompt);

    terminal.onData(async (data: string) => {
        if (data === '\r') { // Enter
            const input = getCurrentInput();
            const output = wasm.execute_command(input);

            if (output) {
                terminal.writeln('');
                terminal.write(output);
            }

            // Persist state
            const state = wasm.get_state_json();
            await saveToOPFS(state);

            // Show new prompt
            prompt = wasm.get_prompt();
            terminal.write(prompt);
        }
        // ... handle other keys
    });
}
```

## Error Handling

All functions are designed to fail gracefully:

| Function | Failure behavior |
|----------|-----------------|
| `init` / `init_with_username` | Returns `false` on invalid JSON, creates fresh VFS |
| `execute_command` | Returns error string (e.g., `"command not found: foo\n"`) |
| `get_prompt` | Returns `"$ "` if shell not initialized |
| `get_completions` | Returns `[]` if shell not initialized |
| `get_history` | Returns `[]` if shell not initialized |
| `get_state_json` | Returns `""` if shell not initialized |

Commands return errors via `Result<String, String>`:
- `Ok(output)` — success, output displayed to user
- `Err(message)` — error, message displayed in `&&` chains this stops execution

## Performance Notes

- **WASM binary size**: ~245 KB (release build with `opt-level = "z"`, LTO, stripped)
- **Initialization**: Near-instant (< 10ms on modern hardware)
- **Command execution**: Microsecond-level for simple commands, millisecond-level for complex operations (large file searches, deep directory trees)
- **Serialization**: `get_state_json()` scales linearly with VFS size
- **Memory**: The VFS is entirely in-memory; large file trees will increase memory usage proportionally
