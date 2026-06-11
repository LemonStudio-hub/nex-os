# Architecture

This document describes the internal architecture of NexOS, covering the Rust core, WASM bridge, TypeScript frontend, and how they interact.

## System Overview

NexOS is a browser-based Linux terminal emulator with no backend. All computation happens client-side:

- **Rust** handles the core logic: a virtual filesystem (VFS), a shell engine, and 35 built-in commands
- **WebAssembly** bridges the Rust core to the browser via `wasm-bindgen`
- **TypeScript** manages the terminal UI, keyboard input, authentication, and persistence

```
┌─────────────────────────────────────────────────────────────────┐
│                          Browser                                 │
│                                                                  │
│  ┌──────────────────┐      ┌──────────────────────────────────┐ │
│  │     xterm.js      │◄────►│      TypeScript Frontend         │ │
│  │   (Terminal UI)   │      │  main.ts  input.ts  auth.ts     │ │
│  └──────────────────┘      │  terminal.ts  persistence.ts     │ │
│                             └──────────────┬───────────────────┘ │
│                                            │ wasm-bindgen FFI    │
│                             ┌──────────────▼───────────────────┐ │
│                             │      Rust WASM Module (core)      │ │
│                             │  ┌────────────────────────────┐  │ │
│                             │  │  lib.rs — WASM entry point  │  │ │
│                             │  │  (7 exported functions)     │  │ │
│                             │  └─────────┬──────────────────┘  │ │
│                             │            │                      │ │
│                             │  ┌─────────▼──────────────────┐  │ │
│                             │  │  Shell Engine               │  │ │
│                             │  │  shell/mod.rs   — execute() │  │ │
│                             │  │  shell/pipeline.rs — pipes  │  │ │
│                             │  │  shell/dispatch.rs — dispatch│ │ │
│                             │  └─────────┬──────────────────┘  │ │
│                             │            │                      │ │
│                             │  ┌─────────▼──────────────────┐  │ │
│                             │  │  Command Registry            │  │ │
│                             │  │  Command trait + 35 commands │  │ │
│                             │  └─────────┬──────────────────┘  │ │
│                             │            │                      │ │
│                             │  ┌─────────▼──────────────────┐  │ │
│                             │  │  Virtual File System (VFS)   │  │ │
│                             │  │  In-memory tree + JSON I/O   │  │ │
│                             │  └────────────────────────────┘  │ │
│                             └──────────────────────────────────┘ │
│                                            │                      │
│                             ┌──────────────▼───────────────────┐ │
│                             │         OPFS (Persistence)        │ │
│                             │  vfs_state.json  user_config.json │ │
│                             └──────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Data Flow

### Command Execution

```
User Input (keyboard)
    │
    ▼
xterm.js onData handler (web/input.ts)
    │
    ▼
wasm.execute_command(input)  ──── FFI boundary ────
    │
    ▼
Shell::execute() (src/shell/mod.rs)
    ├── Split by "&&" (sequential chain, stop on first error)
    │   └── For each segment:
    │       ├── split_pipe_stages() — split by "|" respecting quotes
    │       ├── extract_redirect()  — extract ">" / ">>" targets
    │       └── run_pipeline()      — execute stages, pass stdout→stdin
    │           └── execute_with_stdin() (src/shell/dispatch.rs)
    │               ├── Tokenize input (split_whitespace)
    │               ├── Registry::get(cmd_name) — lookup command
    │               ├── Build CommandContext (vfs, stdin, args, env, ...)
    │               └── command.execute(&mut ctx) → Result<String, String>
    │
    ▼
Output string returned to TypeScript
    │
    ▼
terminal.write(output)  ──── display in xterm.js
    │
    ▼
wasm.get_state_json() → saveToOPFS()  ──── persist VFS
```

### Pipe Mechanism

When a pipeline like `cat file | grep pattern | wc -l` is executed:

1. **Stage 1** (`cat file`): Executes normally, returns file contents as stdout
2. **Stage 2** (`grep pattern`): Receives stage 1's stdout as `stdin` in `CommandContext`. If the command's `accepts_stdin()` returns `true`, the dispatch layer writes stdin to `/tmp/.pipe_input` and appends that path to the args
3. **Stage 3** (`wc -l`): Receives stage 2's stdout as stdin

Commands that declare `accepts_stdin() == true` (like `grep`, `sort`, `wc`, `head`, `tail`, `cat`, `uniq`, `cut`) automatically receive piped input as a file argument. Commands like `tr` and `tee` read `ctx.stdin` directly and return `false` from `accepts_stdin()`.

### Redirection

Redirection (`>` / `>>`) is only applied to the **last stage** of a pipeline:

```bash
echo hello > file.txt       # last (and only) stage redirects
cat file | grep err > log   # last stage (grep) redirects
```

The redirect is extracted by `extract_redirect()` before the command is dispatched. When a redirect target exists, the shell writes the command's output to the VFS file instead of returning it to the terminal.

## Rust Core (`src/`)

### Module Layout

```
src/
├── lib.rs                  # WASM entry point
├── vfs/
│   ├── mod.rs              # Re-exports: Vfs, FsNode, FileNode, DirNode
│   ├── node.rs             # Type definitions
│   └── tree.rs             # Vfs implementation
├── shell/
│   ├── mod.rs              # Shell struct, execute(), run_pipeline()
│   ├── pipeline.rs         # split_pipe_stages(), extract_redirect()
│   └── dispatch.rs         # execute_with_stdin()
└── command/
    ├── mod.rs              # Command trait, CommandContext, Registry
    └── *.rs                # 35 command implementations
