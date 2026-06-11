# Command Reference

NexOS provides 35 built-in commands that emulate common Linux/Unix utilities. This document describes each command's syntax, options, and behavior.

## Table of Contents

- [Filesystem Navigation](#filesystem-navigation)
  - [ls](#ls) — List directory contents
  - [cd](#cd) — Change directory
  - [pwd](#pwd) — Print working directory
  - [mkdir](#mkdir) — Create directories
  - [touch](#touch) — Create files
  - [rm](#rm) — Remove files and directories
  - [cp](#cp) — Copy files and directories
  - [mv](#mv) — Move/rename files and directories
  - [tree](#tree) — Display directory tree
  - [ln](#ln) — Create links
- [File Content](#file-content)
  - [cat](#cat) — Display file contents
  - [echo](#echo) — Display text / write to files
  - [head](#head) — Display first lines
  - [tail](#tail) — Display last lines
- [Text Processing](#text-processing)
  - [grep](#grep) — Search for patterns
  - [sort](#sort) — Sort lines
  - [uniq](#uniq) — Filter duplicate lines
  - [wc](#wc) — Count lines, words, characters
  - [cut](#cut) — Extract fields
  - [tr](#tr) — Translate characters
  - [tee](#tee) — Write to stdout and files
- [Comparison](#comparison)
  - [diff](#diff) — Compare files
- [Search and Usage](#search-and-usage)
  - [find](#find) — Find files by name
  - [du](#du) — Disk usage
- [Permissions and Ownership](#permissions-and-ownership)
  - [chmod](#chmod) — Change permissions
  - [chown](#chown) — Change ownership
- [System Information](#system-information)
  - [whoami](#whoami) — Current username
  - [hostname](#hostname) — System hostname
  - [date](#date) — Current date/time
  - [history](#history) — Command history
- [Environment](#environment)
  - [env](#env) — List environment variables
  - [export](#export) — Set environment variables
- [Path Utilities](#path-utilities)
  - [basename](#basename) — Strip directory from path
  - [dirname](#dirname) — Strip filename from path
- [Documentation and Terminal](#documentation-and-terminal)
  - [man](#man) — Manual pages
  - [help](#help) — List available commands
  - [clear](#clear) — Clear terminal
  - [exit](#exit) — Exit session
- [Shell Features](#shell-features)
  - [Pipes](#pipes)
  - [Output Redirection](#output-redirection)
  - [Command Chaining](#command-chaining)

---

## Filesystem Navigation

### `ls`

List directory contents.

**Synopsis:**
```
ls [path]
ls -l [path]
```

**Options:**
| Flag | Description |
|------|-------------|
| `-l` | Long format — shows type prefix (`-` for file, `d` for directory) |

**Behavior:**
- Without arguments, lists the current directory
- Entries are sorted alphabetically
- With `-l`, each line is prefixed with the node type

**Examples:**
```bash
ls                  # List current directory
ls /home            # List /home
ls -l               # Long format listing
ls -l /tmp          # Long format of /tmp
```

---

### `cd`

Change the current working directory.

**Synopsis:**
```
cd [path]
```

**Behavior:**
- `cd` with no arguments or `cd ~` changes to `/home/user`
- `cd ..` moves to the parent directory
- `cd /` moves to the root
- Supports absolute and relative paths
- `..` at root stays at root (no error)

**Examples:**
```bash
cd                  # Go to home directory
cd /tmp             # Go to /tmp
cd ..               # Go to parent directory
cd ~/Documents      # Go to /home/user/Documents
cd ../../           # Go up two levels
```

---

### `pwd`

Print the absolute path of the current working directory.

**Synopsis:**
```
pwd
```

**Examples:**
```bash
pwd                 # Output: /
cd /home/user
pwd                 # Output: /home/user
```

---

### `mkdir`

Create new directories.

**Synopsis:**
```
mkdir [-p] <directory> [directory2 ...]
```

**Options:**
| Flag | Description |
|------|-------------|
| `-p` | Create parent directories as needed (no error if existing) |

**Behavior:**
- Creates one or more directories
- Without `-p`, fails if the parent directory does not exist
- With `-p`, creates intermediate directories as needed
- Fails if the directory already exists

**Examples:**
```bash
mkdir project           # Create /home/user/project
mkdir -p a/b/c          # Create nested directories
mkdir dir1 dir2 dir3    # Create multiple directories
```

---

### `touch`

Create empty files. No-op if the file already exists.

**Synopsis:**
```
touch <file> [file2 ...]
```

**Behavior:**
- Creates one or more empty files
- If the file already exists, does nothing (matches POSIX `touch` without timestamps)
- Fails if the parent directory does not exist

**Examples:**
```bash
touch README.md             # Create a single file
touch a.txt b.txt c.txt     # Create multiple files
```

---

### `rm`

Remove files or directories.

**Synopsis:**
```
rm [-r | -rf | -fr] <path> [path2 ...]
```

**Options:**
| Flag | Description |
|------|-------------|
| `-r` | Remove directories and their contents recursively |
| `-rf`, `-fr` | Same as `-r` (shorthand) |

**Behavior:**
- Without `-r`, refuses to remove non-empty directories
- Cannot remove the root directory `/`
- Multiple paths can be specified

**Examples:**
```bash
rm file.txt             # Remove a file
rm -r directory         # Remove a directory recursively
rm -rf temp build       # Remove multiple directories
```

---

### `cp`

Copy files or directories.

**Synopsis:**
```
cp <source> <destination>
```

**Behavior:**
- If destination is an existing directory, copies the source into it
- Deep-clones directories (recursive copy)
- The original source is preserved

**Examples:**
```bash
cp file.txt backup.txt          # Copy file
cp file.txt /tmp/               # Copy into directory
cp -r project project_backup    # Copy directory (Note: -r flag is accepted but not required)
```

---

### `mv`

Move or rename files and directories.

**Synopsis:**
```
mv <source> <destination>
```

**Behavior:**
- If destination is an existing directory, moves the source into it
- Works for both files and directories
- Implemented as copy + remove

**Examples:**
```bash
mv old.txt new.txt          # Rename file
mv file.txt /tmp/           # Move to directory
mv project archive/         # Move directory
```

---

### `tree`

Display directory structure as a tree with Unicode box-drawing characters.

**Synopsis:**
```
tree [path]
```

**Behavior:**
- Without arguments, displays the tree from the current directory
- Shows directory and file counts at the end
- Uses `├──`, `└──`, and `│` characters for the tree structure

**Examples:**
```bash
tree                 # Tree of current directory
tree /home           # Tree of /home
```

**Sample output:**
```
├── documents/
│   ├── notes.txt
│   └── todo.txt
├── projects/
│   └── nexos/
│       └── README.md
└── .bashrc

3 directories, 4 files
```

---

### `ln`

Create links to files.

**Synopsis:**
```
ln [-s] <target> <link_name>
```

**Options:**
| Flag | Description |
|------|-------------|
| `-s` | Create a symbolic link (writes `-> target` as text content) |

**Behavior:**
- With `-s`: Creates a file whose content is `-> <target>` (simulated symbolic link)
- Without `-s`: Copies the source file content (simulated hard link, since the VFS has no inode system)

**Examples:**
```bash
ln -s /usr/bin/python python3     # Symbolic link
ln original.txt copy.txt          # Hard link (copy)
```

---

## File Content

### `cat`

Display file contents. Accepts stdin from pipes.

**Synopsis:**
```
cat <file> [file2 ...]
```

**Behavior:**
- Displays contents of one or more files
- Multiple files are concatenated
- Ensures output ends with a newline
- When used in a pipe, stdin is passed as a file argument

**Examples:**
```bash
cat README.md               # Display single file
cat a.txt b.txt             # Concatenate two files
echo "hello" | cat          # Display piped input
```

---

### `echo`

Display text or write to files.

**Synopsis:**
```
echo [text ...]
echo [text ...] > <file>
echo [text ...] >> <file>
```

**Behavior:**
- Outputs the arguments separated by spaces
- `> file` overwrites the file with the output
- `>> file` appends the output to the file
- If the file doesn't exist, it is created

**Examples:**
```bash
echo Hello World                # Output: Hello World
echo "Hello, World!" > msg.txt # Write to file
echo "More text" >> msg.txt    # Append to file
echo $USER                     # Environment variable expansion is NOT supported
```

---

### `head`

Display the first N lines of input. Accepts stdin from pipes.

**Synopsis:**
```
head [-n COUNT] [file]
```

**Options:**
| Flag | Description |
|------|-------------|
| `-n COUNT` | Number of lines to display (default: 10) |

**Behavior:**
- Displays the first N lines of a file or stdin
- Supports `-n COUNT` and `-nCOUNT` forms
- Without a file argument, reads from stdin (in pipes)

**Examples:**
```bash
head log.txt                # First 10 lines
head -n 5 log.txt           # First 5 lines
head -n20 data.csv          # First 20 lines
cat file | head -n 3        # First 3 lines via pipe
```

---

### `tail`

Display the last N lines of input. Accepts stdin from pipes.

**Synopsis:**
```
tail [-n COUNT] [file]
```

**Options:**
| Flag | Description |
|------|-------------|
| `-n COUNT` | Number of lines to display (default: 10) |

**Behavior:**
- Displays the last N lines of a file or stdin
- Supports `-n COUNT` and `-nCOUNT` forms

**Examples:**
```bash
tail log.txt                # Last 10 lines
tail -n 5 log.txt           # Last 5 lines
cat file | tail -n 1        # Last line via pipe
```

---

## Text Processing

### `grep`

Search for patterns in files or stdin. Accepts stdin from pipes.

**Synopsis:**
```
grep [-i] [-n] [-in | -ni] <pattern> [file ...]
```

**Options:**
| Flag | Description |
|------|-------------|
| `-i` | Case-insensitive matching |
| `-n` | Show line numbers |
| `-in`, `-ni` | Combined case-insensitive + line numbers |

**Behavior:**
- Searches for lines containing the pattern (substring match)
- When multiple files are given, prefixes each match with the filename
- Without file arguments, reads from stdin (in pipes)

**Examples:**
```bash
grep "error" log.txt            # Search in file
grep -i "warning" log.txt       # Case-insensitive
grep -n "TODO" src/*.rs         # With line numbers
grep -in "bug" code.txt         # Combined flags
cat file | grep "pattern"       # Via pipe
grep "err" a.log b.log          # Multiple files (shows filename prefix)
```

---

### `sort`

Sort lines alphabetically. Accepts stdin from pipes.

**Synopsis:**
```
sort [-r] [file]
```

**Options:**
| Flag | Description |
|------|-------------|
| `-r` | Reverse sort order |

**Examples:**
```bash
sort names.txt              # Sort alphabetically
sort -r names.txt           # Reverse sort
echo -e "banana\napple\ncherry" | sort    # Sort piped input
```

---

### `uniq`

Filter adjacent duplicate lines. Accepts stdin from pipes.

**Synopsis:**
```
uniq [-c] [file]
```

**Options:**
| Flag | Description |
|------|-------------|
| `-c` | Prefix each line with its occurrence count |

**Behavior:**
- Only removes *adjacent* duplicates (use `sort | uniq` to deduplicate globally)

**Examples:**
```bash
uniq sorted.txt                 # Remove adjacent duplicates
uniq -c sorted.txt              # Show counts
sort data.txt | uniq -c         # Global dedup with counts
```

---

### `wc`

Count lines, words, and characters. Accepts stdin from pipes.

**Synopsis:**
```
wc [-l] [-w] [-c] [file ...]
```

**Options:**
| Flag | Description |
|------|-------------|
| `-l` | Count lines only |
| `-w` | Count words only |
| `-c` | Count characters only |

**Behavior:**
- Without flags, shows all three counts: lines, words, characters
- When multiple files are given, shows a total line at the end

**Examples:**
```bash
wc README.md                # Lines, words, characters
wc -l log.txt               # Line count only
wc -w essay.txt             # Word count only
echo "hello world" | wc -w  # Output: 2
wc a.txt b.txt              # Individual counts + total
```

---

### `cut`

Extract fields from each line. Accepts stdin from pipes.

**Synopsis:**
```
cut -f FIELDS [-d DELIM] [file]
```

**Options:**
| Flag | Description |
|------|-------------|
| `-f FIELDS` | Comma-separated field numbers (1-indexed) |
| `-d DELIM` | Field delimiter (default: tab) |

**Examples:**
```bash
cut -f 1 data.tsv               # First field (tab-delimited)
cut -f 1,3 -d "," data.csv      # Fields 1 and 3 (comma-delimited)
cat /etc/passwd | cut -f 1 -d ":"  # Via pipe
```

---

### `tr`

Translate (replace) characters from stdin.

**Synopsis:**
```
tr <set1> <set2>
```

**Behavior:**
- Reads from stdin (accesses `ctx.stdin` directly)
- Maps each character in set1 to the corresponding character in set2
- If set1 is longer than set2, excess characters map to the last character of set2
- Supports escape sequences: `\n` (newline), `\t` (tab), `\\` (backslash)

**Examples:**
```bash
echo "hello" | tr "a-z" "A-Z"     # Convert to uppercase
echo "hello" | tr "el" "EL"        # hELLo
echo "a-b-c" | tr "-" "/"          # a/b/c
```

---

### `tee`

Read from stdin and write to both stdout and files.

**Synopsis:**
```
tee [-a] [file ...]
```

**Options:**
| Flag | Description |
|------|-------------|
| `-a` | Append to files instead of overwriting |

**Behavior:**
- Reads stdin (accesses `ctx.stdin` directly)
- Writes stdin content to stdout AND to each specified file
- Without `-a`, overwrites files; with `-a`, appends

**Examples:**
```bash
echo "hello" | tee output.txt          # Write to stdout and file
echo "log" | tee -a log.txt            # Append to file
echo "data" | tee a.txt b.txt          # Write to multiple files
```

---

## Comparison

### `diff`

Compare two files line by line.

**Synopsis:**
```
diff <file1> <file2>
```

**Behavior:**
- Uses the LCS (Longest Common Subsequence) algorithm
- Outputs unified diff format with `---`, `+++`, and `@@` headers
- Lines prefixed with `-` are only in file1, `+` only in file2

**Examples:**
```bash
diff old.txt new.txt
```

**Sample output:**
```
--- old.txt
+++ new.txt
@@ -1,3 +1,3 @@
 unchanged line
-removed line
+added line
```

---

## Search and Usage

### `find`

Find files and directories by name.

**Synopsis:**
```
find [path] -name <pattern>
```

**Behavior:**
- Recursively searches from the given path (default: current directory)
- Pattern matching is case-sensitive substring matching
- Returns matching paths relative to the search root

**Examples:**
```bash
find -name "*.txt"              # Find all .txt in current directory
find /home -name "README"      # Find README under /home
find . -name "config"          # Find config files
```

---

### `du`

Estimate disk usage (file content size in bytes).

**Synopsis:**
```
du [-h] [-s] [path]
```

**Options:**
| Flag | Description |
|------|-------------|
| `-h` | Human-readable output (K, M suffixes) |
| `-s` | Summary only (total for the directory) |

**Behavior:**
- Without arguments, shows usage for the current directory
- Default display is in kilobytes
- Recursively sums file content lengths

**Examples:**
```bash
du                  # Usage of current directory
du -h /home         # Human-readable
du -s /tmp          # Summary only
du -sh /home        # Human-readable summary
```

---

## Permissions and Ownership

### `chmod`

Change file permissions (simulated — does not enforce permissions).

**Synopsis:**
```
chmod <mode> <file>
```

**Mode formats:**
- **Octal**: `755`, `0644`, `777`
- **Symbolic**: `+x`, `-w`, `u+rwx`, `g+r`, `o-w`

**Behavior:**
- Validates the mode format
- Does not actually store or enforce permissions (the VFS has no permission system)

**Examples:**
```bash
chmod 755 script.sh         # rwxr-xr-x
chmod 644 config.txt        # rw-r--r--
chmod +x run.sh             # Add execute bit
chmod -w file.txt           # Remove write bit
chmod u+rwx mydir           # Full permissions for user
```

---

### `chown`

Change file ownership (simulated — does not store ownership).

**Synopsis:**
```
chown <owner>[:<group>] <file>
```

**Behavior:**
- Validates the owner/group format
- Does not actually store ownership information

**Examples:**
```bash
chown alice file.txt            # Change owner
chown alice:staff file.txt      # Change owner and group
```

---

## System Information

### `whoami`

Display the current username.

**Synopsis:**
```
whoami
```

**Example output:**
```
alice
```

---

### `hostname`

Display the system hostname.

**Synopsis:**
```
hostname
```

**Example output:**
```
nexos
```

---

### `date`

Display the current date and time.

**Synopsis:**
```
date
```

**Behavior:**
- In the browser (WASM): Displays ISO-8601 UTC timestamp via `js_sys::Date`
- In native tests: Displays epoch seconds via `SystemTime`

**Example output:**
```
2026-06-11T14:30:00.000Z
```

---

### `history`

Display the numbered command history.

**Synopsis:**
```
history
```

**Example output:**
```
    1  ls
    2  cd project
    3  cat README.md
    4  echo hello > greeting.txt
```

---

## Environment

### `env`

Display all environment variables as sorted `KEY=VALUE` pairs.

**Synopsis:**
```
env
```

**Example output:**
```
HOME=/home/user
HOSTNAME=nexos
PATH=/usr/bin:/bin
PWD=/home/user
SHELL=/bin/nexsh
TERM=xterm-256color
USER=alice
```

---

### `export`

Set environment variables.

**Synopsis:**
```
export KEY=VALUE
export
```

**Behavior:**
- `export KEY=VALUE` sets a variable
- `export` with no arguments lists all variables in `declare -x KEY="VALUE"` format

**Examples:**
```bash
export EDITOR=vim            # Set EDITOR
export PATH=/usr/bin:/bin    # Set PATH
export                       # List all variables
```

---

## Path Utilities

### `basename`

Strip the directory component from a filename.

**Synopsis:**
```
basename <path> [suffix]
```

**Behavior:**
- Returns the last component of the path
- If a suffix is given and matches the end of the filename, it is removed

**Examples:**
```bash
basename /home/user/file.txt        # file.txt
basename /home/user/file.txt .txt   # file
basename /usr/local/bin/            # bin
```

---

### `dirname`

Strip the last component from a path.

**Synopsis:**
```
dirname <path>
```

**Behavior:**
- Returns everything before the last `/`
- Returns `.` if there is no directory component

**Examples:**
```bash
dirname /home/user/file.txt     # /home/user
dirname /usr/local/bin/         # /usr/local
dirname file.txt                # .
```

---

## Documentation and Terminal

### `man`

Display manual pages for commands.

**Synopsis:**
```
man <command>
```

**Behavior:**
- Displays a full manual page with NAME, SYNOPSIS, DESCRIPTION, and EXAMPLES sections
- Available for all 35 built-in commands

**Examples:**
```bash
man grep            # Display grep manual
man ls              # Display ls manual
man nonexistent     # Error: no manual entry
```

---

### `help`

Display a table of all available commands with descriptions.

**Synopsis:**
```
help
```

---

### `clear`

Clear the terminal screen.

**Synopsis:**
```
clear
```

**Behavior:**
- Returns the ANSI escape sequence `\x1b[2J\x1b[H` which the frontend interprets as a screen clear

---

### `exit`

Exit the terminal session.

**Synopsis:**
```
exit
```

**Behavior:**
- Returns an empty string
- The frontend may handle this as a session end

---

## Shell Features

### Pipes

Chain commands using `|` to pass stdout of one command as stdin to the next:

```bash
cat access.log | grep "404" | wc -l
```

**Pipe-aware commands** (accept stdin as file input):
`cat`, `head`, `tail`, `grep`, `sort`, `uniq`, `wc`, `cut`

**Direct stdin readers** (read `ctx.stdin` directly):
`tr`, `tee`

Quote-aware: pipes inside quoted strings are not split:
```bash
echo "hello | world"     # Single echo argument, not a pipe
```

### Output Redirection

Redirect command output to a file:

```bash
# Overwrite
echo "Hello" > greeting.txt

# Append
echo "World" >> greeting.txt
```

- `>` creates or overwrites the file
- `>>` creates or appends to the file
- Redirection applies only to the **last stage** of a pipeline:
  ```bash
  cat log | grep error > errors.txt   # grep output goes to file
  ```

### Command Chaining

Chain commands with `&&` for sequential execution with short-circuit on error:

```bash
mkdir project && cd project && touch README.md
```

- Commands execute left to right
- If any command returns an error (`Err`), the chain stops
- Each `&&` segment can contain its own pipes

```bash
cat file | grep pattern && echo "Found matches" > result.txt
```
