# NexOS

<p align="center">
  <strong>A browser-based Linux terminal environment powered by Rust WebAssembly</strong>
</p>

<p align="center">
  <a href="https://nexos.pages.dev">Live Demo</a> ·
  <a href="docs/ARCHITECTURE.md">Architecture</a> ·
  <a href="docs/COMMANDS.md">Commands</a> ·
  <a href="docs/DEVELOPMENT.md">Development</a> ·
  <a href="docs/API.md">API Reference</a>
</p>

---

NexOS creates a persistent virtual filesystem stored in the browser's Origin Private File System (OPFS), rendered through a fully interactive terminal built with xterm.js. No backend server required — everything runs client-side.

## Highlights

- **35 built-in commands** — `ls`, `cd`, `grep`, `cat`, `sort`, `wc`, `diff`, `find`, and more
- **Pipes and redirection** — `cat file | grep error | wc -l` and `echo hello > file.txt`
- **Persistent filesystem** — your files survive page reloads via browser-native OPFS storage
- **Secure authentication** — SHA-256 hashed passwords, client-side only
- **~245 KB WASM binary** — optimized Rust compiled to WebAssembly
- **Zero backend** — runs entirely in the browser

## Quick Start

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable) with `wasm32-unknown-unknown` target
- [Node.js](https://nodejs.org/) 18+

```bash
# Install Rust WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-bindgen CLI (version must match the crate)
cargo install wasm-bindgen-cli
```

### Build and Run

```bash
# Clone the repository
git clone <repository-url>
cd nexos

# Build the WASM module
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/nexos.wasm

# Start the frontend dev server
cd web && npm install && npm run dev
```

Open `http://localhost:5173/` in your browser.

For detailed setup instructions, see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Features

### Virtual File System

- Tree-structured in-memory filesystem rooted at `/`
- Pre-seeded with standard Linux directories: `/home/user`, `/tmp`, `/etc`, `/var`
- POSIX-style path resolution: absolute paths, relative paths, `.`, `..`, `~`
- Full CRUD operations on files and directories
- JSON serialization for persistent storage to OPFS

### Terminal Emulator

- Full xterm.js terminal with ANSI color support
- Responsive layout that adapts to window resizing
- Command history navigation (Up/Down arrow keys)
- Tab completion for command names
- Keyboard shortcuts: Ctrl+C (cancel), Ctrl+L (clear screen)
- Dark theme with Cascadia Code font

### Shell Engine

- Command parsing with `&&` chaining (stops on first error)
- Pipeline support with quote-aware `|` splitting
- Output redirection (`>` overwrite, `>>` append)
- Extensible command system via Rust `Command` trait and registry

### Authentication

- First-time setup: choose a username and password
- Returning login: password-only authentication
- SHA-256 password hashing via the Web Crypto API
- Credentials persisted in OPFS (`user_config.json`)

### Persistence

- VFS state automatically saved to OPFS after every command
- Restored on page load — your files survive browser sessions
- Graceful fallback to memory-only mode when OPFS is unavailable

## Available Commands

| Category | Commands |
|----------|----------|
| **Filesystem** | `ls` · `cd` · `pwd` · `mkdir` · `touch` · `rm` · `cp` · `mv` · `tree` · `ln` |
| **File Content** | `cat` · `echo` · `head` · `tail` |
| **Text Processing** | `grep` · `sort` · `uniq` · `wc` · `cut` · `tr` · `tee` |
| **Comparison** | `diff` |
| **Search** | `find` · `du` |
| **Permissions** | `chmod` · `chown` |
| **System** | `whoami` · `hostname` · `date` · `history` |
| **Environment** | `env` · `export` |
| **Path Utilities** | `basename` · `dirname` |
| **Documentation** | `man` · `help` |
| **Terminal** | `clear` · `exit` |

See [docs/COMMANDS.md](docs/COMMANDS.md) for detailed usage of each command.

### Examples

```bash
# Navigate and explore
cd ~/projects
mkdir -p src/components
tree .

# File operations
echo "Hello, World!" > greeting.txt
cat greeting.txt
cp greeting.txt backup.txt

# Text processing
cat server.log | grep -i error | sort | uniq -c
cut -f 1,3 -d "," data.csv | head -n 20

# Command chaining
mkdir project && cd project && touch README.md && echo "# My Project" > README.md

# Output redirection
ls -l /home/user > filelist.txt
echo "New entry" >> filelist.txt
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Browser                           │
│  ┌──────────────┐    ┌───────────────────────────┐  │
│  │   xterm.js    │◄──►│   TypeScript Frontend     │  │
│  │  (Terminal)   │    │  (Input/Output/Dispatch)  │  │
│  └──────────────┘    └───────────┬───────────────┘  │
│                                  │ wasm-bindgen      │
│                       ┌──────────▼───────────────┐  │
│                       │   Rust WASM Module        │  │
│                       │  ┌─────────────────────┐  │  │
│                       │  │  Command Registry    │  │  │
│                       │  │  (35 commands)       │  │  │
│                       │  └─────────┬───────────┘  │  │
│                       │  ┌─────────▼───────────┐  │  │
│                       │  │  Virtual FS (VFS)    │  │  │
│                       │  │  (In-memory tree)    │  │  │
│                       │  └─────────┬───────────┘  │  │
│                       └────────────┼──────────────┘  │
│                       ┌────────────▼──────────────┐  │
│                       │     OPFS (Persistence)     │  │
│                       └───────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### Data Flow

1. User types a command in xterm.js
2. TypeScript frontend sends the input to the Rust WASM module via `wasm-bindgen`
3. Rust Shell parses the command (`&&` → `|` → `>` → dispatch)
4. The command handler performs operations on the in-memory VFS tree
5. Output string is returned to the frontend and written to the terminal
6. The VFS state is serialized to JSON and saved to OPFS

For a deep dive, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Tech Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Terminal | xterm.js v5 | Browser-based terminal emulator |
| Core Logic | Rust → WebAssembly | VFS, command parsing, shell engine |
| JS-WASM Bridge | wasm-bindgen | Rust/JavaScript interoperability |
| Serialization | serde + serde_json | JSON for VFS persistence |
| Persistence | OPFS | Browser-native persistent storage |
| Build (Rust) | cargo + wasm-bindgen CLI | Compile Rust to WASM |
| Build (Frontend) | Vite 6 + TypeScript | Dev server and bundling |
| Deployment | Cloudflare Pages | Static site hosting |

## WASM API

The Rust core exports 7 functions:

| Function | Signature | Description |
|----------|-----------|-------------|
| `init` | `(state_json: string) → boolean` | Initialize with default username |
| `init_with_username` | `(state_json, username) → boolean` | Initialize with custom username |
| `execute_command` | `(input: string) → string` | Execute a shell command |
| `get_prompt` | `() → string` | Get the colored prompt string |
| `get_completions` | `(partial: string) → string[]` | Tab completion candidates |
| `get_history` | `() → string[]` | Command history |
| `get_state_json` | `() → string` | Serialize VFS to JSON |

See [docs/API.md](docs/API.md) for full documentation.

## Project Structure

```
nexos/
├── Cargo.toml              # Rust project configuration
├── rust-toolchain.toml     # Rust toolchain (stable + wasm32)
├── src/
│   ├── lib.rs              # WASM entry point (7 exports)
│   ├── vfs/
│   │   ├── mod.rs          # Re-exports
│   │   ├── node.rs         # FsNode, FileNode, DirNode types
│   │   └── tree.rs         # Vfs implementation
│   ├── shell/
│   │   ├── mod.rs          # Shell engine
│   │   ├── pipeline.rs     # Pipe and redirect parsing
│   │   └── dispatch.rs     # Command dispatch
│   └── command/
│       ├── mod.rs          # Command trait + Registry
│       └── *.rs            # 35 command implementations
├── tests/
│   └── shell_tests.rs      # Integration tests (~70 tests)
├── pkg/                    # wasm-bindgen output (generated)
└── web/
    ├── package.json        # Frontend dependencies
    ├── vite.config.ts      # Vite configuration
    ├── index.html          # Entry HTML
    ├── main.ts             # Application bootstrap
    ├── terminal.ts         # Terminal creation
    ├── input.ts            # Input handling
    ├── persistence.ts      # OPFS helpers
    ├── auth.ts             # Authentication
    └── style.css           # Styles
```

## Testing

```bash
# Run all tests (unit + integration)
cargo test

# Run specific test
cargo test test_grep_case_insensitive

# Frontend type checking
cd web && npx tsc --noEmit
```

The test suite includes 70+ integration tests covering all commands, pipes, redirection, chaining, and VFS persistence.

## Deployment

### Cloudflare Pages

```bash
# Build everything
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/nexos.wasm
cd web && npm run build && cd ..

# Deploy
npx wrangler pages deploy web/dist/ --project-name=nexos
```

### Other Platforms

The production build in `web/dist/` is a standard static site. Deploy to:

- **Vercel**: `vercel web/dist`
- **Netlify**: Drag and drop `web/dist`
- **GitHub Pages**: Copy `web/dist` to `gh-pages` branch
- **Self-hosted**: Serve with any HTTP server (configure `application/wasm` MIME type)

See [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) for detailed instructions.

## Browser Compatibility

NexOS requires a modern browser with support for:

| Feature | Minimum Version |
|---------|----------------|
| WebAssembly | All major browsers (2017+) |
| OPFS | Chrome 86+, Edge 86+, Firefox 111+, Safari 15.2+ |
| Web Crypto API | All major browsers |
| ES2020 / top-level await | All modern browsers |

If OPFS is not available, the system falls back to memory-only mode — the VFS functions normally during the session but data does not persist across page reloads.

## Documentation

| Document | Description |
|----------|-------------|
| [README.md](README.md) | This file — project overview and quick start |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | System architecture, data flow, design decisions |
| [docs/COMMANDS.md](docs/COMMANDS.md) | Complete command reference with examples |
| [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) | Development setup, testing, contributing workflow |
| [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) | Production build and deployment instructions |
| [docs/API.md](docs/API.md) | WASM API reference for frontend integration |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution guidelines |
| [CHANGELOG.md](CHANGELOG.md) | Version history |
| [LICENSE](LICENSE) | MIT License |

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

The most common contribution is adding a new command — see [Adding a New Command](docs/DEVELOPMENT.md#adding-a-new-command) for the 3-step process.

## License

This project is licensed under the [MIT License](LICENSE).
