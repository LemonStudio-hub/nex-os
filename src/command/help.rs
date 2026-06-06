//! help command - display available commands

pub fn execute() -> String {
    let commands = [
        ("ls", "List directory contents (-l for long format)"),
        ("cd", "Change the current directory"),
        ("pwd", "Print the current working directory"),
        ("mkdir", "Create directories (-p for recursive)"),
        ("touch", "Create empty files"),
        ("rm", "Remove files or directories (-r for recursive)"),
        ("cat", "Display file contents"),
        ("echo", "Display a line of text"),
        ("cp", "Copy files or directories"),
        ("mv", "Move or rename files and directories"),
        ("tree", "Display directory tree structure"),
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
