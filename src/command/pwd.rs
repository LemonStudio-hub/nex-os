//! pwd command - print working directory

use crate::vfs::Vfs;

pub fn execute(vfs: &Vfs) -> Result<String, String> {
    Ok(format!("{}\n", vfs.cwd))
}
