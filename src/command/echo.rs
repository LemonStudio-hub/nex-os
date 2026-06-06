//! echo command - display text or write to files

use crate::vfs::Vfs;

pub fn execute(vfs: &mut Vfs, args: &[&str]) -> Result<String, String> {
    // Check for >> or > redirection operators within args
    for i in 0..args.len() {
        if args[i] == ">>" && i + 1 < args.len() {
            let content = args[..i].join(" ");
            let file = args[i + 1];
            let resolved = vfs.resolve_path(file)?;
            let existing = vfs.read_file(&resolved).unwrap_or_default();
            vfs.write_file(&resolved, &format!("{}{}\n", existing, content))?;
            return Ok(String::new());
        }
        if args[i] == ">" && i + 1 < args.len() {
            let content = args[..i].join(" ");
            let file = args[i + 1];
            let resolved = vfs.resolve_path(file)?;
            vfs.write_file(&resolved, &format!("{}\n", content))?;
            return Ok(String::new());
        }
    }

    Ok(format!("{}\n", args.join(" ")))
}
