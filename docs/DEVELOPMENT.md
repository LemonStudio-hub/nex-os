# Development Guide

This guide covers everything you need to set up a local development environment, build the project, run tests, and contribute to NexOS.

## Prerequisites

### Required

| Tool | Version | Purpose |
|------|---------|---------|
| **Rust** | stable (1.70+) | Core logic compilation |
| **wasm32-unknown-unknown target** | — | WASM compilation target |
| **wasm-bindgen CLI** | Must match `wasm-bindgen` crate version | JS binding generation |
| **Node.js** | 18+ | Frontend build tooling |
| **npm** | 9+ | Frontend dependency management |

### Optional

| Tool | Purpose |
|------|---------|
| **cargo-watch** | Auto-rebuild on file changes |
| **Clippy** | Rust linter (included with rustup) |
| **rustfmt** | Rust formatter (included with rustup) |

## Initial Setup

### 1. Install Rust and the WASM target

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add the WASM target
rustup target add wasm32-unknown-unknown

# Verify installation
rustc --version
cargo --version
```

### 2. Install wasm-bindgen CLI

The CLI version **must** match the `wasm-bindgen` crate version in `Cargo.toml`. To find the correct version:

```bash
grep 'wasm-bindgen' Cargo.toml
# Example output: wasm-bindgen = "0.2"
```

Install the matching version:

```bash
cargo install wasm-bindgen-cli --version "0.2.x"
```

> **Tip**: The CI pipeline dynamically extracts the version from `Cargo.lock`. You can do the same locally:
> ```bash
> VERSION=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | grep 'version =' | head -1 | sed 's/.*"\(.*\)"/\1/')
> cargo install wasm-bindgen-cli --version "$VERSION"
> ```

### 3. Install frontend dependencies

```bash
cd web
npm install
cd ..
```

## Building

### Full Build (WASM + Frontend)

```bash
# Step 1: Compile Rust to WASM
cargo build --target wasm32-unknown-unknown --release

# Step 2: Generate JavaScript bindings
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/nexos.wasm

# Step 3: Build frontend
cd web && npm run build && cd ..
```

The production output is in `web/dist/`.

### Development Server

For active development with hot-reload:

```bash
# Terminal 1: Build WASM (rebuild after Rust changes)
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/nexos.wasm

# Terminal 2: Start Vite dev server
cd web && npm run dev
```

The dev server runs at `http://localhost:5173/` with hot module replacement. After changing Rust code, you must rebuild the WASM module and refresh the browser.

### Using cargo-watch (Optional)

Automatically rebuild WASM on Rust file changes:

```bash
cargo install cargo-watch
cargo watch -s 'cargo build --target wasm32-unknown-unknown --release && wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/nexos.wasm'
```

## Project Layout

```
nexos/
├── Cargo.toml              # Rust project configuration
├── Cargo.lock              # Rust dependency lock file
├── rust-toolchain.toml     # Pins Rust stable + wasm32 target
├── src/
│   ├── lib.rs              # WASM entry point (7 exported functions)
│   ├── vfs/
│   │   ├── mod.rs          # Re-exports
│   │   ├── node.rs         # FsNode, FileNode, DirNode types
│   │   └── tree.rs         # Vfs implementation (path resolution, CRUD, JSON)
│   ├── shell/
│   │   ├── mod.rs          # Shell struct, execute(), run_pipeline()
│   │   ├── pipeline.rs     # Pipe splitting, redirect extraction
│   │   └── dispatch.rs     # Command dispatch via Registry
│   └── command/
│       ├── mod.rs          # Command trait, CommandContext, Registry, register_all()
│       ├── ls.rs           # ls command
│       ├── cat.rs          # cat command
│       ├── ...             # One file per command (35 total)
│       └── exit.rs         # exit command
├── tests/
│   └── shell_tests.rs      # Integration tests (~70 tests)
├── pkg/                    # wasm-bindgen output (generated, git-ignored)
├── web/
│   ├── package.json        # Frontend dependencies and scripts
│   ├── tsconfig.json       # TypeScript configuration
│   ├── vite.config.ts      # Vite configuration (WASM plugins)
│   ├── index.html          # Entry HTML
│   ├── style.css           # Global styles
│   ├── main.ts             # Application bootstrap
│   ├── terminal.ts         # Terminal creation and resize
│   ├── input.ts            # Keyboard input handler
│   ├── persistence.ts      # OPFS load/save helpers
│   ├── auth.ts             # Authentication flow
│   └── dist/               # Production build output (generated)
└── .github/
    └── workflows/
        └── ci.yml          # CI/CD pipeline
```

## Testing

### Running All Tests

```bash
cargo test
```

This runs both unit tests (in individual source files) and integration tests (in `tests/shell_tests.rs`).

### Running Specific Tests

```bash
# By test name
cargo test test_grep_case_insensitive

# By module
cargo test vfs::

# With output
cargo test -- --nocapture

# List available tests
cargo test -- --list
```

### Test Organization

**Unit tests** are colocated with the code they test, inside `#[cfg(test)]` modules:

```
src/vfs/tree.rs      — Vfs path resolution, CRUD, JSON roundtrip
src/command/ls.rs    — ls-specific tests
src/command/grep.rs  — grep-specific tests
...
```

**Integration tests** in `tests/shell_tests.rs` test commands through the full shell pipeline:

```rust
#[test]
fn test_grep_case_insensitive() {
    let mut shell = Shell::new(Vfs::new());
    shell.execute("echo Hello > file.txt");
    let output = shell.execute("grep -i hello file.txt");
    assert!(output.contains("Hello"));
}
```

### Writing New Tests

