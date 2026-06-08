//! Integration tests for the Shell: command dispatch, piping, redirection,
//! chaining, and error handling.

use web_code::shell::Shell;
use web_code::vfs::Vfs;

/// Helper: create a fresh Shell with default VFS
fn new_shell() -> Shell {
    Shell::new(Vfs::new())
}

/// Helper: create a Shell, seed some files, and return it
#[allow(dead_code)]
fn shell_with_files() -> Shell {
    let mut shell = new_shell();
    shell.execute("mkdir /home/user/project");
    shell.execute("echo Hello World > /home/user/project/greeting.txt");
    shell.execute("echo Line two >> /home/user/project/greeting.txt");
    shell.execute("echo alpha > /home/user/project/letters.txt");
    shell.execute("echo beta >> /home/user/project/letters.txt");
    shell.execute("echo gamma >> /home/user/project/letters.txt");
    shell
}

// =========================================================================
// Filesystem commands
// =========================================================================

#[test]
fn test_pwd_at_root() {
    let mut shell = new_shell();
    let out = shell.execute("pwd");
    assert_eq!(out.trim(), "/");
}

#[test]
fn test_cd_and_pwd() {
    let mut shell = new_shell();
    shell.execute("cd /home/user");
    let out = shell.execute("pwd");
    assert_eq!(out.trim(), "/home/user");
}

#[test]
fn test_cd_tilde() {
    let mut shell = new_shell();
    shell.execute("cd ~");
    let out = shell.execute("pwd");
    assert_eq!(out.trim(), "/home/user");
}

#[test]
fn test_cd_dotdot() {
    let mut shell = new_shell();
    shell.execute("cd /home/user");
    shell.execute("cd ..");
    let out = shell.execute("pwd");
    assert_eq!(out.trim(), "/home");
}

#[test]
fn test_mkdir_and_ls() {
    let mut shell = new_shell();
    shell.execute("mkdir /tmp/testdir");
    let out = shell.execute("ls /tmp");
    assert!(out.contains("testdir"));
}

#[test]
fn test_mkdir_p_recursive() {
    let mut shell = new_shell();
    shell.execute("mkdir -p /tmp/a/b/c");
    assert!(shell.vfs.is_dir("/tmp/a/b/c"));
}

#[test]
fn test_touch_creates_file() {
    let mut shell = new_shell();
    shell.execute("touch /tmp/newfile.txt");
    assert!(shell.vfs.exists("/tmp/newfile.txt"));
}

#[test]
fn test_rm_file() {
    let mut shell = new_shell();
    shell.execute("touch /tmp/to_delete.txt");
    shell.execute("rm /tmp/to_delete.txt");
    assert!(!shell.vfs.exists("/tmp/to_delete.txt"));
}

#[test]
fn test_rm_dir_without_r_fails() {
    let mut shell = new_shell();
    shell.execute("mkdir /tmp/del_dir");
    let out = shell.execute("rm /tmp/del_dir");
    assert!(out.contains("directory") || out.contains("Is a directory"));
}

#[test]
fn test_rm_recursive() {
    let mut shell = new_shell();
    shell.execute("mkdir /tmp/del_dir");
    shell.execute("touch /tmp/del_dir/f.txt");
    shell.execute("rm -r /tmp/del_dir");
    assert!(!shell.vfs.exists("/tmp/del_dir"));
}

#[test]
fn test_cp_file() {
    let mut shell = new_shell();
    shell.execute("echo data > /tmp/src.txt");
    shell.execute("cp /tmp/src.txt /tmp/dst.txt");
    let out = shell.execute("cat /tmp/dst.txt");
    assert!(out.contains("data"));
}

#[test]
fn test_mv_file() {
    let mut shell = new_shell();
    shell.execute("echo data > /tmp/old.txt");
    shell.execute("mv /tmp/old.txt /tmp/new.txt");
    assert!(!shell.vfs.exists("/tmp/old.txt"));
    let out = shell.execute("cat /tmp/new.txt");
    assert!(out.contains("data"));
}

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

#[test]
fn test_echo_basic() {
    let mut shell = new_shell();
    let out = shell.execute("echo Hello World");
    assert_eq!(out.trim(), "Hello World");
}

#[test]
fn test_echo_redirect_overwrite() {
    let mut shell = new_shell();
    shell.execute("echo first > /tmp/out.txt");
    shell.execute("echo second > /tmp/out.txt");
    let out = shell.execute("cat /tmp/out.txt");
    assert!(out.contains("second"));
    assert!(!out.contains("first"));
}

#[test]
fn test_echo_redirect_append() {
    let mut shell = new_shell();
    shell.execute("echo line1 > /tmp/out.txt");
    shell.execute("echo line2 >> /tmp/out.txt");
    let out = shell.execute("cat /tmp/out.txt");
    assert!(out.contains("line1"));
    assert!(out.contains("line2"));
}

