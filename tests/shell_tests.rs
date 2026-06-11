//! Integration tests for the NexOS shell.
//!
//! These tests exercise the full command execution pipeline — from raw input
//! string through parsing, dispatch, and VFS mutation — without involving
//! the WASM layer or the browser frontend.
//!
//! Tests are grouped by feature area:
//! - Filesystem commands (cd, ls, mkdir, rm, cp, mv, tree, touch)
//! - File content commands (echo, cat, head, tail)
//! - Text processing (grep, sort, uniq, wc, cut, tr, tee, diff)
//! - Search and disk usage (find, du)
//! - System info (whoami, hostname, date, history)
//! - Environment variables (env, export)
//! - Path utilities (basename, dirname)
//! - Link commands (ln)
//! - Permissions (chmod, chown)
//! - Documentation (man, help)
//! - Terminal (clear, exit)
//! - Piping (`|`), redirection (`>`, `>>`), and chaining (`&&`)
//! - Prompt and tab completion

use nexos::shell::Shell;
use nexos::vfs::Vfs;

/// Helper: create a fresh [`Shell`] with a default VFS for testing.
fn new_shell() -> Shell {
    Shell::new(Vfs::new())
}

// =========================================================================
// Filesystem commands
// =========================================================================

/// `pwd` at the initial working directory should report `/`.
#[test]
fn test_pwd_at_root() {
    let mut shell = new_shell();
    let out = shell.execute("pwd");
    assert_eq!(out.trim(), "/");
}

/// `cd` followed by `pwd` should reflect the new working directory.
#[test]
fn test_cd_and_pwd() {
    let mut shell = new_shell();
    shell.execute("cd /home/user");
    let out = shell.execute("pwd");
    assert_eq!(out.trim(), "/home/user");
}

/// `cd ~` should expand to `/home/user`.
#[test]
fn test_cd_tilde() {
    let mut shell = new_shell();
    shell.execute("cd ~");
    let out = shell.execute("pwd");
    assert_eq!(out.trim(), "/home/user");
}

/// `cd ..` should navigate to the parent directory.
#[test]
fn test_cd_dotdot() {
    let mut shell = new_shell();
    shell.execute("cd /home/user");
    shell.execute("cd ..");
    let out = shell.execute("pwd");
    assert_eq!(out.trim(), "/home");
}

/// `mkdir` followed by `ls` should show the newly created directory.
#[test]
fn test_mkdir_and_ls() {
    let mut shell = new_shell();
    shell.execute("mkdir /tmp/testdir");
    let out = shell.execute("ls /tmp");
    assert!(out.contains("testdir"));
}

/// `mkdir -p` should create intermediate directories recursively.
#[test]
fn test_mkdir_p_recursive() {
    let mut shell = new_shell();
    shell.execute("mkdir -p /tmp/a/b/c");
    assert!(shell.vfs.is_dir("/tmp/a/b/c"));
}

/// `touch` should create a new empty file.
#[test]
fn test_touch_creates_file() {
    let mut shell = new_shell();
    shell.execute("touch /tmp/newfile.txt");
    assert!(shell.vfs.exists("/tmp/newfile.txt"));
}

/// `rm` should remove an existing file.
#[test]
fn test_rm_file() {
    let mut shell = new_shell();
    shell.execute("touch /tmp/to_delete.txt");
    shell.execute("rm /tmp/to_delete.txt");
    assert!(!shell.vfs.exists("/tmp/to_delete.txt"));
}

/// `rm` without `-r` should refuse to remove a directory.
#[test]
fn test_rm_dir_without_r_fails() {
    let mut shell = new_shell();
    shell.execute("mkdir /tmp/del_dir");
    let out = shell.execute("rm /tmp/del_dir");
    assert!(out.contains("directory") || out.contains("Is a directory"));
}

/// `rm -r` should remove a directory and all its contents.
#[test]
fn test_rm_recursive() {
    let mut shell = new_shell();
    shell.execute("mkdir /tmp/del_dir");
    shell.execute("touch /tmp/del_dir/f.txt");
    shell.execute("rm -r /tmp/del_dir");
    assert!(!shell.vfs.exists("/tmp/del_dir"));
}

/// `cp` should copy a file, preserving its content.
#[test]
fn test_cp_file() {
    let mut shell = new_shell();
    shell.execute("echo data > /tmp/src.txt");
    shell.execute("cp /tmp/src.txt /tmp/dst.txt");
    let out = shell.execute("cat /tmp/dst.txt");
    assert!(out.contains("data"));
}

