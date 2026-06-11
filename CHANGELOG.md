# Changelog

All notable changes to NexOS will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-06-11

### Added

#### Core System
- In-memory Virtual File System (VFS) with POSIX-style path resolution (`/`, `.`, `..`, `~`)
- Pre-seeded directory structure: `/home/user`, `/tmp`, `/etc`, `/var`
- JSON serialization for VFS state persistence
- Shell engine with command parsing, pipeline execution, and output redirection

#### Commands (35 total)
- **Filesystem navigation**: `ls` (with `-l`), `cd`, `pwd`, `mkdir` (with `-p`), `touch`, `rm` (with `-r`/`-rf`), `cp`, `mv`, `tree`, `ln` (with `-s`)
- **File content**: `cat`, `echo` (with `>`/`>>` redirection), `head` (with `-n`), `tail` (with `-n`)
- **Text processing**: `grep` (with `-i`/`-n`), `sort` (with `-r`), `uniq` (with `-c`), `wc` (with `-l`/`-w`/`-c`), `cut` (with `-f`/`-d`), `tr`, `tee` (with `-a`)
- **Comparison**: `diff` (LCS-based unified diff output)
- **Search**: `find` (with `-name`)
- **Disk usage**: `du` (with `-h`/`-s`)
- **Permissions**: `chmod` (octal and symbolic modes), `chown` (simulated)
- **System info**: `whoami`, `hostname`, `date`, `history`
- **Environment**: `env`, `export`
- **Path utilities**: `basename` (with suffix removal), `dirname`
- **Documentation**: `man` (full man pages for all commands), `help`
- **Terminal**: `clear`, `exit`

#### Shell Features
- Pipe support (`|`) with quote-aware parsing
- Output redirection (`>` overwrite, `>>` append)
- Command chaining with `&&` (stops on first error)
- Tab completion for command names
- Command history with Up/Down arrow navigation
- ANSI-colored prompt: `user@host:cwd$`

#### Frontend
- xterm.js terminal emulator with Cascadia Code font
- Responsive layout with FitAddon and ResizeObserver
- Keyboard shortcuts: Ctrl+C (cancel), Ctrl+L (clear)
- Dark theme with cursor blinking
- Clickable URL support via WebLinksAddon

#### Authentication
- First-time setup flow (username + password + confirmation)
- Returning user login (password only)
- SHA-256 password hashing via Web Crypto API
- Credentials stored in OPFS (`user_config.json`)

#### Persistence
- Automatic VFS state save to OPFS after every command
- VFS restoration on page load
- Graceful fallback to memory-only mode when OPFS is unavailable

#### WASM API
- 7 exported functions: `init`, `init_with_username`, `execute_command`, `get_prompt`, `get_completions`, `get_history`, `get_state_json`

#### Testing
- 70+ integration tests covering all commands, pipes, redirection, chaining, and persistence
- Unit tests in individual command modules and VFS tree
- CI pipeline with Rust linting (clippy, rustfmt), tests, and WASM build

#### Deployment
- Cloudflare Pages deployment via Wrangler CLI
- Automated CI/CD pipeline (GitHub Actions)
- Production WASM binary optimized with LTO and `opt-level = "z"`