#[test]
fn test_cat_multiple_files() {
    let mut shell = new_shell();
    shell.execute("echo AAA > /tmp/a.txt");
    shell.execute("echo BBB > /tmp/b.txt");
    let out = shell.execute("cat /tmp/a.txt /tmp/b.txt");
    assert!(out.contains("AAA"));
    assert!(out.contains("BBB"));
}

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

#[test]
fn test_grep_basic() {
    let mut shell = new_shell();
    shell.execute("echo hello > /tmp/f.txt");
    shell.execute("echo world >> /tmp/f.txt");
    shell.execute("echo hello again >> /tmp/f.txt");
    let out = shell.execute("grep hello /tmp/f.txt");
    assert_eq!(out.lines().count(), 2);
}

#[test]
fn test_grep_case_insensitive() {
    let mut shell = new_shell();
    shell.execute("echo Hello > /tmp/f.txt");
    shell.execute("echo WORLD >> /tmp/f.txt");
    let out = shell.execute("grep -i hello /tmp/f.txt");
    assert!(out.contains("Hello"));
}

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

#[test]
fn test_wc_lines_words_chars() {
    let mut shell = new_shell();
    shell.execute("echo hello world > /tmp/f.txt");
    shell.execute("echo foo bar >> /tmp/f.txt");
    let out = shell.execute("wc /tmp/f.txt");
    assert!(out.contains("2")); // 2 lines
    assert!(out.contains("4")); // 4 words
}

#[test]
fn test_wc_lines_only() {
    let mut shell = new_shell();
    shell.execute("echo a > /tmp/f.txt");
    shell.execute("echo b >> /tmp/f.txt");
    shell.execute("echo c >> /tmp/f.txt");
    let out = shell.execute("wc -l /tmp/f.txt");
    assert!(out.contains("3"));
}

#[test]
fn test_cut_fields() {
    let mut shell = new_shell();
    shell.execute("echo 'a,b,c' > /tmp/f.txt");
    shell.execute("echo 'd,e,f' >> /tmp/f.txt");
    let out = shell.execute("cut -f 1,3 -d , /tmp/f.txt");
    assert!(out.contains("a,c"));
    assert!(out.contains("d,f"));
}

#[test]
fn test_diff_identical_files() {
    let mut shell = new_shell();
    shell.execute("echo same > /tmp/a.txt");
    shell.execute("echo same > /tmp/b.txt");
    let out = shell.execute("diff /tmp/a.txt /tmp/b.txt");
    assert!(out.is_empty());
}

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

#[test]
fn test_whoami() {
    let mut shell = new_shell();
    let out = shell.execute("whoami");
    assert_eq!(out.trim(), "user");
}

#[test]
fn test_hostname() {
    let mut shell = new_shell();
    let out = shell.execute("hostname");
    assert_eq!(out.trim(), "web-code");
}

#[test]
fn test_date_returns_output() {
    let mut shell = new_shell();
    let out = shell.execute("date");
    assert!(!out.is_empty());
}

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

#[test]
fn test_env_shows_defaults() {
    let mut shell = new_shell();
    let out = shell.execute("env");
    assert!(out.contains("USER=user"));
    assert!(out.contains("HOSTNAME=web-code"));
}

#[test]
fn test_export_sets_variable() {
    let mut shell = new_shell();
    shell.execute("export MY_VAR=hello");
    let out = shell.execute("env");
    assert!(out.contains("MY_VAR=hello"));
}

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

#[test]
fn test_basename() {
    let mut shell = new_shell();
    let out = shell.execute("basename /home/user/file.txt");
    assert_eq!(out.trim(), "file.txt");
}

#[test]
fn test_basename_with_suffix() {
    let mut shell = new_shell();
    let out = shell.execute("basename /home/user/file.txt .txt");
    assert_eq!(out.trim(), "file");
}

#[test]
fn test_dirname() {
    let mut shell = new_shell();
    let out = shell.execute("dirname /home/user/file.txt");
    assert_eq!(out.trim(), "/home/user");
}

#[test]
fn test_dirname_no_slash() {
    let mut shell = new_shell();
    let out = shell.execute("dirname file.txt");
    assert_eq!(out.trim(), ".");
}

// =========================================================================
// Link commands
// =========================================================================

#[test]
fn test_ln_symbolic() {
    let mut shell = new_shell();
    shell.execute("echo content > /tmp/target.txt");
    shell.execute("ln -s /tmp/target.txt /tmp/link.txt");
    let out = shell.execute("cat /tmp/link.txt");
    assert!(out.contains("-> /tmp/target.txt"));
}

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