/// `mv` should move a file, removing the original.
#[test]
fn test_mv_file() {
    let mut shell = new_shell();
    shell.execute("echo data > /tmp/old.txt");
    shell.execute("mv /tmp/old.txt /tmp/new.txt");
    assert!(!shell.vfs.exists("/tmp/old.txt"));
    let out = shell.execute("cat /tmp/new.txt");
    assert!(out.contains("data"));
}

/// `tree` should display the directory structure recursively.
#[test]
fn test_tree() {
    let mut shell = new_shell();
    shell.execute("mkdir /tmp/tree_test");
    shell.execute("touch /tmp/tree_test/a.txt");
    shell.execute("touch /tmp/tree_test/b.txt");
    let out = shell.execute("tree /tmp/tree_test");
    assert!(out.contains("a.txt"));
    assert!(out.contains("b.txt"));
}

// =========================================================================
// File content commands
// =========================================================================

/// `echo` with plain text should output the text followed by a newline.
#[test]
fn test_echo_basic() {
    let mut shell = new_shell();
    let out = shell.execute("echo Hello World");
    assert_eq!(out.trim(), "Hello World");
}

/// `echo ... > file` should overwrite the file content.
#[test]
fn test_echo_redirect_overwrite() {
    let mut shell = new_shell();
    shell.execute("echo first > /tmp/out.txt");
    shell.execute("echo second > /tmp/out.txt");
    let out = shell.execute("cat /tmp/out.txt");
    assert!(out.contains("second"));
    assert!(!out.contains("first"));
}

/// `echo ... >> file` should append to the file content.
#[test]
fn test_echo_redirect_append() {
    let mut shell = new_shell();
    shell.execute("echo line1 > /tmp/out.txt");
    shell.execute("echo line2 >> /tmp/out.txt");
    let out = shell.execute("cat /tmp/out.txt");
    assert!(out.contains("line1"));
    assert!(out.contains("line2"));
}

/// `cat` with multiple files should concatenate their contents.
#[test]
fn test_cat_multiple_files() {
    let mut shell = new_shell();
    shell.execute("echo AAA > /tmp/a.txt");
    shell.execute("echo BBB > /tmp/b.txt");
    let out = shell.execute("cat /tmp/a.txt /tmp/b.txt");
    assert!(out.contains("AAA"));
    assert!(out.contains("BBB"));
}

/// `head` without `-n` should show the first 10 lines by default.
#[test]
fn test_head_default() {
    let mut shell = new_shell();
    for i in 1..=20 {
        shell.execute(&format!("echo line{} >> /tmp/many.txt", i));
    }
    let out = shell.execute("head /tmp/many.txt");
    assert!(out.contains("line1"));
    assert!(out.contains("line10"));
    assert!(!out.contains("line11"));
}

/// `head -n 3` should show exactly the first 3 lines.
#[test]
fn test_head_n() {
    let mut shell = new_shell();
    for i in 1..=20 {
        shell.execute(&format!("echo line{} >> /tmp/many.txt", i));
    }
    let out = shell.execute("head -n 3 /tmp/many.txt");
    assert!(out.contains("line1"));
    assert!(out.contains("line3"));
    assert!(!out.contains("line4"));
}

/// `tail -n 3` should show the last 3 lines.
#[test]
fn test_tail_n() {
    let mut shell = new_shell();
    for i in 1..=20 {
        shell.execute(&format!("echo line{} >> /tmp/many.txt", i));
    }
    let out = shell.execute("tail -n 3 /tmp/many.txt");
    assert!(out.contains("line18"));
    assert!(out.contains("line20"));
}

// =========================================================================
// Text processing commands
// =========================================================================

/// `grep` should find lines matching the given pattern.
#[test]
fn test_grep_basic() {
    let mut shell = new_shell();
    shell.execute("echo hello > /tmp/f.txt");
    shell.execute("echo world >> /tmp/f.txt");
    shell.execute("echo hello again >> /tmp/f.txt");
    let out = shell.execute("grep hello /tmp/f.txt");
    assert_eq!(out.lines().count(), 2);
}

/// `grep -i` should match case-insensitively.
#[test]
fn test_grep_case_insensitive() {
    let mut shell = new_shell();
    shell.execute("echo Hello > /tmp/f.txt");
    shell.execute("echo WORLD >> /tmp/f.txt");
    let out = shell.execute("grep -i hello /tmp/f.txt");
    assert!(out.contains("Hello"));
}

