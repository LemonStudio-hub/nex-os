//! man command - display manual page for a command

/// Execute the `man` command.
///
/// Usage: `man <command>`
///
/// Displays a detailed manual page for the given command, including
/// synopsis, description, and examples.
pub fn execute(args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err("man: what manual page do you want?\nFor example, try 'man ls'".to_string());
    }

    let cmd = args[0];
    let page = get_manual(cmd);
    Ok(page)
}

/// Return the manual page text for a command.
fn get_manual(cmd: &str) -> String {
    match cmd {
        "ls" => "LS(1)\n\nNAME\n    ls - list directory contents\n\nSYNOPSIS\n    ls [-l] [path]\n\nDESCRIPTION\n    List information about files and directories in the given path.\n    If no path is given, the current directory is used.\n\n    -l    Use long listing format (shows type, size)\n\nEXAMPLES\n    ls\n    ls -l /home\n    ls -l ..".to_string(),

        "cd" => "CD(1)\n\nNAME\n    cd - change the working directory\n\nSYNOPSIS\n    cd [path]\n\nDESCRIPTION\n    Change the current working directory to the given path.\n    With no arguments, changes to the home directory (~).\n\n    Special paths:\n    ~     Home directory (/home/user)\n    ..    Parent directory\n    /     Root directory\n\nEXAMPLES\n    cd /tmp\n    cd ..\n    cd ~".to_string(),

        "pwd" => "PWD(1)\n\nNAME\n    pwd - print name of current working directory\n\nSYNOPSIS\n    pwd\n\nDESCRIPTION\n    Outputs the full path of the current working directory.".to_string(),

        "mkdir" => "MKDIR(1)\n\nNAME\n    mkdir - create directories\n\nSYNOPSIS\n    mkdir [-p] <directory> [directory2 ...]\n\nDESCRIPTION\n    Create one or more directories.\n\n    -p    Create parent directories as needed\n\nEXAMPLES\n    mkdir test\n    mkdir -p path/to/dir".to_string(),

        "touch" => "TOUCH(1)\n\nNAME\n    touch - create empty files\n\nSYNOPSIS\n    touch <file> [file2 ...]\n\nDESCRIPTION\n    Create one or more empty files. If the file already exists,\n    it is not modified.\n\nEXAMPLES\n    touch newfile.txt\n    touch a.txt b.txt c.txt".to_string(),

        "rm" => "RM(1)\n\nNAME\n    rm - remove files or directories\n\nSYNOPSIS\n    rm [-r] <target> [target2 ...]\n\nDESCRIPTION\n    Remove files or directories.\n\n    -r    Remove directories and their contents recursively\n\nEXAMPLES\n    rm file.txt\n    rm -r directory".to_string(),

        "cat" => "CAT(1)\n\nNAME\n    cat - concatenate and display files\n\nSYNOPSIS\n    cat <file> [file2 ...]\n\nDESCRIPTION\n    Read and display the contents of one or more files.\n    When multiple files are given, their contents are concatenated.\n\nEXAMPLES\n    cat file.txt\n    cat a.txt b.txt".to_string(),

        "echo" => "ECHO(1)\n\nNAME\n    echo - display a line of text\n\nSYNOPSIS\n    echo <text> [> file] [>> file]\n\nDESCRIPTION\n    Output the given text. Supports output redirection:\n    >     Write to file (overwrite)\n    >>    Append to file\n\nEXAMPLES\n    echo Hello World\n    echo data > output.txt\n    echo more >> output.txt".to_string(),

        "cp" => "CP(1)\n\nNAME\n    cp - copy files and directories\n\nSYNOPSIS\n    cp <source> <destination>\n\nDESCRIPTION\n    Copy a file or directory to a new location.\n    Directories are copied recursively.\n\nEXAMPLES\n    cp file.txt backup.txt\n    cp -r dir1 dir2".to_string(),

        "mv" => "MV(1)\n\nNAME\n    mv - move (rename) files and directories\n\nSYNOPSIS\n    mv <source> <destination>\n\nDESCRIPTION\n    Move or rename a file or directory.\n\nEXAMPLES\n    mv old.txt new.txt\n    mv file.txt /tmp/".to_string(),

        "tree" => "TREE(1)\n\nNAME\n    tree - list contents in a tree-like format\n\nSYNOPSIS\n    tree [path]\n\nDESCRIPTION\n    Display the directory tree structure starting from the given path.\n    Defaults to the current directory.\n\nEXAMPLES\n    tree\n    tree /home".to_string(),

        "head" => "HEAD(1)\n\nNAME\n    head - display first lines of a file\n\nSYNOPSIS\n    head [-n COUNT] <file>\n\nDESCRIPTION\n    Output the first part of a file. Default is 10 lines.\n\n    -n COUNT    Number of lines to display\n\nEXAMPLES\n    head file.txt\n    head -n 5 file.txt\n    head -n20 log.txt".to_string(),

        "tail" => "TAIL(1)\n\nNAME\n    tail - display last lines of a file\n\nSYNOPSIS\n    tail [-n COUNT] <file>\n\nDESCRIPTION\n    Output the last part of a file. Default is 10 lines.\n\n    -n COUNT    Number of lines to display\n\nEXAMPLES\n    tail file.txt\n    tail -n 5 file.txt".to_string(),

        "grep" => "GREP(1)\n\nNAME\n    grep - search for patterns in files\n\nSYNOPSIS\n    grep [-i] [-n] <pattern> <file> [file2 ...]\n\nDESCRIPTION\n    Search for lines matching the given pattern in one or more files.\n\n    -i    Case-insensitive matching\n    -n    Show line numbers\n\nEXAMPLES\n    grep hello file.txt\n    grep -i error log.txt\n    grep -n TODO *.rs".to_string(),

        "find" => "FIND(1)\n\nNAME\n    find - search for files by name\n\nSYNOPSIS\n    find [path] -name <pattern>\n\nDESCRIPTION\n    Recursively search for files and directories whose names\n    contain the given pattern.\n\nEXAMPLES\n    find -name README\n    find /home -name .txt\n    find . -name config".to_string(),

        "sort" => "SORT(1)\n\nNAME\n    sort - sort lines of a file\n\nSYNOPSIS\n    sort [-r] <file>\n\nDESCRIPTION\n    Sort lines alphabetically.\n\n    -r    Reverse sort order\n\nEXAMPLES\n    sort file.txt\n    sort -r file.txt".to_string(),

        "uniq" => "UNIQ(1)\n\nNAME\n    uniq - filter adjacent duplicate lines\n\nSYNOPSIS\n    uniq [-c] <file>\n\nDESCRIPTION\n    Filter out adjacent duplicate lines.\n\n    -c    Prefix each line with its occurrence count\n\nEXAMPLES\n    uniq file.txt\n    sort file.txt | uniq -c".to_string(),

        "wc" => "WC(1)\n\nNAME\n    wc - word, line, and character count\n\nSYNOPSIS\n    wc [-l] [-w] [-c] <file> [file2 ...]\n\nDESCRIPTION\n    Count lines, words, and characters in files.\n\n    -l    Lines only\n    -w    Words only\n    -c    Characters only\n\nEXAMPLES\n    wc file.txt\n    wc -l *.txt".to_string(),

        "diff" => "DIFF(1)\n\nNAME\n    diff - compare two files\n\nSYNOPSIS\n    diff <file1> <file2>\n\nDESCRIPTION\n    Compare two files line by line and display the differences.\n\nEXAMPLES\n    diff old.txt new.txt".to_string(),

        "du" => "DU(1)\n\nNAME\n    du - estimate disk usage\n\nSYNOPSIS\n    du [-h] [-s] [path]\n\nDESCRIPTION\n    Estimate file space usage for directories.\n\n    -h    Human-readable sizes (K, M)\n    -s    Display only a total for each argument\n\nEXAMPLES\n    du\n    du -h /home\n    du -s .".to_string(),

        "tr" => "TR(1)\n\nNAME\n    tr - translate characters\n\nSYNOPSIS\n    echo text | tr <set1> <set2>\n\nDESCRIPTION\n    Translate characters from set1 to set2. Reads from stdin (pipe).\n    Supports escape sequences: \\n \\t \\\\\n\nEXAMPLES\n    echo hello | tr a-z A-Z\n    echo hello | tr eo OE".to_string(),

        "cut" => "CUT(1)\n\nNAME\n    cut - extract fields from each line\n\nSYNOPSIS\n    cut -f FIELDS [-d DELIM] <file>\n\nDESCRIPTION\n    Extract specified fields from each line.\n\n    -f FIELDS   Comma-separated field numbers (1-indexed)\n    -d DELIM    Field delimiter (default: tab)\n\nEXAMPLES\n    cut -f 1,3 data.csv\n    cut -f 2 -d , data.csv".to_string(),

        "tee" => "TEE(1)\n\nNAME\n    tee - read from stdin and write to stdout and files\n\nSYNOPSIS\n    echo text | tee [-a] <file> [file2 ...]\n\nDESCRIPTION\n    Read stdin and write it to both stdout and the specified files.\n\n    -a    Append to files instead of overwriting\n\nEXAMPLES\n    echo hello | tee output.txt\n    echo data | tee -a log.txt".to_string(),

        "ln" => "LN(1)\n\nNAME\n    ln - create links\n\nSYNOPSIS\n    ln [-s] <target> <link_name>\n\nDESCRIPTION\n    Create a link to a file.\n\n    -s    Create a symbolic link (contains target path)\n    Without -s, copies the file content (simulated hard link)\n\nEXAMPLES\n    ln file.txt link.txt\n    ln -s /path/to/file symlink".to_string(),

        "chmod" => "CHMOD(1)\n\nNAME\n    chmod - change file permissions (simulated)\n\nSYNOPSIS\n    chmod <mode> <file> [file2 ...]\n\nDESCRIPTION\n    Change file permissions. Accepts octal (755, 644) or\n    symbolic (+x, -w, u+r) modes.\n\nNOTE\n    Permissions are simulated in the VFS.\n\nEXAMPLES\n    chmod 755 script.sh\n    chmod +x script.sh\n    chmod u+r file.txt".to_string(),

        "chown" => "CHOWN(1)\n\nNAME\n    chown - change file ownership (simulated)\n\nSYNOPSIS\n    chown <owner>[:<group>] <file> [file2 ...]\n\nDESCRIPTION\n    Change file ownership. Accepts owner or owner:group format.\n\nNOTE\n    Ownership is simulated in the VFS.\n\nEXAMPLES\n    chown alice file.txt\n    chown alice:staff file.txt".to_string(),

        "whoami" => "WHOAMI(1)\n\nNAME\n    whoami - display current username\n\nSYNOPSIS\n    whoami\n\nDESCRIPTION\n    Print the username of the current user.".to_string(),

        "hostname" => "HOSTNAME(1)\n\nNAME\n    hostname - display system hostname\n\nSYNOPSIS\n    hostname\n\nDESCRIPTION\n    Print the hostname of the virtual system.".to_string(),

        "date" => "DATE(1)\n\nNAME\n    date - display current date and time\n\nSYNOPSIS\n    date\n\nDESCRIPTION\n    Print the current date and time in ISO-8601 format (UTC).".to_string(),

        "history" => "HISTORY(1)\n\nNAME\n    history - display command history\n\nSYNOPSIS\n    history\n\nDESCRIPTION\n    Show a numbered list of previously executed commands.".to_string(),

        "clear" => "CLEAR(1)\n\nNAME\n    clear - clear the terminal screen\n\nSYNOPSIS\n    clear\n\nDESCRIPTION\n    Clear all output from the terminal screen.".to_string(),

        "help" => "HELP(1)\n\nNAME\n    help - display available commands\n\nSYNOPSIS\n    help\n\nDESCRIPTION\n    Show a list of all available commands with brief descriptions.".to_string(),

        "man" => "MAN(1)\n\nNAME\n    man - display manual pages\n\nSYNOPSIS\n    man <command>\n\nDESCRIPTION\n    Display a detailed manual page for the given command.".to_string(),

        "basename" => "BASENAME(1)\n\nNAME\n    basename - strip directory from filename\n\nSYNOPSIS\n    basename <path>\n\nDESCRIPTION\n    Print the last component of a path.\n\nEXAMPLES\n    basename /home/user/file.txt  -> file.txt\n    basename /usr/bin/            -> bin".to_string(),

        "dirname" => "DIRNAME(1)\n\nNAME\n    dirname - strip last component from path\n\nSYNOPSIS\n    dirname <path>\n\nDESCRIPTION\n    Print the directory portion of a path.\n\nEXAMPLES\n    dirname /home/user/file.txt  -> /home/user\n    dirname file.txt             -> .".to_string(),

        _ => format!("man: no manual entry for '{}'", cmd),
    }
}
