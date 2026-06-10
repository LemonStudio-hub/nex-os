//! tree command - display directory tree structure

use crate::vfs::{FsNode, Vfs};

pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    let path = if args.is_empty() { "." } else { args[0] };
    let resolved = vfs.resolve_path(path)?;

    if !vfs.exists(&resolved) {
        return Err(format!("tree: '{}': No such file or directory", path));
    }

    if !vfs.is_dir(&resolved) {
        return Ok(format!("{}\n0 directories, 0 files\n", path));
    }

    let mut dir_count: usize = 0;
    let mut file_count: usize = 0;
    let mut output = String::new();

    // Print root name
    if resolved == "/" {
        output.push('/');
    } else {
        let root_name = resolved
            .rfind('/')
            .map(|i| &resolved[i + 1..])
            .unwrap_or(&resolved);
        output.push_str(root_name);
    }
    output.push('\n');

    build_tree(
        vfs,
        &resolved,
        "",
        &mut output,
        &mut dir_count,
        &mut file_count,
    );

    output.push_str(&format!(
        "\n{} directories, {} files\n",
        dir_count, file_count
    ));
    Ok(output)
}

fn build_tree(
    vfs: &Vfs,
    path: &str,
    prefix: &str,
    output: &mut String,
    dir_count: &mut usize,
    file_count: &mut usize,
) {
    let entries: Vec<FsNode> = match vfs.list_dir(path) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut sorted = entries;
    sorted.sort_by(|a, b| {
        let name_a = match a {
            FsNode::File(f) => &f.name,
            FsNode::Directory(d) => &d.name,
        };
        let name_b = match b {
            FsNode::File(f) => &f.name,
            FsNode::Directory(d) => &d.name,
        };
        name_a.cmp(name_b)
    });

    for (i, entry) in sorted.iter().enumerate() {
        let is_last = i == sorted.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let next_prefix = if is_last { "    " } else { "│   " };

        match entry {
            FsNode::File(f) => {
                output.push_str(prefix);
                output.push_str(connector);
                output.push_str(&f.name);
                output.push('\n');
                *file_count += 1;
            }
            FsNode::Directory(d) => {
                output.push_str(prefix);
                output.push_str(connector);
                output.push_str(&d.name);
                output.push('/');
                output.push('\n');
                *dir_count += 1;

                let child_path = if path == "/" {
                    format!("/{}", d.name)
                } else {
                    format!("{}/{}", path, d.name)
                };

                let child_prefix = format!("{}{}", prefix, next_prefix);
                build_tree(
                    vfs,
                    &child_path,
                    &child_prefix,
                    output,
                    dir_count,
                    file_count,
                );
            }
        }
    }
}

pub struct TreeCommand;

impl super::Command for TreeCommand {
    fn name(&self) -> &'static str { "tree" }
    fn description(&self) -> &'static str { "Display directory tree structure" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.vfs, ctx.args)
    }
}