/// `grep -n` should prefix matching lines with their line numbers.
#[test]
fn test_grep_line_numbers() {
    let mut shell = new_shell();
    shell.execute("echo aaa > /tmp/f.txt");
    shell.execute("echo bbb >> /tmp/f.txt");
    shell.execute("echo aaa >> /tmp/f.txt");
    let out = shell.execute("grep -n aaa /tmp/f.txt");
    assert!(out.contains("1:"));
    assert!(out.contains("3:"));
}

/// `sort` should sort lines alphabetically.
#[test]
fn test_sort_basic() {
    let mut shell = new_shell();
    shell.execute("echo banana > /tmp/f.txt");
    shell.execute("echo apple >> /tmp/f.txt");
    shell.execute("echo cherry >> /tmp/f.txt");
    let out = shell.execute("sort /tmp/f.txt");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "apple");
    assert_eq!(lines[1], "banana");
    assert_eq!(lines[2], "cherry");
}

/// `sort -r` should sort lines in reverse alphabetical order.
#[test]
fn test_sort_reverse() {
    let mut shell = new_shell();
    shell.execute("echo banana > /tmp/f.txt");
    shell.execute("echo apple >> /tmp/f.txt");
    let out = shell.execute("sort -r /tmp/f.txt");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "banana");
    assert_eq!(lines[1], "apple");
}

/// `uniq -c` should prefix each unique line with its occurrence count.
#[test]
fn test_uniq_with_count() {
    let mut shell = new_shell();
    shell.execute("echo a > /tmp/f.txt");
    shell.execute("echo a >> /tmp/f.txt");
    shell.execute("echo b >> /tmp/f.txt");
    shell.execute("echo b >> /tmp/f.txt");
    shell.execute("echo b >> /tmp/f.txt");
    shell.execute("echo c >> /tmp/f.txt");
    let out = shell.execute("uniq -c /tmp/f.txt");
    assert!(out.contains("2 a"));
    assert!(out.contains("3 b"));
    assert!(out.contains("1 c"));
}

/// `wc` should report line, word, and character counts.
#[test]
fn test_wc_lines_words_chars() {
    let mut shell = new_shell();
    shell.execute("echo hello world > /tmp/f.txt");
    shell.execute("echo foo bar >> /tmp/f.txt");
    let out = shell.execute("wc /tmp/f.txt");
    assert!(out.contains("2")); // 2 lines
    assert!(out.contains("4")); // 4 words
}

/// `wc -l` should report only the line count.
#[test]
fn test_wc_lines_only() {
    let mut shell = new_shell();
    shell.execute("echo a > /tmp/f.txt");
    shell.execute("echo b >> /tmp/f.txt");
    shell.execute("echo c >> /tmp/f.txt");
    let out = shell.execute("wc -l /tmp/f.txt");
    assert!(out.contains("3"));
}

/// `cut -f 1,3 -d ,` should extract fields 1 and 3 using comma as delimiter.
#[test]
fn test_cut_fields() {
    let mut shell = new_shell();
    shell.execute("echo 'a,b,c' > /tmp/f.txt");
    shell.execute("echo 'd,e,f' >> /tmp/f.txt");
    let out = shell.execute("cut -f 1,3 -d , /tmp/f.txt");
    assert!(out.contains("a,c"));
    assert!(out.contains("d,f"));
}

/// `diff` on identical files should produce no output.
#[test]
fn test_diff_identical_files() {
    let mut shell = new_shell();
    shell.execute("echo same > /tmp/a.txt");
    shell.execute("echo same > /tmp/b.txt");
    let out = shell.execute("diff /tmp/a.txt /tmp/b.txt");
    assert!(out.is_empty());
}

/// `diff` on different files should produce a non-empty diff.
#[test]
fn test_diff_different_files() {
    let mut shell = new_shell();
    shell.execute("echo hello > /tmp/a.txt");
    shell.execute("echo world > /tmp/b.txt");
    let out = shell.execute("diff /tmp/a.txt /tmp/b.txt");
    assert!(!out.is_empty());
    assert!(out.contains("-") || out.contains("+"));
}

// =========================================================================
// Search and disk usage
// =========================================================================

