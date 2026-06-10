//! Virtual File System module.
//!
//! Provides a tree-structured in-memory filesystem with POSIX-style paths,
//! serialized to JSON for OPFS persistence.

mod node;
mod tree;

pub use node::{DirNode, FileNode, FsNode};
pub use tree::Vfs;
