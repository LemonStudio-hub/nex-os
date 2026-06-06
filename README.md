# web-code

A browser-based Linux-like terminal environment powered by Rust WebAssembly. web-code creates a persistent virtual filesystem stored in the browser's Origin Private File System (OPFS), rendered through a fully interactive terminal built with xterm.js. No backend server required — everything runs client-side.

## Features

### Virtual File System (VFS)

- Tree-structured filesystem rooted at `/`
- Pre-seeded with standard Linux directories (`/home`, `/tmp`, `/etc`, `/var`)
- POSIX-style path resolution: absolute paths, relative paths, `.`, `..`, and `~`
- Full CRUD operations on files and directories
- JSON serialization for persistent storage

### Terminal Emulator

- Full xterm.js terminal with ANSI color support
- Responsive layout that adapts to window resizing (FitAddon + ResizeObserver)
- Command history navigation (Up/Down arrow keys)
- Tab completion for command names
- Keyboard shortcuts: Ctrl+C (cancel), Ctrl+L (clear screen)
- Dark theme with cursor blinking

### Authentication System

- First-time setup: choose a username and password
- Returning login: password-only authentication
- SHA-256 password hashing via the Web Crypto API
- Credentials persisted in OPFS (`user_config.json`)

### Persistence

- VFS state automatically saved to OPFS after every command
- User configuration stored separately in OPFS
- Graceful degradation: if OPFS is unavailable, the system operates in memory-only mode

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Browser                           │
│                                                      │
│  ┌──────────────┐    ┌───────────────────────────┐  │
│  │   xterm.js    │◄──►│   TypeScript Shell Layer  │  │
│  │  (Terminal)   │    │  (Input/Output/Dispatch)  │  │
│  └──────────────┘    └───────────┬───────────────┘  │
│                                  │ wasm-bindgen      │
│                       ┌──────────▼───────────────┐  │
│                       │   Rust WASM Module        │  │
│                       │  ┌─────────────────────┐  │  │
│                       │  │  Command Parser      │  │  │
│                       │  │  (ls, cd, pwd, etc.) │  │  │
│                       │  └─────────┬───────────┘  │  │
│                       │  ┌─────────▼───────────┐  │  │
│                       │  │  Virtual FS (VFS)    │  │  │
│                       │  │  (In-memory tree)    │  │  │
│                       │  └─────────┬───────────┘  │  │
│                       └────────────┼──────────────┘  │
│                                    │                  │
│                       ┌────────────▼──────────────┐  │
│                       │     OPFS (Persistence)     │  │
│                       │  (Browser sandboxed FS)    │  │
│                       └───────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### Data Flow

1. User types a command in xterm.js
2. TypeScript frontend sends the input string to the Rust WASM module via `wasm-bindgen`
3. Rust Shell parses the command, dispatches to the appropriate handler
4. The handler performs operations on the in-memory VFS tree
5. Output string is returned to the frontend and written to the terminal
6. The VFS state is serialized to JSON and saved to OPFS

## Tech Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Terminal Rendering | xterm.js v5 (`@xterm/xterm`) | Browser-based terminal emulator |
| Terminal Addons | `@xterm/addon-fit`, `@xterm/addon-web-links` | Responsive sizing, clickable URLs |
| Core Logic | Rust → WebAssembly | VFS implementation, command parsing and execution |
| JS-WASM Bridge | `wasm-bindgen` | Rust/JavaScript interoperability |
| Browser APIs | `web-sys`, `js-sys` | Access to browser-native APIs |
| Serialization | `serde`, `serde_json` | JSON serialization for VFS state |
| Persistence | OPFS (`navigator.storage.getDirectory()`) | Browser-native persistent storage |
| Build Tool (Rust) | `cargo` + `wasm-bindgen` CLI | Compile Rust to WASM, generate JS bindings |
| Build Tool (Frontend) | Vite 6 + TypeScript | Dev server and production bundling |
| Deployment | Cloudflare Pages + Wrangler CLI | Static site hosting |