#[test]
fn test_chmod_valid() {
    let mut shell = new_shell();
    shell.execute("touch /tmp/f.txt");
    let out = shell.execute("chmod 755 /tmp/f.txt");
    assert!(out.is_empty()); // no error
}

#[test]
fn test_chmod_invalid_mode() {
    let mut shell = new_shell();
    shell.execute("touch /tmp/f.txt");
    let out = shell.execute("chmod invalid /tmp/f.txt");
    assert!(out.contains("invalid mode"));
}

#[test]
fn test_chown_valid() {
    let mut shell = new_shell();
    let out = shell.execute("chown alice /tmp/f.txt");
    assert!(out.is_empty()); // no error
}

// =========================================================================
// Documentation
// =========================================================================

#[test]
fn test_man_known_command() {
    let mut shell = new_shell();
    let out = shell.execute("man ls");
    assert!(out.contains("LS(1)"));
    assert!(out.contains("SYNOPSIS"));
}

#[test]
fn test_man_unknown_command() {
    let mut shell = new_shell();
    let out = shell.execute("man nonexistent");
    assert!(out.contains("no manual entry"));
}

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

#[test]
fn test_clear_returns_escape_sequence() {
    let mut shell = new_shell();
    let out = shell.execute("clear");
    assert!(out.contains("\x1b[2J"));
}

#[test]
fn test_exit_returns_empty() {
    let mut shell = new_shell();
    let out = shell.execute("exit");
    assert!(out.is_empty());
}

#[test]
fn test_unknown_command() {
    let mut shell = new_shell();
    let out = shell.execute("foobar");
    assert!(out.contains("command not found"));
}

#[test]
fn test_empty_input() {
    let mut shell = new_shell();
    let out = shell.execute("");
    assert!(out.is_empty());
}

// =========================================================================
// Piping
// =========================================================================

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

#[test]
fn test_pipe_echo_grep() {
    let mut shell = new_shell();
    let out = shell.execute("echo hello world | grep hello");
    assert!(out.contains("hello"));
}

#[test]
fn test_pipe_echo_wc() {
    let mut shell = new_shell();
    let out = shell.execute("echo hello world | wc -w");
    assert!(out.contains("2"));
}

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

#[test]
fn test_and_chain_success() {
    let mut shell = new_shell();
    let out = shell.execute("mkdir /tmp/new && touch /tmp/new/f.txt && echo done");
    assert!(out.contains("done"));
    assert!(shell.vfs.exists("/tmp/new/f.txt"));
}

#[test]
fn test_and_chain_stops_on_error() {
    let mut shell = new_shell();
    let out = shell.execute("rm /nonexistent && echo should_not_appear");
    assert!(!out.contains("should_not_appear"));
}

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

#[test]
fn test_prompt_contains_user_and_host() {
    let shell = new_shell();
    let prompt = shell.get_prompt();
    assert!(prompt.contains("user"));
    assert!(prompt.contains("web-code"));
    assert!(prompt.contains("/"));
}

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

#[test]
fn test_completions_partial_match() {
    let shell = new_shell();
    let completions = shell.get_completions("he");
    assert!(completions.contains(&"help".to_string()));
    assert!(completions.contains(&"head".to_string()));
}

#[test]
fn test_completions_no_match() {
    let shell = new_shell();
    let completions = shell.get_completions("zzz");
    assert!(completions.is_empty());
}

#[test]
fn test_completions_exact_match() {
    let shell = new_shell();
    let completions = shell.get_completions("ls");
    assert_eq!(completions, vec!["ls"]);
}

// =========================================================================
// VFS persistence (JSON roundtrip via shell)
// =========================================================================

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

#[test]
fn test_tr_via_pipe() {
    let mut shell = new_shell();
    let out = shell.execute("echo hello | tr h H");
    assert!(out.contains("Hello"));
}

#[test]
fn test_tr_translate_multiple_chars() {
    let mut shell = new_shell();
    let out = shell.execute("echo abc | tr ac XY");
    assert!(out.contains("XbY"));
}

// -- tee via pipe ---------------------------------------------------------

#[test]
fn test_tee_writes_and_outputs() {
    let mut shell = new_shell();
    let out = shell.execute("echo piped | tee /tmp/tee_out.txt");
    assert!(out.contains("piped"));
    let file_content = shell.execute("cat /tmp/tee_out.txt");
    assert!(file_content.contains("piped"));
}

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

#[test]
fn test_ls_direct() {
    let mut shell = new_shell();
    shell.execute("touch /tmp/ls_test.txt");
    shell.execute("mkdir /tmp/ls_dir");
    let out = shell.execute("ls /tmp");
    assert!(out.contains("ls_test.txt"));
    assert!(out.contains("ls_dir"));
}

