//! VFS node types: files and directories.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A node in the virtual file system (either a file or directory)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FsNode {
    File(FileNode),
    Directory(DirNode),
}

/// A file node with text content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub name: String,
    pub content: String,
}

/// A directory node with children
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirNode {
    pub name: String,
    pub children: HashMap<String, FsNode>,
}