```

### Virtual File System (`vfs/`)

The VFS is an in-memory tree structure with two node types:

```rust
enum FsNode {
    File(FileNode),       // name + content (String)
    Directory(DirNode),   // name + children (HashMap<String, FsNode>)
}
```

The `Vfs` struct holds:
- `root: DirNode` — the root directory
- `cwd: String` — current working directory as an absolute path

Key features:
- **Path resolution**: Handles absolute paths, relative paths, `.`, `..`, and `~` (mapped to `/home/user`)
- **CRUD operations**: `mkdir`, `touch`, `rm`, `read_file`, `write_file`, `cp`, `mv`, `list_dir`
- **Serialization**: `to_json()` / `from_json()` for OPFS persistence via serde

Default directory structure:
```
/
├── home/
│   └── user/          # Home directory
├── tmp/               # Temporary files
├── etc/               # Configuration
└── var/               # Variable data
```

### Shell Engine (`shell/`)

The `Shell` struct holds all runtime state:

```rust
struct Shell {
    vfs: Vfs,
    username: String,
    hostname: String,
    history: Vec<String>,
    env_vars: HashMap<String, String>,
    registry: Registry,
}
```

**Execution pipeline** (`Shell::execute()`):
1. Trim input, add to history
2. Update `PWD` env var
3. Split by `&&` — execute segments sequentially, stop on first error
4. For each segment, split by `|` into pipe stages
5. For each stage, extract `>` / `>>` redirection
6. Run the pipeline: each stage's stdout becomes the next stage's stdin
7. Only the last stage applies redirection

**Default environment variables**:
| Variable | Value |
|----------|-------|
| `USER` | `user` (or authenticated username) |
| `HOSTNAME` | `nexos` |
| `HOME` | `/home/user` |
| `SHELL` | `/bin/nexsh` |
| `PATH` | `/usr/bin:/bin` |
| `PWD` | Current working directory |
| `TERM` | `xterm-256color` |

### Command System (`command/`)

Commands implement the `Command` trait:

```rust
trait Command {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn accepts_stdin(&self) -> bool { false }
    fn execute(&self, ctx: &mut CommandContext) -> Result<String, String>;
}
```

The `CommandContext` bundles everything a command might need:

```rust
struct CommandContext<'a> {
    vfs: &'a mut Vfs,
    stdin: &'a str,
    args: &'a [&'a str],
    username: &'a str,
    hostname: &'a str,
    history: &'a [String],
    env_vars: &'a mut HashMap<String, String>,
}
```

Commands are registered in a `Registry` (a `Vec<Box<dyn Command>>`) built once at shell initialization. Dispatch is a simple name lookup — no match statement needed.

**Return conventions**:
- `Ok(String)` — success output displayed to the user
- `Err(String)` — error message; in a `&&` chain, this stops execution

### Pipeline Parsing (`shell/pipeline.rs`)

**`split_pipe_stages(input)`**: Splits a command string by top-level `|` tokens while respecting single and double quotes. Example:

```
"cat 'file | name' | grep hello"
→ ["cat 'file | name'", "grep hello"]
```

**`extract_redirect(cmd)`**: Extracts `>` (overwrite) or `>>` (append) redirection from a command string. Handles both spaced (`cmd > file`) and unspaced (`cmd>file`) forms. Returns `(command_part, Option<(target, is_append)>)`.

## TypeScript Frontend (`web/`)

### Module Responsibilities

| Module | Responsibility |
|--------|---------------|
| `main.ts` | Application bootstrap, WASM loading, initialization sequence |
| `terminal.ts` | xterm.js terminal creation, theme, addons, resize handling |
| `input.ts` | Keyboard input handler, history navigation, tab completion, command execution |
| `persistence.ts` | OPFS read/write helpers for VFS state |
| `auth.ts` | First-time setup and returning login authentication flow |
| `style.css` | Full-screen terminal layout, dark theme |

### Boot Sequence

```
1. Create xterm.js terminal, mount to DOM
2. Load WASM module (dynamic import of pkg/nexos)
3. Run authentication flow (setup or login)
4. Load saved VFS state from OPFS
5. Initialize WASM shell (wasm.init_with_username)
6. Display prompt, hand off to input handler
```

### Authentication Flow

```
                    ┌─────────────────┐
                    │  Load user_config │
                    │  from OPFS        │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │  Config exists?   │
                    └────────┬────────┘
                     No │         │ Yes
                ┌───────▼──┐  ┌──▼──────────┐
                │  SETUP    │  │  LOGIN       │
                │  Username │  │  Password    │
                └───────┬──┘  └──┬───────────┘
                ┌───────▼──┐     │
                │  Password │     │
                └───────┬──┘     │
                ┌───────▼──┐     │
                │  Confirm  │     │
                └───────┬──┘     │
                        │        │
                ┌───────▼────────▼──────┐
                │  Hash with SHA-256     │
                │  Save to OPFS          │
                │  Return username       │
                └───────────────────────┘
