//! Virtual File System (VFS) module.
//!
//! Provides a tree-structured in-memory filesystem with POSIX-style paths,
//! serialized to JSON for OPFS persistence.
//!
//! # Module layout
//!
//! - [`node`] — data types: `FsNode` (enum), `FileNode`, `DirNode`.
//! - [`tree`]  — the `Vfs` struct with path resolution, CRUD operations,
//!   and JSON round-tripping.
//!
//! # Design notes
//!
//! The VFS is entirely client-side; there is no server.  Every mutation is
//! immediately reflected in memory and periodically serialised to the
//! browser's Origin Private File System (OPFS) so that the user's work
//! survives page reloads.

pub mod host_fs;
pub mod host_fs_wasm;
mod node;
pub mod permissions;
mod tree;
pub mod user_db;

// Re-export the public types so consumers can write `use crate::vfs::Vfs`
// instead of `use crate::vfs::tree::Vfs`.
pub use host_fs::{HostEntry, HostFs};
pub use node::{ChunkedContent, DirNode, FileNode, FsNode};
pub use permissions::{default_dir_meta, default_file_meta, NodeMeta};
pub use tree::{current_timestamp, Vfs};
pub use user_db::UserDatabase;
