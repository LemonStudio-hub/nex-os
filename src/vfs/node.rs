//! VFS node types: files and directories.
//!
//! The filesystem tree is built from [`FsNode`] values.  Each node is either a
//! [`FileNode`] (leaf, stores text content) or a [`DirNode`] (branch, stores a
//! map of child names to nodes).  Nodes are serialisable so the entire tree can
//! be persisted as JSON in the browser's OPFS.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A node in the virtual file system — either a file or a directory.
///
/// This is the fundamental building block of the VFS tree.  The root of the
/// tree is a [`DirNode`], and every descendant is wrapped in this enum so
/// that heterogeneous children can live in the same `HashMap`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FsNode {
    /// A leaf node that stores text content.
    File(FileNode),
    /// A branch node that contains named children.
    Directory(DirNode),
}

/// A file node with plain-text content.
///
/// Files in NexOS are simple text blobs; there is no binary support.
/// The `name` field holds the file's basename (e.g. `"readme.txt"`),
/// while `content` holds the full body as a UTF-8 string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    /// Basename of the file (no path separators).
    pub name: String,
    /// Full text content of the file.
    pub content: String,
}

/// A directory node that contains zero or more named children.
///
/// Children are stored in a `HashMap` keyed by their name, giving O(1)
/// lookup during path resolution.  The `name` field holds the directory's
/// own basename (empty string for the root).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirNode {
    /// Basename of the directory (empty for the root node).
    pub name: String,
    /// Map of child name → child node.
    pub children: HashMap<String, FsNode>,
}

impl FsNode {
    /// Get the basename of this node.
    pub fn name(&self) -> &str {
        match self {
            FsNode::File(f) => &f.name,
            FsNode::Directory(d) => &d.name,
        }
    }

    /// Set the basename of this node.
    pub fn set_name(&mut self, name: String) {
        match self {
            FsNode::File(f) => f.name = name,
            FsNode::Directory(d) => d.name = name,
        }
    }

    /// Returns `true` if this node is a directory.
    pub fn is_dir(&self) -> bool {
        matches!(self, FsNode::Directory(_))
    }
}