/// `find ... -name pattern` should return matching files recursively.
#[test]
fn test_find_by_name() {
    let mut shell = new_shell();
    shell.execute("mkdir /tmp/search_test");
    shell.execute("touch /tmp/search_test/readme.txt");
    shell.execute("touch /tmp/search_test/data.csv");
    shell.execute("touch /tmp/search_test/readme.md");
    let out = shell.execute("find /tmp/search_test -name readme");
    assert!(out.contains("readme.txt"));
    assert!(out.contains("readme.md"));
    assert!(!out.contains("data.csv"));
}

/// `du -s` should report disk usage for the given path.
#[test]
fn test_du_summary() {
    let mut shell = new_shell();
    shell.execute("echo some content here > /tmp/f.txt");
    let out = shell.execute("du -s /tmp");
    assert!(!out.is_empty());
}

// =========================================================================
// System info commands
// =========================================================================

/// `whoami` should return the current username.
#[test]
fn test_whoami() {
    let mut shell = new_shell();
    let out = shell.execute("whoami");
    assert_eq!(out.trim(), "user");
}

/// `hostname` should return the machine hostname.
#[test]
fn test_hostname() {
    let mut shell = new_shell();
    let out = shell.execute("hostname");
    assert_eq!(out.trim(), "nexos");
}

/// `date` should return a non-empty string.
#[test]
fn test_date_returns_output() {
    let mut shell = new_shell();
    let out = shell.execute("date");
    assert!(!out.is_empty());
}

/// `history` should list all previously executed commands.
#[test]
fn test_history_tracks_commands() {
    let mut shell = new_shell();
    shell.execute("echo a");
    shell.execute("echo b");
    shell.execute("echo c");
    let out = shell.execute("history");
    assert!(out.contains("echo a"));
    assert!(out.contains("echo b"));
    assert!(out.contains("echo c"));
}

// =========================================================================
// Environment variables
// =========================================================================

/// `env` should display the default environment variables.
#[test]
fn test_env_shows_defaults() {
    let mut shell = new_shell();
    let out = shell.execute("env");
    assert!(out.contains("USER=user"));
    assert!(out.contains("HOSTNAME=nexos"));
}

/// `export KEY=value` should add the variable to the environment.
#[test]
fn test_export_sets_variable() {
    let mut shell = new_shell();
    shell.execute("export MY_VAR=hello");
    let out = shell.execute("env");
    assert!(out.contains("MY_VAR=hello"));
}

/// `export` without arguments should list all exported variables.
#[test]
fn test_export_no_args_lists_all() {
    let mut shell = new_shell();
    shell.execute("export FOO=bar");
    let out = shell.execute("export");
    assert!(out.contains("FOO"));
}

// =========================================================================
// Path utilities
// =========================================================================

/// `basename` should extract the final component of a path.
#[test]
fn test_basename() {
    let mut shell = new_shell();
    let out = shell.execute("basename /home/user/file.txt");
    assert_eq!(out.trim(), "file.txt");
}

/// `basename path .ext` should strip the given suffix.
#[test]
fn test_basename_with_suffix() {
    let mut shell = new_shell();
    let out = shell.execute("basename /home/user/file.txt .txt");
    assert_eq!(out.trim(), "file");
}

/// `dirname` should extract the parent directory of a path.
#[test]
fn test_dirname() {
    let mut shell = new_shell();
    let out = shell.execute("dirname /home/user/file.txt");
    assert_eq!(out.trim(), "/home/user");
}

/// `dirname` on a bare filename should return `.`.
#[test]
fn test_dirname_no_slash() {
    let mut shell = new_shell();
    let out = shell.execute("dirname file.txt");
    assert_eq!(out.trim(), ".");
}

// =========================================================================
// Link commands
// =========================================================================

/// `ln -s` should create a symbolic link showing the target path.
#[test]
fn test_ln_symbolic() {
    let mut shell = new_shell();
    shell.execute("echo content > /tmp/target.txt");
    shell.execute("ln -s /tmp/target.txt /tmp/link.txt");
    let out = shell.execute("cat /tmp/link.txt");
    assert!(out.contains("-> /tmp/target.txt"));
}

/// `ln` (hard link) should copy the file content.
#[test]
fn test_ln_hard() {
    let mut shell = new_shell();
    shell.execute("echo data > /tmp/src.txt");
    shell.execute("ln /tmp/src.txt /tmp/copy.txt");
    let out = shell.execute("cat /tmp/copy.txt");
    assert!(out.contains("data"));
}