When adding a command, include both:

1. **Unit tests** in the command file for internal logic
2. **Integration tests** in `shell_tests.rs` for end-to-end behavior

Example integration test structure:

```rust
#[test]
fn test_mycommand_basic() {
    let mut shell = Shell::new(Vfs::new());
    // Setup
    shell.execute("echo test data > input.txt");
    // Execute
    let output = shell.execute("mycommand input.txt");
    // Verify
    assert!(output.contains("expected output"));
}

#[test]
fn test_mycommand_error() {
    let mut shell = Shell::new(Vfs::new());
    let output = shell.execute("mycommand nonexistent");
    assert!(output.contains("No such file"));
}
```

### Frontend Type Checking

```bash
cd web
npx tsc --noEmit
```

## Code Quality

### Rust Linting

```bash
# Format code
cargo fmt

# Check formatting (CI runs this)
cargo fmt --check

# Run clippy lints (CI treats warnings as errors)
cargo clippy -- -D warnings
```

### Common Clippy Issues

- Unused variables or imports
- Unnecessary clones
- Missing `#[must_use]` annotations
- Inefficient string operations

## Adding a New Command

This is the most common contribution. Here's the complete workflow:

### Step 1: Create the command file

Create `src/command/mycommand.rs`:

```rust
use super::{Command, CommandContext};

pub struct MyCommand;

impl Command for MyCommand {
    fn name(&self) -> &'static str {
        "mycommand"
    }

    fn description(&self) -> &'static str {
        "Brief description of what the command does"
    }

    fn accepts_stdin(&self) -> bool {
        false
    }

    fn execute(&self, ctx: &mut CommandContext) -> Result<String, String> {
        // Implementation here
        Ok("output\n".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vfs::Vfs;
    use std::collections::HashMap;

    fn make_ctx<'a>(
        vfs: &'a mut Vfs,
        env: &'a mut HashMap<String, String>,
        args: &'a [&'a str],
    ) -> CommandContext<'a> {
        CommandContext {
            vfs,
            stdin: "",
            args,
            username: "user",
            hostname: "nexos",
            history: &[],
            env_vars: env,
        }
    }

    #[test]
    fn test_mycommand_basic() {
        let mut vfs = Vfs::new();
        let mut env = HashMap::new();
        let ctx = make_ctx(&mut vfs, &mut env, &[]);
        let cmd = MyCommand;
        assert!(cmd.execute(&mut ctx).is_ok());
    }
}
```

### Step 2: Register the command

In `src/command/mod.rs`:

```rust
// Add module declaration
pub mod mycommand;

// Add to register_all()
fn register_all(commands: &mut Vec<Box<dyn Command>>) {
    // ... existing commands ...
    commands.push(Box::new(mycommand::MyCommand));
}
```

### Step 3: Add integration tests

In `tests/shell_tests.rs`:

```rust
#[test]
fn test_mycommand() {
    let mut shell = Shell::new(Vfs::new());
    let output = shell.execute("mycommand");
    assert!(!output.is_empty());
}
```

### Step 4: Run tests and lint

```bash
cargo test mycommand
cargo clippy -- -D warnings
cargo fmt
```

### Step 5: (Optional) Add a man page

Edit `src/command/man.rs` to add a manual page entry for your command.

## Debugging

### Rust Debugging

For WASM debugging, use `console_log` via `web-sys`:

```rust
// In Cargo.toml, ensure "console" feature is enabled for web-sys
// Then in your code:
web_sys::console::log_1(&"Debug message".into());
```

### Browser DevTools

1. Open DevTools (F12)
2. **Console tab**: See `console.log` output and errors
3. **Network tab**: Verify WASM loading
4. **Application tab → Storage → OPFS**: Inspect persisted VFS state and user config
5. **Sources tab**: Set breakpoints in TypeScript code

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| `wasm-bindgen` version mismatch | CLI version doesn't match crate | Reinstall CLI with matching version |
| WASM module fails to load | Missing build step | Run `cargo build` + `wasm-bindgen` |
| TypeScript errors after Rust changes | Stale type declarations | Rebuild WASM module |
| OPFS not available | Unsupported browser or insecure context | Use HTTPS or Chrome 86+ |

## CI/CD Pipeline

The GitHub Actions pipeline (`.github/workflows/ci.yml`) runs on every push and PR to `main`:

### Job 1: Rust Tests & WASM Build
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test --verbose`
- `cargo build --target wasm32-unknown-unknown --release`
- `wasm-bindgen --target web --out-dir pkg`
- Upload `pkg/` as artifact

### Job 2: Frontend Build (depends on Job 1)
- `npm ci`
- `npx tsc --noEmit`
- `npm run build`
- Upload `web/dist/` as artifact

### Job 3: Deploy (depends on Jobs 1 & 2, main branch only)
- Rebuilds everything from scratch
- Deploys to Cloudflare Pages via Wrangler

## Release Process

1. Update `CHANGELOG.md` with the new version
2. Update `version` in `Cargo.toml` and `web/package.json`
3. Create a git tag: `git tag v0.2.0`
4. Push the tag: `git push origin v0.2.0`
5. The CI pipeline will build and deploy automatically

## Useful Commands Reference

```bash
# Build
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/nexos.wasm

# Test
cargo test
cargo test -- --nocapture
cargo test test_name

# Lint
cargo fmt
cargo fmt --check
cargo clippy -- -D warnings

# Frontend
cd web && npm run dev          # Dev server
cd web && npm run build        # Production build
cd web && npx tsc --noEmit     # Type check

# Deploy
npx wrangler pages deploy web/dist/ --project-name=nexos
```