```

### Input Handling

The input handler (`web/input.ts`) manages:
- **Character input**: Appends to buffer, echoes to terminal
- **Enter**: Executes command via `wasm.execute_command()`, saves VFS state
- **Backspace**: Removes last character from buffer and display
- **Up/Down arrows**: Navigate command history
- **Tab**: Triggers `wasm.get_completions()`, inserts common prefix or cycles through matches
- **Ctrl+C**: Cancels current input, writes `^C`, shows new prompt
- **Ctrl+L**: Clears terminal, redraws prompt

### Persistence

Two files are stored in OPFS:
- `vfs_state.json` — Serialized VFS tree (saved after every command)
- `user_config.json` — `{username, passwordHash}` (saved on account creation)

OPFS access uses `navigator.storage.getDirectory()` with graceful fallback. If OPFS is unavailable, the system operates in memory-only mode.

## WASM Bridge

The Rust WASM module exports 7 functions via `wasm-bindgen`:

| Function | Signature | Purpose |
|----------|-----------|---------|
| `init` | `(state_json: string) → boolean` | Initialize with default username |
| `init_with_username` | `(state_json: string, username: string) → boolean` | Initialize with custom username |
| `execute_command` | `(input: string) → string` | Execute a shell command |
| `get_prompt` | `() → string` | Get the colored prompt string |
| `get_completions` | `(partial: string) → string[]` | Get tab completion candidates |
| `get_history` | `() → string[]` | Get command history |
| `get_state_json` | `() → string` | Serialize VFS to JSON |

The global shell state is held in a `thread_local! { RefCell<Option<Shell>> }` — a single mutable instance accessible from all exported functions.

The `wasm-bindgen --target web` flag generates ES module-compatible output. The frontend imports it as:
```typescript
const mod = await import('../pkg/nexos');
await mod.default(); // Initialize WASM memory
```

## Build Pipeline

```
Rust Source (src/**/*.rs)
    │
    ▼ cargo build --target wasm32-unknown-unknown --release
WASM Binary (target/.../nexos.wasm)
    │
    ▼ wasm-bindgen --target web --out-dir pkg
JS Bindings (pkg/nexos.js, pkg/nexos.d.ts, pkg/nexos_bg.wasm)
    │
    ▼ Vite bundles with TypeScript frontend
Production Build (web/dist/)
    │
    ▼ Deploy to Cloudflare Pages
https://nexos.pages.dev
```

### Release Optimizations

The Cargo release profile is tuned for minimal WASM binary size:

```toml
[profile.release]
opt-level = "z"    # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Single codegen unit (better optimization)
strip = true        # Strip debug symbols
```

This produces a ~245 KB WASM binary (before gzip compression).

## Design Decisions

### Why Rust + WASM?

- **Performance**: VFS operations and text processing run at near-native speed
- **Safety**: Rust's type system prevents common bugs (null pointers, data races)
- **Size**: Optimized Rust WASM is compact (~245 KB)
- **No runtime**: Unlike JavaScript frameworks, Rust compiles to standalone WASM with no runtime overhead

### Why an In-Memory VFS?

- **Simplicity**: No need for a real filesystem implementation or database
- **Speed**: All operations are in-memory HashMap lookups and string manipulations
- **Serialization**: serde makes JSON roundtrips trivial
- **Portability**: Works in any browser with WASM support

### Why OPFS for Persistence?

- **Native**: Browser-native API, no third-party libraries
- **Sandboxed**: Data is scoped to the origin
- **Fast**: Synchronous-like API with async handles
- **Large storage**: Much larger limits than localStorage

### Why a Command Trait + Registry?

- **Extensibility**: Adding a command requires only 3 steps (create file, add mod, register)
- **Decoupling**: Commands don't know about each other or the shell internals
- **Metadata**: `name()`, `description()`, `accepts_stdin()` are declarative
- **Testability**: Each command can be tested in isolation with a mock `CommandContext`