## Prerequisites

- **Rust** (1.70+) with the `wasm32-unknown-unknown` target:

  ```bash
  rustup target add wasm32-unknown-unknown
  ```

- **wasm-bindgen CLI** (matching the `wasm-bindgen` crate version):

  ```bash
  cargo install wasm-bindgen-cli
  ```

- **Node.js** (18+) and npm

## Getting Started

### 1. Clone the Repository

```bash
git clone <repository-url>
cd web-code
```

### 2. Build the WASM Module

```bash
# Compile Rust to WebAssembly
cargo build --target wasm32-unknown-unknown --release

# Generate JavaScript bindings
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/web_code.wasm
```

### 3. Install Frontend Dependencies

```bash
cd web
npm install
cd ..
```

### 4. Run the Development Server

```bash
cd web
npm run dev
```

The application will be available at `http://localhost:5173/` (or the next available port).

### 5. Build for Production

```bash
cd web
npm run build
```

The production output is written to `web/dist/`.

## Available Commands

| Command | Description | Usage |
|---------|-------------|-------|
| `ls` | List directory contents | `ls [path]` — `ls -l [path]` for long format |
| `cd` | Change the current directory | `cd [path]` — `cd ~` for home, `cd ..` for parent, `cd /` for root |
| `pwd` | Print the current working directory | `pwd` |
| `mkdir` | Create directories | `mkdir <path>` — `mkdir -p <path>` for recursive creation |
| `touch` | Create empty files | `touch <file> [file2 ...]` |
| `rm` | Remove files or directories | `rm <file>` — `rm -r <dir>` for recursive deletion |
| `cat` | Display file contents | `cat <file> [file2 ...]` |
| `echo` | Display text or write to a file | `echo <text>` — `echo <text> > <file>` to write — `echo <text> >> <file>` to append |
| `cp` | Copy files or directories | `cp <source> <destination>` |
| `mv` | Move or rename files and directories | `mv <source> <destination>` |
| `tree` | Display directory tree structure | `tree [path]` |
| `clear` | Clear the terminal screen | `clear` |
| `help` | Display available commands | `help` |
| `exit` | Exit the terminal session | `exit` |

### Command Chaining

Commands can be chained with `&&`:

```bash
mkdir project && cd project && touch README.md
```

### Output Redirection

Redirect command output to a file:

```bash
echo Hello World > greeting.txt
echo Updated >> greeting.txt
```

## Authentication

### First-Time Visit

When you open web-code for the first time, you will be prompted to create an account:

```
web-code — first-time setup
Create your account to get started.

Username: alice
Password: ********
Confirm password: ********
Account created for alice
```

### Returning Visit

On subsequent visits, only the password is required:

```
web-code — login required

Password: ********
Welcome back, alice!
```

### Security Notes

- Passwords are hashed with SHA-256 using the Web Crypto API before storage
- Only the hash is stored in OPFS — plaintext passwords are never persisted
- Credentials are scoped to the browser origin (same-origin policy)
- Clearing browser data for the site will remove stored credentials

## Project Structure

