//! `man` command -- display manual pages for built-in commands.
//!
//! # Usage
//!
//! ```text
//! man <command>
//! ```
//!
//! # Description
//!
//! Prints a detailed manual page for the specified command, including its
//! **NAME**, **SYNOPSIS**, **DESCRIPTION**, and **EXAMPLES** sections.
//!
//! Manual pages are generated dynamically from the command
//! [`Registry`](super::Registry) — each command's [`Command`](super::Command)
//! trait methods (`name`, `synopsis`, `man_description`, `examples`) provide
//! the content.  Adding a new command automatically produces its man page
//! with zero extra work in this module.
//!
//! # Examples
//!
//! ```text
//! $ man ls
//! LS(1)
//!
//! NAME
//!     ls - list directory contents
//! ...
//!
//! $ man nonexistent
//! man: no manual entry for 'nonexistent'
//! ```

use crate::command::Registry;

/// Execute the `man` command.
///
/// Looks up the command in the registry and generates a man page from
/// its trait metadata.
///
/// # Arguments
///
/// * `registry` — The command registry to search.
/// * `args` — Command-line arguments; `args[0]` is the command to look up.
///
/// # Returns
///
/// `Ok(page)` with the formatted manual text, or `Err` if no command name
/// was given.  Unknown command names still return `Ok` with a "no manual
/// entry" message (matching real `man` behaviour).
pub fn execute(registry: &Registry, args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err("man: what manual page do you want?\nFor example, try 'man ls'".to_string());
    }

    let cmd_name = args[0];

    let command = match registry.get(cmd_name) {
        Some(c) => c,
        None => return Ok(format!("man: no entry for '{}'", cmd_name)),
    };

    Ok(build_man_page(command))
}

/// Format a man page from a command's trait metadata.
///
/// Produces output following Unix man page conventions:
/// `NAME(1)`, `NAME`, `SYNOPSIS`, `DESCRIPTION`, `EXAMPLES`.
fn build_man_page(command: &dyn crate::command::Command) -> String {
    let name = command.name();
    let upper_name = name.to_uppercase();
    let synopsis = command.synopsis();
    let desc = if command.man_description().is_empty() {
        command.description()
    } else {
        command.man_description()
    };
    let examples = command.examples();

    let mut page = format!("{}(1)\n\n", upper_name);
    page.push_str(&format!("NAME\n    {} - {}\n\n", name, command.description()));
    page.push_str(&format!("SYNOPSIS\n    {}\n\n", synopsis));
    page.push_str(&format!("DESCRIPTION\n    {}\n", desc));

    if !examples.is_empty() {
        page.push('\n');
        page.push_str("EXAMPLES\n");
        for ex in examples {
            page.push_str(&format!("    {}\n", ex));
        }
    }

    page
}

/// Unit struct representing the `man` command.
pub struct ManCommand;

impl super::Command for ManCommand {
    fn name(&self) -> &'static str { "man" }
    fn description(&self) -> &'static str { "Display manual page for a command" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.registry, ctx.args)
    }
    fn synopsis(&self) -> &'static str { "man command" }
    fn man_description(&self) -> &'static str {
        "Display a detailed manual page for the specified command, including its name, synopsis, description, and examples. Every registered command has a corresponding manual page."
    }
    fn examples(&self) -> &'static [&'static str] { &["man ls", "man grep"] }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_registry() -> Registry {
        Registry::new()
    }

    #[test]
    fn known_command_has_synopsis() {
        let registry = test_registry();
        let out = execute(&registry, &["ls"]).unwrap();
        assert!(out.contains("LS(1)"));
        assert!(out.contains("SYNOPSIS"));
        assert!(out.contains("NAME"));
        assert!(out.contains("DESCRIPTION"));
    }

    #[test]
    fn unknown_command() {
        let registry = test_registry();
        let out = execute(&registry, &["nonexistent"]).unwrap();
        assert!(out.contains("no entry"));
    }

    #[test]
    fn missing_args() {
        let registry = test_registry();
        assert!(execute(&registry, &[]).is_err());
    }

    #[test]
    fn all_commands_have_pages() {
        let registry = test_registry();
        let commands = [
            "ls", "cd", "pwd", "mkdir", "touch", "rm", "cat", "echo", "cp", "mv", "tree", "head",
            "tail", "grep", "find", "sort", "uniq", "wc", "diff", "du", "tr", "cut", "tee", "ln",
            "chmod", "chown", "whoami", "hostname", "date", "history", "clear", "help", "man",
            "basename", "dirname",
        ];
        for cmd in &commands {
            let out = execute(&registry, &[cmd]).unwrap();
            assert!(
                !out.contains("no entry"),
                "Missing man page for: {}",
                cmd
            );
        }
    }

    #[test]
    fn examples_appear_when_present() {
        let registry = test_registry();
        let out = execute(&registry, &["ls"]).unwrap();
        assert!(out.contains("EXAMPLES"));
    }

    #[test]
    fn no_examples_section_when_empty() {
        let registry = test_registry();
        let out = execute(&registry, &["pwd"]).unwrap();
        assert!(!out.contains("EXAMPLES"));
    }
}
