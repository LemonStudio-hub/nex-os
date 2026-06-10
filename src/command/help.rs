//! help command - display available commands

/// Execute the `help` command.
///
/// Returns a formatted table of all available commands with short descriptions.
pub fn execute() -> String {
    let commands = [
        // Filesystem navigation
        ("ls", "List directory contents (-l for long format)"),
        ("cd", "Change the current directory"),
        ("pwd", "Print the current working directory"),
        ("mkdir", "Create directories (-p for recursive)"),
        ("touch", "Create empty files"),
        ("rm", "Remove files or directories (-r for recursive)"),
        ("cp", "Copy files or directories"),
        ("mv", "Move or rename files and directories"),
        ("ln", "Create links (-s for symbolic)"),
        ("tree", "Display directory tree structure"),
        // File content
        ("cat", "Display file contents"),
        (
            "echo",
            "Display a line of text (supports > and >> redirection)",
        ),
        ("head", "Display first N lines of a file (-n COUNT)"),
        ("tail", "Display last N lines of a file (-n COUNT)"),
        // Text processing
        (
            "grep",
            "Search for patterns in files (-i case-insensitive, -n line numbers)",
        ),
        ("sort", "Sort lines of a file (-r for reverse)"),
        ("uniq", "Filter adjacent duplicate lines (-c for counts)"),
        ("wc", "Count lines, words, and characters (-l -w -c)"),
        ("cut", "Extract fields from each line (-f FIELDS -d DELIM)"),
        (
            "tr",
            "Translate characters from stdin (echo text | tr a-z A-Z)",
        ),
        ("tee", "Write stdin to stdout and files (-a for append)"),
        // Comparison
        ("diff", "Compare two files line by line"),
        // Search
        ("find", "Find files by name (find [path] -name PATTERN)"),
        // Disk usage
        ("du", "Estimate disk usage (-h human-readable, -s summary)"),
        // Permissions & ownership (simulated)
        ("chmod", "Change file permissions (octal or symbolic)"),
        ("chown", "Change file ownership (owner[:group])"),
        // System info
        ("whoami", "Display the current username"),
        ("hostname", "Display the system hostname"),
        ("date", "Display the current date and time"),
        ("history", "Display command history"),
        // Environment
        ("env", "Display environment variables"),
        ("export", "Set environment variables (export KEY=VALUE)"),
        // Path utilities
        ("basename", "Strip directory from filename"),
        ("dirname", "Strip filename from path"),
        // Documentation
        ("man", "Display manual page for a command"),
        // Terminal
        ("clear", "Clear the terminal screen"),
        ("help", "Display this help message"),
        ("exit", "Exit the terminal"),
    ];

    let mut output = String::from("Available commands:\n");
    for (name, desc) in &commands {
        output.push_str(&format!("  {:12} {}\n", name, desc));
    }
    output
}

pub struct HelpCommand;

impl super::Command for HelpCommand {
    fn name(&self) -> &'static str { "help" }
    fn description(&self) -> &'static str { "Display this help message" }
    fn execute(&self, _ctx: &mut super::CommandContext) -> Result<String, String> {
        Ok(execute())
    }
}