// =========================================================================
// Permissions (simulated)
// =========================================================================

/// `chmod` with a valid octal mode should succeed silently.
#[test]
fn test_chmod_valid() {
    let mut shell = new_shell();
    shell.execute("touch /tmp/f.txt");
    let out = shell.execute("chmod 755 /tmp/f.txt");
    assert!(out.is_empty()); // no error
}

/// `chmod` with an invalid mode should return an error message.
#[test]
fn test_chmod_invalid_mode() {
    let mut shell = new_shell();
    shell.execute("touch /tmp/f.txt");
    let out = shell.execute("chmod invalid /tmp/f.txt");
    assert!(out.contains("invalid mode"));
}

/// `chown` with a valid owner should succeed silently.
#[test]
fn test_chown_valid() {
    let mut shell = new_shell();
    let out = shell.execute("chown alice /tmp/f.txt");
    assert!(out.is_empty()); // no error
}

// =========================================================================
// Documentation
// =========================================================================

/// `man ls` should display the manual page with SYNOPSIS section.
#[test]
fn test_man_known_command() {
    let mut shell = new_shell();
    let out = shell.execute("man ls");
    assert!(out.contains("LS(1)"));
    assert!(out.contains("SYNOPSIS"));
}

/// `man` on an unknown command should report "no entry".
#[test]
fn test_man_unknown_command() {
    let mut shell = new_shell();
    let out = shell.execute("man nonexistent");
    assert!(out.contains("no entry"));
}

/// `help` should list all available commands.
#[test]
fn test_help_lists_commands() {
    let mut shell = new_shell();
    let out = shell.execute("help");
    assert!(out.contains("Available commands"));
    assert!(out.contains("ls"));
    assert!(out.contains("grep"));
    assert!(out.contains("diff"));
    assert!(out.contains("man"));
}

// =========================================================================
// Terminal commands
// =========================================================================

/// `clear` should return the ANSI escape sequence for screen clearing.
#[test]
fn test_clear_returns_escape_sequence() {
    let mut shell = new_shell();
    let out = shell.execute("clear");
    assert!(out.contains("\x1b[2J"));
}

/// `exit` should return an empty string (no-op in this environment).
#[test]
fn test_exit_returns_empty() {
    let mut shell = new_shell();
    let out = shell.execute("exit");
    assert!(out.is_empty());
}

/// An unrecognised command should return "command not found".
#[test]
fn test_unknown_command() {
    let mut shell = new_shell();
    let out = shell.execute("foobar");
    assert!(out.contains("command not found"));
}

/// An empty input string should produce no output.
#[test]
fn test_empty_input() {
    let mut shell = new_shell();
    let out = shell.execute("");
    assert!(out.is_empty());
}

// =========================================================================
// Piping
// =========================================================================

/// `cat file | grep pattern` should filter lines through the pipe.
#[test]
fn test_pipe_cat_grep() {
    let mut shell = new_shell();
    shell.execute("echo hello > /tmp/f.txt");
    shell.execute("echo world >> /tmp/f.txt");
    shell.execute("echo hello again >> /tmp/f.txt");
    let out = shell.execute("cat /tmp/f.txt | grep hello");
    assert!(out.contains("hello"));
    assert!(!out.contains("world"));
}

/// `sort | uniq -c` should sort and count unique lines.
#[test]
fn test_pipe_sort_uniq() {
    let mut shell = new_shell();
    shell.execute("echo b > /tmp/f.txt");
    shell.execute("echo a >> /tmp/f.txt");
    shell.execute("echo b >> /tmp/f.txt");
    shell.execute("echo a >> /tmp/f.txt");
    let out = shell.execute("sort /tmp/f.txt | uniq -c");
    assert!(out.contains("2 a"));
    assert!(out.contains("2 b"));
}

/// `echo ... | grep pattern` should filter piped text.
#[test]
fn test_pipe_echo_grep() {
    let mut shell = new_shell();
    let out = shell.execute("echo hello world | grep hello");
    assert!(out.contains("hello"));
}

/// `echo ... | wc -w` should count words from piped input.
#[test]
fn test_pipe_echo_wc() {
    let mut shell = new_shell();
    let out = shell.execute("echo hello world | wc -w");
    assert!(out.contains("2"));
}