```
web-code/
├── Cargo.toml                          # Rust project configuration
├── rust-toolchain.toml                 # Rust toolchain (stable + wasm32 target)
├── README.md                           # This file
├── src/
│   ├── lib.rs                          # WASM entry point — exports init, execute_command, etc.
│   ├── vfs.rs                          # Virtual File System — tree data structure + operations
│   ├── shell.rs                        # Shell state — command dispatch, history, prompt
│   └── command/
│       ├── mod.rs                      # Command module declarations
│       ├── ls.rs                       # ls command
│       ├── cd.rs                       # cd command
│       ├── pwd.rs                      # pwd command
│       ├── mkdir.rs                    # mkdir command
│       ├── touch.rs                    # touch command
│       ├── rm.rs                       # rm command
│       ├── cat.rs                      # cat command
│       ├── echo.rs                     # echo command (with redirection)
│       ├── cp.rs                       # cp command
│       ├── mv.rs                       # mv command
│       ├── tree.rs                     # tree command
│       └── help.rs                     # help command
├── pkg/                                # wasm-bindgen output (generated)
│   ├── web_code.js                     # JavaScript bindings
│   ├── web_code.d.ts                   # TypeScript declarations
│   ├── web_code_bg.wasm               # Compiled WebAssembly binary (~160 KB)
│   └── web_code_bg.wasm.d.ts          # WASM TypeScript declarations
└── web/                                # Frontend application
    ├── package.json                    # npm dependencies and scripts
    ├── tsconfig.json                   # TypeScript configuration
    ├── vite.config.ts                  # Vite configuration (WASM plugins)
    ├── index.html                      # Entry HTML page
    ├── style.css                       # Global styles (full-screen terminal)
    ├── auth.ts                         # Authentication flow (setup + login)
    ├── main.ts                         # Application entry (xterm.js + WASM init)
    ├── shell.ts                        # Shell frontend helpers
    └── dist/                           # Vite production build output (generated)
```

## WASM API Reference

The Rust WASM module exposes the following functions via `wasm-bindgen`:

### `init(state_json: string): boolean`

Initialize the VFS and shell with the default username `"user"`. If `state_json` is non-empty, attempts to restore the VFS from the provided JSON string. Returns `true` if the VFS was successfully restored from persisted state.

### `init_with_username(state_json: string, username: string): boolean`

Same as `init`, but allows specifying a custom username. Used after authentication to set the prompt to the logged-in user's name.

### `execute_command(input: string): string`

Parse and execute a shell command. Returns the command output as a string. Special case: the `clear` command returns the ANSI sequence `\x1b[2J\x1b[H` which the frontend interprets as a screen clear.

### `get_prompt(): string`

Return the current shell prompt string, including ANSI color codes. Format: `\x1b[1;32m{user}@{host}:\x1b[1;34m{cwd}\x1b[0m$ `

### `get_completions(partial: string): string[]`

Return an array of command names that start with the given `partial` string. Used for tab completion.

### `get_history(): string[]`

Return the command history as an array of previously entered commands.

### `get_state_json(): string`

Serialize the current VFS state to a JSON string for OPFS persistence. Returns an empty string if the shell has not been initialized.

## Deployment

### Cloudflare Pages

The project is configured for deployment to Cloudflare Pages using the Wrangler CLI.

#### Build and Deploy

```bash
# 1. Build WASM
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/web_code.wasm

# 2. Build frontend
cd web && npm run build && cd ..

# 3. Deploy to Cloudflare Pages
npx wrangler pages deploy web/dist/ --project-name=web-code
```

#### Prerequisites for Deployment

- A Cloudflare account
- Wrangler authenticated via `CLOUDFLARE_API_TOKEN` environment variable or `npx wrangler login`

#### Live URL

The production deployment is available at: **https://851be110.web-code-9x1.pages.dev**

### Other Static Hosts

The production build in `web/dist/` is a standard static site (HTML + JS + CSS + WASM). It can be deployed to any static hosting provider:

- **Vercel**: `vercel web/dist`
- **Netlify**: Drag and drop the `web/dist` folder
- **GitHub Pages**: Copy `web/dist` contents to the `gh-pages` branch
- **Self-hosted**: Serve `web/dist` with any HTTP server (ensure `application/wasm` MIME type is configured for `.wasm` files)

## Browser Compatibility

web-code requires a modern browser with support for:

- **WebAssembly** (all major browsers since 2017)
- **OPFS** (`navigator.storage.getDirectory()`): Chrome 86+, Edge 86+, Firefox 111+, Safari 15.2+
- **Web Crypto API** (`crypto.subtle.digest`): all major browsers
- **ES2020** / top-level `await`

If OPFS is not available, the system falls back to memory-only mode. The VFS will function normally during the session but data will not persist across page reloads.

## License

MIT
