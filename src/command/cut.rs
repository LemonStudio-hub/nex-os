//! cut command - extract sections from each line of a file

use crate::vfs::Vfs;

/// Execute the `cut` command.
///
/// Usage: `cut -f FIELDS [-d DELIM] [file]`
///
/// Extracts specified fields from each line. Fields are 1-indexed.
/// `-f` specifies comma-separated field numbers (e.g. `-f 1,3`).
/// `-d` specifies the field delimiter (default: tab).
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    let mut fields: Vec<usize> = Vec::new();
    let mut delimiter = '\t';
    let mut file_path: Option<&str> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "-f" if i + 1 < args.len() => {
                fields = args[i + 1]
                    .split(',')
                    .filter_map(|s| s.trim().parse::<usize>().ok())
                    .collect();
                i += 2;
            }
            "-d" if i + 1 < args.len() => {
                delimiter = args[i + 1].chars().next().unwrap_or('\t');
                i += 2;
            }
            _ if !args[i].starts_with('-') && file_path.is_none() => {
                file_path = Some(args[i]);
                i += 1;
            }
            _ => return Err(format!("cut: unknown option: {}", args[i])),
        }
    }

    if fields.is_empty() {
        return Err("cut: missing -f argument".to_string());
    }

    let path = file_path.ok_or("cut: missing file operand")?;
    let resolved = vfs.resolve_path(path)?;
    let content = vfs.read_file(&resolved)?;

    let mut output = String::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split(delimiter).collect();
        let selected: Vec<&str> = fields
            .iter()
            .filter_map(|&f| {
                if f >= 1 && f <= parts.len() {
                    Some(parts[f - 1])
                } else {
                    None
                }
            })
            .collect();
        output.push_str(&format!("{}\n", selected.join(&delimiter.to_string())));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vfs_with_content(content: &str) -> Vfs {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/f.txt", content).unwrap();
        vfs
    }

    #[test]
    fn extract_single_field() {
        let vfs = vfs_with_content("a\tb\tc");
        let out = execute(&vfs, &["-f", "2", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("b"));
    }

    #[test]
    fn extract_multiple_fields() {
        let vfs = vfs_with_content("a,b,c\nd,e,f");
        let out = execute(&vfs, &["-f", "1,3", "-d", ",", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("a,c"));
        assert!(out.contains("d,f"));
    }

    #[test]
    fn out_of_range_field_skipped() {
        let vfs = vfs_with_content("a,b");
        let out = execute(&vfs, &["-f", "1,5", "-d", ",", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("a"));
        assert!(!out.contains("5"));
    }

    #[test]
    fn missing_f_flag() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["/tmp/f.txt"]).is_err());
    }

    #[test]
    fn missing_file() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["-f", "1"]).is_err());
    }

    #[test]
    fn default_delimiter_is_tab() {
        let vfs = vfs_with_content("x\ty\tz");
        let out = execute(&vfs, &["-f", "1,3", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("x"));
        assert!(out.contains("z"));
    }
}