#[test]
fn test_ls_long_format() {
    let mut shell = new_shell();
    shell.execute("touch /tmp/ls_l.txt");
    shell.execute("mkdir /tmp/ls_d");
    let out = shell.execute("ls -l /tmp");
    assert!(out.contains("- ls_l.txt"));
    assert!(out.contains("d ls_d/"));
}

#[test]
fn test_ls_single_file() {
    let mut shell = new_shell();
    shell.execute("touch /tmp/single.txt");
    let out = shell.execute("ls /tmp/single.txt");
    assert!(out.contains("single.txt"));
}

// -- grep edge cases ------------------------------------------------------

#[test]
fn test_grep_multiple_files_shows_filename() {
    let mut shell = new_shell();
    shell.execute("echo hello > /tmp/ga.txt");
    shell.execute("echo hello > /tmp/gb.txt");
    let out = shell.execute("grep hello /tmp/ga.txt /tmp/gb.txt");
    assert!(out.contains("/tmp/ga.txt:"));
    assert!(out.contains("/tmp/gb.txt:"));
}

#[test]
fn test_grep_combined_in_flag() {
    let mut shell = new_shell();
    shell.execute("Hello > /tmp/gi.txt");
    shell.execute("echo Hello > /tmp/gi.txt");
    let out = shell.execute("grep -in hello /tmp/gi.txt");
    assert!(out.contains("1:"));
}

#[test]
fn test_grep_no_match() {
    let mut shell = new_shell();
    shell.execute("echo hello > /tmp/gn.txt");
    let out = shell.execute("grep xyz /tmp/gn.txt");
    assert!(out.trim().is_empty());
}

// -- wc specific flags ----------------------------------------------------

#[test]
fn test_wc_words_only() {
    let mut shell = new_shell();
    shell.execute("echo one two three > /tmp/ww.txt");
    let out = shell.execute("wc -w /tmp/ww.txt");
    assert!(out.contains("3"));
}

// -- echo edge cases ------------------------------------------------------

#[test]
fn test_echo_no_args() {
    let mut shell = new_shell();
    let out = shell.execute("echo");
    assert_eq!(out.trim(), "");
}

// -- cat edge cases -------------------------------------------------------

#[test]
fn test_cat_nonexistent_file() {
    let mut shell = new_shell();
    let out = shell.execute("cat /nonexistent");
    assert!(out.contains("No such file") || out.contains("no such file"));
}

// -- find edge cases ------------------------------------------------------

#[test]
fn test_find_no_results() {
    let mut shell = new_shell();
    shell.execute("mkdir /tmp/empty_dir");
    let out = shell.execute("find /tmp/empty_dir -name nothing");
    assert!(out.trim().is_empty());
}

// -- export edge cases ----------------------------------------------------

#[test]
fn test_export_value_with_equals() {
    let mut shell = new_shell();
    shell.execute("export KEY=val=ue");
    let out = shell.execute("env");
    assert!(out.contains("KEY=val=ue"));
}

// -- pipe with grep from file ---------------------------------------------

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

#[test]
fn test_and_chain_three_commands() {
    let mut shell = new_shell();
    let out = shell.execute("echo a && echo b && echo c");
    assert!(out.contains("a"));
    assert!(out.contains("b"));
    assert!(out.contains("c"));
}

#[test]
fn test_and_chain_first_fails() {
    let mut shell = new_shell();
    let out = shell.execute("rm /nonexistent && echo should_not_run");
    assert!(!out.contains("should_not_run"));
}

// -- error handling -------------------------------------------------------

#[test]
fn test_tr_missing_args() {
    let mut shell = new_shell();
    let out = shell.execute("echo hello | tr a");
    assert!(out.contains("missing operand") || out.contains("error"));
}

#[test]
fn test_tee_missing_file() {
    let mut shell = new_shell();
    let out = shell.execute("echo data | tee");
    assert!(out.contains("missing") || out.contains("operand"));
}

// -- diff edge cases ------------------------------------------------------

#[test]
fn test_diff_one_empty() {
    let mut shell = new_shell();
    shell.execute("echo content > /tmp/d1.txt");
    shell.execute("touch /tmp/d2.txt");
    let out = shell.execute("diff /tmp/d1.txt /tmp/d2.txt");
    assert!(!out.is_empty());
}

// -- cp directory ---------------------------------------------------------

#[test]
fn test_cp_into_existing_dir() {
    let mut shell = new_shell();
    shell.execute("echo data > /tmp/cp_src.txt");
    shell.execute("mkdir /tmp/cp_dest");
    shell.execute("cp /tmp/cp_src.txt /tmp/cp_dest");
    let out = shell.execute("cat /tmp/cp_dest/cp_src.txt");
    assert!(out.contains("data"));
}