/// Multi-stage pipeline (`sort | uniq | wc -l`) should chain correctly.
#[test]
fn test_pipe_multi_stage() {
    let mut shell = new_shell();
    shell.execute("echo banana > /tmp/f.txt");
    shell.execute("echo apple >> /tmp/f.txt");
    shell.execute("echo cherry >> /tmp/f.txt");
    shell.execute("echo banana >> /tmp/f.txt");
    let out = shell.execute("sort /tmp/f.txt | uniq | wc -l");
    // After sort: apple banana banana cherry; after uniq: apple banana cherry; wc -l = 3
    assert!(out.contains("3"));
}

// =========================================================================
// Redirection with pipes
// =========================================================================

/// `cmd | cmd > file` should redirect the final pipeline output to a file.
#[test]
fn test_pipe_redirect_to_file() {
    let mut shell = new_shell();
    shell.execute("echo hello > /tmp/src.txt");
    shell.execute("echo world >> /tmp/src.txt");
    shell.execute("cat /tmp/src.txt | grep hello > /tmp/result.txt");
    let out = shell.execute("cat /tmp/result.txt");
    assert!(out.contains("hello"));
    assert!(!out.contains("world"));
}

/// `>>` should append to a file rather than overwrite.
#[test]
fn test_redirect_append() {
    let mut shell = new_shell();
    shell.execute("echo first > /tmp/out.txt");
    shell.execute("echo second >> /tmp/out.txt");
    let out = shell.execute("cat /tmp/out.txt");
    assert!(out.contains("first"));
    assert!(out.contains("second"));
}

// =========================================================================
// Chaining with &&
// =========================================================================

/// `cmd1 && cmd2 && cmd3` should execute all commands when each succeeds.
#[test]
fn test_and_chain_success() {
    let mut shell = new_shell();
    let out = shell.execute("mkdir /tmp/new && touch /tmp/new/f.txt && echo done");
    assert!(out.contains("done"));
    assert!(shell.vfs.exists("/tmp/new/f.txt"));
}

/// `&&` chaining should stop at the first failing command.
#[test]
fn test_and_chain_stops_on_error() {
    let mut shell = new_shell();
    let out = shell.execute("rm /nonexistent && echo should_not_appear");
    assert!(!out.contains("should_not_appear"));
}

/// `&&` chains can contain piped commands.
#[test]
fn test_and_chain_with_pipes() {
    let mut shell = new_shell();
    shell.execute("echo hello > /tmp/f.txt");
    let out = shell.execute("cat /tmp/f.txt | grep hello && echo success");
    assert!(out.contains("hello"));
    assert!(out.contains("success"));
}

// =========================================================================
// Prompt
// =========================================================================

/// The prompt should contain the username, hostname, and current directory.
#[test]
fn test_prompt_contains_user_and_host() {
    let shell = new_shell();
    let prompt = shell.get_prompt();
    assert!(prompt.contains("user"));
    assert!(prompt.contains("nexos"));
    assert!(prompt.contains("/"));
}

/// The prompt should update when the working directory changes.
#[test]
fn test_prompt_reflects_cwd() {
    let mut shell = new_shell();
    shell.execute("cd /home/user");
    let prompt = shell.get_prompt();
    assert!(prompt.contains("/home/user"));
}

// =========================================================================
// Tab completion
// =========================================================================

/// Tab completion with a partial prefix should return all matching commands.
#[test]
fn test_completions_partial_match() {
    let shell = new_shell();
    let completions = shell.get_completions("he");
    assert!(completions.contains(&"help".to_string()));
    assert!(completions.contains(&"head".to_string()));
}

/// Tab completion with no matching prefix should return an empty list.
#[test]
fn test_completions_no_match() {
    let shell = new_shell();
    let completions = shell.get_completions("zzz");
    assert!(completions.is_empty());
}

/// Tab completion with an exact command name should return just that command.
#[test]
fn test_completions_exact_match() {
    let shell = new_shell();
    let completions = shell.get_completions("ls");
    assert_eq!(completions, vec!["ls"]);
}

// =========================================================================
// VFS persistence (JSON roundtrip via shell)
// =========================================================================

/// Serialising the VFS to JSON and restoring it should preserve all data
/// and the current working directory.
#[test]
fn test_vfs_persistence_via_shell() {
    let mut shell = new_shell();
    shell.execute("mkdir /home/user/project");
    shell.execute("echo data > /home/user/project/file.txt");
    shell.execute("cd /home/user/project");

    let json = shell.vfs.to_json();
    let restored_vfs = Vfs::from_json(&json).unwrap();
    let mut new_shell = Shell::new(restored_vfs);

    let out = new_shell.execute("cat file.txt");
    assert!(out.contains("data"));
    assert_eq!(new_shell.vfs.cwd, "/home/user/project");
}

