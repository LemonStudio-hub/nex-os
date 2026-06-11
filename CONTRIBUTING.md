# Contributing to NexOS

Thank you for your interest in contributing to NexOS! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Making Changes](#making-changes)
  - [Adding a New Command](#adding-a-new-command)
  - [Modifying Existing Commands](#modifying-existing-commands)
  - [Frontend Changes](#frontend-changes)
- [Testing](#testing)
- [Code Style](#code-style)
- [Commit Messages](#commit-messages)
- [Pull Requests](#pull-requests)
- [Reporting Issues](#reporting-issues)

## Code of Conduct

Be respectful, constructive, and inclusive. We are committed to providing a welcoming and inspiring community for everyone.

## Getting Started

1. **Fork** the repository on GitHub
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/<your-username>/nexos.git
   cd nexos
   ```
3. **Create a branch** for your changes:
   ```bash
   git checkout -b feature/my-feature
   ```

## Development Setup

### Prerequisites

- **Rust** (stable) with `wasm32-unknown-unknown` target:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- **wasm-bindgen CLI** (version must match the `wasm-bindgen` crate in `Cargo.toml`):
  ```bash
  cargo install wasm-bindgen-cli
  ```
- **Node.js** (18+) and npm

### Build and Run

```bash
# Build WASM module
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/nexos.wasm

# Start frontend dev server
cd web && npm install && npm run dev
```

The application will be available at `http://localhost:5173/`.

For detailed instructions, see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Project Structure

```
src/
├── lib.rs              # WASM entry point
├── vfs/
│   ├── mod.rs          # Re-exports
│   ├── node.rs         # FsNode, FileNode, DirNode types
│   └── tree.rs         # Vfs struct with path resolution and CRUD
├── shell/
│   ├── mod.rs          # Shell struct, execute(), pipeline runner
│   ├── pipeline.rs     # Pipe splitting, redirect extraction
│   └── dispatch.rs     # Command dispatch via Registry
└── command/
    ├── mod.rs          # Command trait, CommandContext, Registry
    └── *.rs            # Individual command implementations

web/
├── main.ts             # Application bootstrap
├── terminal.ts         # Terminal creation and resize
├── input.ts            # Keyboard input handler
├── persistence.ts      # OPFS load/save helpers
├── auth.ts             # Authentication flow
└── style.css           # Global styles
```

## Making Changes

### Adding a New Command

This is the most common type of contribution. NexOS uses a trait-based command system that makes adding commands straightforward.

#### Step 1: Create the command file

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
        false // Set to true if this command reads from stdin via pipes
    }

    fn execute(&self, ctx: &mut CommandContext) -> Result<String, String> {
        // ctx.args   — command arguments (excluding the command name)
        // ctx.vfs    — mutable reference to the virtual filesystem
        // ctx.stdin  — stdin content from pipe (empty string if no pipe)
        // ctx.username, ctx.hostname — shell state
        // ctx.env_vars — environment variables

        // Return Ok(output) for success, Err(message) for errors
        Ok("output text\n".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vfs::Vfs;

    #[test]
    fn test_mycommand_basic() {
        let mut vfs = Vfs::new();
        let mut env = std::collections::HashMap::new();
        let ctx = CommandContext {
            vfs: &mut vfs,
            stdin: "",
            args: &[],
            username: "user",
            hostname: "nexos",
            history: &[],
            env_vars: &mut env,
        };
        let cmd = MyCommand;
        assert!(cmd.execute(&mut ctx).is_ok());
    }
}
```

#### Step 2: Register the command

In `src/command/mod.rs`:

1. Add `pub mod mycommand;` to the module declarations
2. Add `commands.push(Box::new(mycommand::MyCommand));` to `register_all()`

#### Step 3: Add tests

Add integration tests in `tests/shell_tests.rs`:

```rust
#[test]
fn test_mycommand() {
    let mut shell = Shell::new(Vfs::new());
    let output = shell.execute("mycommand");
    assert!(output.contains("expected text"));
}
```

#### Step 4: (Optional) Add a man page

The `man` command has manual pages for all built-in commands. To add one for your command, edit `src/command/man.rs` and add an entry to the `pages` map.

### Modifying Existing Commands

1. Find the command file in `src/command/`
2. Make your changes
3. Run existing tests to ensure nothing breaks: `cargo test`
4. Add new tests for your changes

### Frontend Changes

Frontend code lives in `web/`. The entry point is `main.ts`. Key modules:

- `terminal.ts` — Terminal creation, theme, resize handling
- `input.ts` — Keyboard input, history, tab completion, command execution
- `persistence.ts` — OPFS read/write helpers
- `auth.ts` — Login and account setup flow

After making changes:
```bash
cd web && npm run build
```

## Testing

### Running Tests

```bash
# All Rust tests (unit + integration)
cargo test

# Specific test by name
cargo test test_name

# With verbose output
cargo test -- --nocapture

# Frontend type checking
cd web && npx tsc --noEmit
```

### Writing Tests

- **Unit tests**: Place `#[cfg(test)]` modules at the bottom of the file being tested
- **Integration tests**: Add to `tests/shell_tests.rs` for command-level testing through the shell
- Tests should be self-contained — create a fresh `Shell` or `Vfs` instance per test
- Test both success and error cases
- Test edge cases (empty input, missing files, invalid arguments)

### Test Coverage

The project currently has 70+ integration tests covering:
- All 35 commands with various flag combinations
- Pipe chains (single and multi-stage)
- Output redirection (`>` and `>>`)
- Command chaining with `&&`
- Error handling and edge cases
- VFS persistence (JSON roundtrip)
- Tab completion
- Prompt formatting

## Code Style

### Rust

- Follow standard Rust conventions and idioms
- Run `cargo fmt` before committing
- Run `cargo clippy -- -D warnings` — all warnings are treated as errors in CI
- Use `Result<String, String>` for command return types — `Ok` for success, `Err` for errors
- Keep commands in separate files, one struct per file
- Document public functions with `///` doc comments

### TypeScript

- Strict TypeScript mode is enabled
- Run `npx tsc --noEmit` to check for type errors
- Use ES module imports
- Document exported functions with JSDoc comments

## Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**
- `feat` — New feature
- `fix` — Bug fix
- `docs` — Documentation changes
- `style` — Code style changes (formatting, no logic change)
- `refactor` — Code refactoring
- `test` — Adding or updating tests
- `chore` — Build process, CI, dependencies

**Examples:**
```
feat(command): add uniq command with -c flag
fix(pipe): handle quoted strings in pipe splitting
docs: update README with new commands
test(grep): add case-insensitive search tests
refactor: introduce Command trait and Registry
```

## Pull Requests

1. **Update your branch** with the latest changes from `main`:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```
2. **Ensure all tests pass**: `cargo test`
3. **Ensure code is formatted**: `cargo fmt`
4. **Ensure no clippy warnings**: `cargo clippy -- -D warnings`
5. **Write a clear PR description** explaining what changed and why
6. **Reference related issues** if applicable (e.g., "Fixes #42")
7. **Keep PRs focused** — one feature or fix per PR

### PR Checklist

- [ ] Tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] New commands have integration tests
- [ ] Documentation is updated if needed
- [ ] Commit messages follow Conventional Commits

## Reporting Issues

When reporting a bug, please include:

1. **Steps to reproduce** the issue
2. **Expected behavior** vs. **actual behavior**
3. **Browser and OS** information
4. **Console errors** (if any, from browser DevTools)

For feature requests, describe:
1. The feature you'd like
2. The use case it solves
3. Any implementation ideas (optional)

## Questions?

If you have questions about contributing, feel free to open an issue with the `question` label.

Thank you for contributing to NexOS!