// =========================================================================
// Additional integration tests
// =========================================================================

// -- tr via pipe ----------------------------------------------------------

/// `echo ... | tr old new` should translate characters via pipe.
#[test]
fn test_tr_via_pipe() {
    let mut shell = new_shell();
    let out = shell.execute("echo hello | tr h H");
    assert!(out.contains("Hello"));
}

/// `tr` with multiple characters should translate each independently.
#[test]
fn test_tr_translate_multiple_chars() {
    let mut shell = new_shell();
    let out = shell.execute("echo abc | tr ac XY");
    assert!(out.contains("XbY"));
}

// -- tee via pipe ---------------------------------------------------------

/// `tee` should write to a file and also pass data through to stdout.
#[test]
fn test_tee_writes_and_outputs() {
    let mut shell = new_shell();
    let out = shell.execute("echo piped | tee /tmp/tee_out.txt");
    assert!(out.contains("piped"));
    let file_content = shell.execute("cat /tmp/tee_out.txt");
    assert!(file_content.contains("piped"));
}

/// `tee -a` should append to the file rather than overwrite.
#[test]
fn test_tee_append_mode() {
    let mut shell = new_shell();
    shell.execute("echo first > /tmp/tee_log.txt");
    shell.execute("echo second | tee -a /tmp/tee_log.txt");
    let content = shell.execute("cat /tmp/tee_log.txt");
    assert!(content.contains("first"));
    assert!(content.contains("second"));
}

// -- ls -------------------------------------------------------------------

/// `ls` on a directory should list its contents.
#[test]
fn test_ls_direct() {
    let mut shell = new_shell();
    shell.execute("touch /tmp/ls_test.txt");
    shell.execute("mkdir /tmp/ls_dir");
    let out = shell.execute("ls /tmp");
    assert!(out.contains("ls_test.txt"));
    assert!(out.contains("ls_dir"));
}

/// `ls -l` should show type indicators and names.
#[test]
fn test_ls_long_format() {
    let mut shell = new_shell();
    shell.execute("touch /tmp/ls_l.txt");
    shell.execute("mkdir /tmp/ls_d");
    let out = shell.execute("ls -l /tmp");
    assert!(out.contains("- ls_l.txt"));
    assert!(out.contains("d ls_d/"));
}

/// `ls` on a single file should show just that file's name.
#[test]
fn test_ls_single_file() {
    let mut shell = new_shell();
    shell.execute("touch /tmp/single.txt");
    let out = shell.execute("ls /tmp/single.txt");
    assert!(out.contains("single.txt"));
}

// -- grep edge cases ------------------------------------------------------

/// `grep` with multiple files should prefix matches with the filename.
#[test]
fn test_grep_multiple_files_shows_filename() {
    let mut shell = new_shell();
    shell.execute("echo hello > /tmp/ga.txt");
    shell.execute("echo hello > /tmp/gb.txt");
    let out = shell.execute("grep hello /tmp/ga.txt /tmp/gb.txt");
    assert!(out.contains("/tmp/ga.txt:"));
    assert!(out.contains("/tmp/gb.txt:"));
}

/// `grep -in` should combine case-insensitive and line-number flags.
#[test]
fn test_grep_combined_in_flag() {
    let mut shell = new_shell();
    shell.execute("Hello > /tmp/gi.txt");
    shell.execute("echo Hello > /tmp/gi.txt");
    let out = shell.execute("grep -in hello /tmp/gi.txt");
    assert!(out.contains("1:"));
}

/// `grep` with no matching lines should return empty output.
#[test]
fn test_grep_no_match() {
    let mut shell = new_shell();
    shell.execute("echo hello > /tmp/gn.txt");
    let out = shell.execute("grep xyz /tmp/gn.txt");
    assert!(out.trim().is_empty());
}

// -- wc specific flags ----------------------------------------------------

/// `wc -w` should report only the word count.
#[test]
fn test_wc_words_only() {
    let mut shell = new_shell();
    shell.execute("echo one two three > /tmp/ww.txt");
    let out = shell.execute("wc -w /tmp/ww.txt");
    assert!(out.contains("3"));
}

// -- echo edge cases ------------------------------------------------------

/// `echo` with no arguments should print an empty line.
#[test]
fn test_echo_no_args() {
    let mut shell = new_shell();
    let out = shell.execute("echo");
    assert_eq!(out.trim(), "");
}

// -- cat edge cases -------------------------------------------------------

/// `cat` on a nonexistent file should return an error.
#[test]
fn test_cat_nonexistent_file() {
    let mut shell = new_shell();
    let out = shell.execute("cat /nonexistent");
    assert!(out.contains("No such file") || out.contains("no such file"));
}

// -- find edge cases ------------------------------------------------------

/// `find` with no matching files should return empty output.
#[test]
fn test_find_no_results() {
    let mut shell = new_shell();
    shell.execute("mkdir /tmp/empty_dir");
    let out = shell.execute("find /tmp/empty_dir -name nothing");
    assert!(out.trim().is_empty());
}

// -- export edge cases ----------------------------------------------------

/// `export` should handle values containing `=` signs correctly.
#[test]
fn test_export_value_with_equals() {
    let mut shell = new_shell();
    shell.execute("export KEY=val=ue");
    let out = shell.execute("env");
    assert!(out.contains("KEY=val=ue"));
}

// -- pipe with grep from file ---------------------------------------------

/// Piped grep from a file should correctly filter lines.
#[test]
fn test_pipe_grep_from_file() {
    let mut shell = new_shell();
    shell.execute("echo hello > /tmp/pg.txt");
    shell.execute("echo world >> /tmp/pg.txt");
    shell.execute("echo hello again >> /tmp/pg.txt");
    let out = shell.execute("cat /tmp/pg.txt | grep hello");
    assert!(out.contains("hello"));
    assert!(!out.contains("world"));
}

// -- multi-stage pipes ----------------------------------------------------

/// Three-stage pipeline (`sort | uniq | wc -l`) should chain correctly.
#[test]
fn test_pipe_sort_uniq_wc() {
    let mut shell = new_shell();
    shell.execute("echo b > /tmp/ms.txt");
    shell.execute("echo a >> /tmp/ms.txt");
    shell.execute("echo b >> /tmp/ms.txt");
    shell.execute("echo a >> /tmp/ms.txt");
    shell.execute("echo c >> /tmp/ms.txt");
    let out = shell.execute("sort /tmp/ms.txt | uniq | wc -l");
    assert!(out.contains("3"));
}

// -- && chain edge cases --------------------------------------------------

/// Three-command `&&` chain should execute all commands.
#[test]
fn test_and_chain_three_commands() {
    let mut shell = new_shell();
    let out = shell.execute("echo a && echo b && echo c");
    assert!(out.contains("a"));
    assert!(out.contains("b"));
    assert!(out.contains("c"));
}

/// `&&` chain where the first command fails should not run subsequent commands.
#[test]
fn test_and_chain_first_fails() {
    let mut shell = new_shell();
    let out = shell.execute("rm /nonexistent && echo should_not_run");
    assert!(!out.contains("should_not_run"));
}

// -- error handling -------------------------------------------------------

/// `tr` with missing arguments should return an error message.
#[test]
fn test_tr_missing_args() {
    let mut shell = new_shell();
    let out = shell.execute("echo hello | tr a");
    assert!(out.contains("missing operand") || out.contains("error"));
}

/// `tee` without a filename argument should return an error.
#[test]
fn test_tee_missing_file() {
    let mut shell = new_shell();
    let out = shell.execute("echo data | tee");
    assert!(out.contains("missing") || out.contains("operand"));
}

// -- diff edge cases ------------------------------------------------------

/// `diff` between a file and an empty file should show differences.
#[test]
fn test_diff_one_empty() {
    let mut shell = new_shell();
    shell.execute("echo content > /tmp/d1.txt");
    shell.execute("touch /tmp/d2.txt");
    let out = shell.execute("diff /tmp/d1.txt /tmp/d2.txt");
    assert!(!out.is_empty());
}

// -- cp directory ---------------------------------------------------------

/// `cp file dir/` should copy the file into the directory.
#[test]
fn test_cp_into_existing_dir() {
    let mut shell = new_shell();
    shell.execute("echo data > /tmp/cp_src.txt");
    shell.execute("mkdir /tmp/cp_dest");
    shell.execute("cp /tmp/cp_src.txt /tmp/cp_dest");
    let out = shell.execute("cat /tmp/cp_dest/cp_src.txt");
    assert!(out.contains("data"));
}
