//! Virtual File System tree operations.
//!
//! Provides the [`Vfs`] struct with POSIX-style path resolution, file/directory
//! CRUD operations, and JSON serialization for OPFS persistence.
//!
//! # Path resolution
//!
//! All public methods accept **absolute** paths (e.g. `"/home/user/file.txt"`).
//! Relative paths and shell expansions (`~`, `.`, `..`) are first normalised
//! by [`Vfs::resolve_path`] before being handed to the internal helpers.
//!
//! # Persistence model
//!
//! The entire tree is serialised to JSON via serde after every command
//! execution.  The frontend writes that JSON to the browser's Origin Private
//! File System (OPFS) so that the user's files survive page reloads.

use super::host_fs::HostFs;
use super::node::{ChunkedContent, DirNode, FileNode, FsNode};
use super::permissions::{default_dir_meta, default_file_meta, NodeMeta};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Get the current timestamp as seconds since Unix epoch.
///
/// On wasm32, uses `Date.now()`. On native targets, returns a fixed value
/// for test determinism.
pub fn current_timestamp() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        // Date.now() returns milliseconds
        (js_sys::Date::now() / 1000.0) as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        1_700_000_000 // Fixed test value
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions (no VFS state access)
// ---------------------------------------------------------------------------

/// Split a path string into its non-empty, slash-separated components.
///
/// Leading and trailing slashes are ignored, so both absolute and relative
/// paths produce the same component list.
fn split_path(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

/// Join path components back into an absolute path string.
///
/// Returns `"/"` when the slice is empty (i.e. the root directory).
fn join_components(components: &[&str]) -> String {
    if components.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", components.join("/"))
    }
}

/// Extract the last component of a path for display or error messages.
///
/// For `"/home/user/file.txt"` this returns `"file.txt"`.
/// For `"/"` it returns `"/"`.
fn path_display_name(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) if i + 1 < path.len() => &path[i + 1..],
        _ => path,
    }
}

// ---------------------------------------------------------------------------
// Vfs implementation
// ---------------------------------------------------------------------------

/// The virtual file system — holds the root directory tree and tracks the
/// current working directory (cwd) as an absolute POSIX path.
///
/// # Thread safety
///
/// NexOS runs in a single-threaded WASM environment, so `Vfs` does not
/// implement `Sync` or `Send`.  All mutations go through `&mut self`.
///
/// # Persistence
///
/// The struct derives `Serialize` / `Deserialize` so it can be round-tripped
/// through JSON and stored in the browser's OPFS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vfs {
    /// The root directory of the filesystem (name is always empty).
    pub root: DirNode,
    /// Absolute path of the current working directory (e.g. `"/home/user"`).
    pub cwd: String,
    /// Paths of files modified or created since the last save.
    #[serde(skip)]
    dirty_files: HashSet<String>,
    /// Paths of files deleted since the last save.
    #[serde(skip)]
    deleted_files: HashSet<String>,
    /// Map of VFS mount path -> host directory name (e.g. `"/mnt/project"` -> `"project"`).
    /// Serialized so mount points survive page reloads. The actual
    /// `FileSystemDirectoryHandle` objects live on the JS side and must be
    /// re-authorized after each reload.
    pub mounts: HashMap<String, String>,
}

impl Default for Vfs {
    fn default() -> Self {
        Self::new()
    }
}

impl Vfs {
    // ---- Construction --------------------------------------------------------

    /// Create a fresh VFS seeded with the standard POSIX top-level directories
    /// (`/home`, `/tmp`, `/etc`, `/var`) and a default `/home/user` directory.
    ///
    /// The cwd is set to `"/"` (root).
    pub fn new() -> Self {
        let now = current_timestamp();

        let mut root = DirNode {
            name: String::new(), // root's name is empty for convenience
            children: HashMap::new(),
            meta: NodeMeta::root_default(now),
        };

        // Helper to create an empty dir with metadata
        fn empty_dir(name: &str, uid: u32, gid: u32, now: u64) -> FsNode {
            FsNode::Directory(DirNode {
                name: name.to_string(),
                children: HashMap::new(),
                meta: NodeMeta::dir_default(uid, gid, now),
            })
        }

        // Default dirs owned by root
        root.children
            .insert("home".to_string(), empty_dir("home", 0, 0, now));
        root.children
            .insert("tmp".to_string(), empty_dir("tmp", 0, 0, now));
        root.children
            .insert("etc".to_string(), empty_dir("etc", 0, 0, now));
        root.children
            .insert("var".to_string(), empty_dir("var", 0, 0, now));

        // /tmp gets sticky bit
        if let Some(FsNode::Directory(ref mut tmp)) = root.children.get_mut("tmp") {
            tmp.meta.mode = 0o1777;
        }

        // Create /home/user owned by uid 1000
        if let Some(FsNode::Directory(ref mut home)) = root.children.get_mut("home") {
            home.children
                .insert("user".to_string(), empty_dir("user", 1000, 1000, now));
        }

        Vfs {
            root,
            cwd: "/".to_string(),
            dirty_files: HashSet::new(),
            deleted_files: HashSet::new(),
            mounts: HashMap::new(),
        }
    }

    // ---- JSON (de)serialization ----------------------------------------------

    /// Serialise the entire VFS tree to a JSON string.
    ///
    /// Used by the frontend to persist state to OPFS after every command.
    /// Returns an error JSON object if serialisation fails (should be rare).
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    }

    /// Deserialise a VFS tree from a JSON string produced by [`to_json`].
    ///
    /// Returns `Err` if the JSON is malformed or does not match the expected
    /// schema.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Failed to parse VFS JSON: {}", e))
    }

    // ---- Path resolution -----------------------------------------------------

    /// Resolve a user-supplied path (absolute or relative) to a canonical
    /// absolute path string.
    ///
    /// Supports the following expansions:
    /// - `~`       → `/home/user`
    /// - `.`       → current directory (no-op)
    /// - `..`      → parent directory
    /// - relative paths are resolved against `self.cwd`
    ///
    /// Returns `Ok(absolute_path)` on success.  Note that the resolved path
    /// does **not** guarantee the target exists — callers should check with
    /// [`exists`] or [`is_dir`] as needed.
    pub fn resolve_path(&self, path: &str) -> Result<String, String> {
        // Reject null bytes to prevent injection attacks
        if path.contains('\0') {
            return Err("path contains null byte".to_string());
        }

        // ~ expansion
        let expanded: String;
        let working_path = if let Some(rest) = path.strip_prefix('~') {
            expanded = format!("/home/user{}", rest);
            &expanded
        } else {
            path
        };

        // Choose starting components: absolute → empty base, relative → cwd base
        let mut components: Vec<String> = if working_path.starts_with('/') {
            Vec::new()
        } else {
            split_path(&self.cwd)
                .iter()
                .map(|s| s.to_string())
                .collect()
        };

        for part in split_path(working_path) {
            match part {
                "." => {}
                ".." => {
                    components.pop();
                }
                _ => components.push(part.to_string()),
            }
        }

        // Build absolute path string
        Ok(join_components(
            &components.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        ))
    }

    // ---- Node access ---------------------------------------------------------

    /// Check whether a node (file or directory) exists at the given absolute
    /// `path`.  The root path `"/"` is always considered to exist.
    pub fn exists(&self, path: &str) -> bool {
        if path == "/" || path.is_empty() {
            return true;
        }
        self.get_node_at(path).is_some()
    }

    /// Check whether the given absolute `path` points to a directory.
    /// The root path `"/"` is always considered a directory.
    pub fn is_dir(&self, path: &str) -> bool {
        if path == "/" || path.is_empty() {
            return true;
        }
        matches!(self.get_node_at(path), Some(FsNode::Directory(_)))
    }

    /// Internal: get an immutable reference to the directory node at `path`.
    ///
    /// Returns the root when `path` is `"/"`.  Returns `None` if the path
    /// does not exist or points to a file.
    fn get_dir(&self, path: &str) -> Option<&DirNode> {
        if path == "/" || path.is_empty() {
            return Some(&self.root);
        }
        let components = split_path(path);
        let mut current = &self.root;
        for comp in &components {
            match current.children.get(*comp) {
                Some(FsNode::Directory(dir)) => current = dir,
                _ => return None,
            }
        }
        Some(current)
    }

    /// Internal: get a mutable reference to the directory node at `path`.
    ///
    /// Returns `None` if the path does not exist or points to a file.
    fn get_dir_mut(&mut self, path: &str) -> Option<&mut DirNode> {
        if path == "/" || path.is_empty() {
            return Some(&mut self.root);
        }
        let components = split_path(path);
        let mut current = &mut self.root;
        for comp in &components {
            match current.children.get_mut(*comp) {
                Some(FsNode::Directory(dir)) => current = dir,
                _ => return None,
            }
        }
        Some(current)
    }

    /// Get the metadata for the node at the given absolute `path`.
    ///
    /// Returns the root metadata when `path` is `"/"`.
    pub fn get_meta(&self, path: &str) -> Option<&NodeMeta> {
        if path == "/" || path.is_empty() {
            return Some(&self.root.meta);
        }
        self.get_node_at(path).map(|n| n.meta())
    }

    /// Get mutable metadata for the node at the given absolute `path`.
    ///
    /// Returns the root metadata when `path` is `"/"`.
    pub fn get_meta_mut(&mut self, path: &str) -> Option<&mut NodeMeta> {
        if path == "/" || path.is_empty() {
            return Some(&mut self.root.meta);
        }
        self.get_node_at_mut(path).map(|n| n.meta_mut())
    }

    // ---- File / directory operations ------------------------------------------

    /// Create a new directory at the given absolute `path`.
    ///
    /// The parent directory must already exist; intermediate directories are
    /// **not** created automatically.  The `mkdir -p` command handles
    /// recursive creation at a higher level.
    ///
    /// Returns `Err` if the parent is missing or a node with the same name
    /// already exists.
    pub fn mkdir(&mut self, path: &str) -> Result<String, String> {
        self.mkdir_with_owner(path, 0, 0)
    }

    /// Create a new directory at `path` owned by the given uid/gid.
    ///
    /// Like [`mkdir`] but sets the new directory's owner.
    pub fn mkdir_with_owner(&mut self, path: &str, uid: u32, gid: u32) -> Result<String, String> {
        let (parent_path, name) = self.split_parent_name(path)?;
        let parent = self.get_dir_mut(&parent_path).ok_or_else(|| {
            format!(
                "cannot create directory '{}': No such file or directory",
                path_display_name(path)
            )
        })?;

        if parent.children.contains_key(&name) {
            return Err(format!(
                "cannot create directory '{}': File exists",
                path_display_name(path)
            ));
        }

        parent.children.insert(
            name.clone(),
            FsNode::Directory(DirNode {
                name,
                children: HashMap::new(),
                meta: NodeMeta::dir_default(uid, gid, current_timestamp()),
            }),
        );
        Ok(String::new())
    }

    /// Create an empty file at the given absolute `path` if it does not already
    /// exist.  If a node already exists at that path, this is a no-op (matching
    /// POSIX `touch` semantics for a VFS without timestamps).
    ///
    /// Returns `Err` if the parent directory does not exist.
    pub fn touch(&mut self, path: &str) -> Result<String, String> {
        self.touch_with_owner(path, 0, 0)
    }

    /// Create an empty file at `path` owned by the given uid/gid.
    ///
    /// Like [`touch`] but sets the new file's owner.  No-op if the file
    /// already exists.
    pub fn touch_with_owner(&mut self, path: &str, uid: u32, gid: u32) -> Result<String, String> {
        if self.exists(path) {
            return Ok(String::new());
        }
        let (parent_path, name) = self.split_parent_name(path)?;
        let parent = self.get_dir_mut(&parent_path).ok_or_else(|| {
            format!(
                "cannot touch '{}': No such file or directory",
                path_display_name(path)
            )
        })?;

        parent.children.insert(
            name.clone(),
            FsNode::File(FileNode {
                name,
                content: ChunkedContent::new(),
                meta: NodeMeta::file_default(uid, gid, current_timestamp()),
            }),
        );
        self.mark_dirty(path);
        Ok(String::new())
    }

    /// Remove the node (file or directory) at the given absolute `path`.
    ///
    /// Non-empty directories **cannot** be removed with this method — use
    /// [`rm_recursive`] for that.  The root path `"/"` can never be removed.
    ///
    /// Returns `Err` if the path does not exist or is a non-empty directory.
    pub fn rm(&mut self, path: &str) -> Result<String, String> {
        self.rm_inner(path, false)
    }

    /// Remove the node at the given absolute `path`, including non-empty
    /// directories and all their descendants.
    ///
    /// This is the backing implementation for `rm -rf`.
    pub fn rm_recursive(&mut self, path: &str) -> Result<String, String> {
        self.rm_inner(path, true)
    }

    /// Internal implementation shared by [`rm`] and [`rm_recursive`].
    ///
    /// When `recursive` is `false`, attempting to remove a non-empty directory
    /// returns an error.  When `true`, the entire subtree is deleted.
    fn rm_inner(&mut self, path: &str, recursive: bool) -> Result<String, String> {
        if path == "/" {
            return Err("cannot remove '/'".to_string());
        }
        let (parent_path, name) = self.split_parent_name(path)?;
        let parent = self.get_dir_mut(&parent_path).ok_or_else(|| {
            format!(
                "cannot remove '{}': No such file or directory",
                path_display_name(path)
            )
        })?;

        if let Some(FsNode::Directory(dir)) = parent.children.get(&name) {
            if !dir.children.is_empty() && !recursive {
                return Err(format!(
                    "cannot remove '{}': Is a directory",
                    path_display_name(path)
                ));
            }
        }

        // Collect file paths for dirty tracking before removing.
        let removed = parent.children.remove(&name).ok_or_else(|| {
            format!(
                "cannot remove '{}': No such file or directory",
                path_display_name(path)
            )
        })?;

        // Mark deleted files for incremental persistence.
        match removed {
            FsNode::File(_) => {
                self.mark_deleted(path);
            }
            FsNode::Directory(dir) => {
                self.mark_deleted_recursive(path, &dir);
            }
        }

        Ok(String::new())
    }

    /// Read and return the text content of the file at the given absolute `path`.
    ///
    /// Returns `Err` if the path does not exist or points to a directory.
    pub fn read_file(&self, path: &str) -> Result<String, String> {
        match self.get_node_at(path) {
            Some(FsNode::File(f)) => Ok(f.content.as_string()),
            Some(FsNode::Directory(_)) => {
                Err(format!("{}: Is a directory", path_display_name(path)))
            }
            None => Err(format!(
                "{}: No such file or directory",
                path_display_name(path)
            )),
        }
    }

    /// Write `content` to the file at the given absolute `path`.
    ///
    /// If the file already exists its content is **overwritten**.  If it does
    /// not exist, a new file is created (the parent directory must exist).
    ///
    /// Returns `Err` if the parent directory does not exist.
    pub fn write_file(&mut self, path: &str, content: &str) -> Result<String, String> {
        self.write_file_with_owner(path, content, 0, 0)
    }

    /// Write `content` to the file at `path`, creating with the given owner if new.
    ///
    /// Like [`write_file`] but sets the owner on newly created files.
    pub fn write_file_with_owner(
        &mut self,
        path: &str,
        content: &str,
        uid: u32,
        gid: u32,
    ) -> Result<String, String> {
        // If file already exists, update in place
        if let Some(FsNode::File(f)) = self.get_node_at_mut(path) {
            f.content = ChunkedContent::from_string(content);
            self.mark_dirty(path);
            return Ok(String::new());
        }

        // Otherwise create it
        let (parent_path, name) = self.split_parent_name(path)?;
        let parent = self.get_dir_mut(&parent_path).ok_or_else(|| {
            format!(
                "cannot create '{}': No such file or directory",
                path_display_name(path)
            )
        })?;

        parent.children.insert(
            name.clone(),
            FsNode::File(FileNode {
                name,
                content: ChunkedContent::from_string(content),
                meta: NodeMeta::file_default(uid, gid, current_timestamp()),
            }),
        );
        self.mark_dirty(path);
        Ok(String::new())
    }

    /// List the immediate children of the directory at the given absolute `path`.
    ///
    /// Returns a `Vec<FsNode>` (files and subdirectories).  The order is
    /// unspecified — callers that need sorting should do so themselves.
    ///
    /// Returns `Err` if the path does not exist or is not a directory.
    pub fn list_dir(&self, path: &str) -> Result<Vec<FsNode>, String> {
        let dir = self.get_dir(path).ok_or_else(|| {
            format!(
                "cannot access '{}': No such file or directory",
                path_display_name(path)
            )
        })?;
        Ok(dir.children.values().cloned().collect())
    }

    /// Copy the file or directory at `src` to `dst`.  Both paths are absolute.
    ///
    /// If `dst` is an existing directory, the source node is copied **into**
    /// it (preserving its original name).  Otherwise the source is copied to
    /// the exact `dst` path, renaming the top-level node as needed.
    ///
    /// Returns `Err` if the source does not exist or the destination parent
    /// is missing.
    pub fn cp(&mut self, src: &str, dst: &str) -> Result<String, String> {
        let node = self
            .get_node_at(src)
            .ok_or_else(|| {
                format!(
                    "cannot stat '{}': No such file or directory",
                    path_display_name(src)
                )
            })?
            .clone();

        // If dst is an existing directory, copy into it
        let actual_dst = if self.is_dir(dst) {
            Self::child_path(dst, path_display_name(src))
        } else {
            dst.to_string()
        };

        // Ensure parent directory exists
        let (parent_path, name) = self.split_parent_name(&actual_dst)?;
        let parent = self.get_dir_mut(&parent_path).ok_or_else(|| {
            format!(
                "cannot create '{}': No such file or directory",
                path_display_name(&actual_dst)
            )
        })?;

        // Rename the top-level node if needed
        let mut new_node = node;
        new_node.set_name(name);

        parent
            .children
            .insert(path_display_name(&actual_dst).to_string(), new_node);
        self.mark_dirty(&actual_dst);
        Ok(String::new())
    }

    /// Move (rename) the file or directory from `src` to `dst`.
    ///
    /// Internally this copies the node to `dst` and then removes the original.
    /// Both paths are absolute.
    ///
    /// Returns `Err` if the source does not exist or the destination parent
    /// is missing.
    pub fn mv(&mut self, src: &str, dst: &str) -> Result<String, String> {
        // Copy then remove source
        self.cp(src, dst)?;
        // Remove the original – need to handle directory removal without -r check
        let (parent_path, name) = self.split_parent_name(src)?;
        if let Some(parent) = self.get_dir_mut(&parent_path) {
            if let Some(removed) = parent.children.remove(&name) {
                match removed {
                    FsNode::File(_) => self.mark_deleted(src),
                    FsNode::Directory(dir) => self.mark_deleted_recursive(src, &dir),
                }
            }
        }
        Ok(String::new())
    }

    // ---- Mount management ----------------------------------------------------

    /// Register a mount point mapping a VFS path to a host directory name.
    pub fn add_mount(&mut self, vfs_path: String, host_path: String) {
        self.mounts.insert(vfs_path, host_path);
    }

    /// Remove a mount point. Returns `true` if it existed.
    pub fn remove_mount(&mut self, vfs_path: &str) -> bool {
        self.mounts.remove(vfs_path).is_some()
    }

    /// Check if a path falls under a mount point.
    ///
    /// Returns `Some((mount_vfs_path, relative_remainder))` if the path
    /// is exactly a mount point or is nested beneath one. The `relative`
    /// portion is the path relative to the mount root (empty string for
    /// the mount root itself).
    pub fn find_mount<'a>(&'a self, path: &'a str) -> Option<(&'a str, &'a str)> {
        // Check exact match first
        if self.mounts.contains_key(path) {
            return Some((path, ""));
        }
        // Check prefix match (path starts with mount + "/")
        for mount_path in self.mounts.keys() {
            if let Some(rest) = path.strip_prefix(mount_path.as_str()) {
                if let Some(stripped) = rest.strip_prefix('/') {
                    return Some((mount_path.as_str(), stripped));
                }
            }
        }
        None
    }

    /// List all active mounts.
    pub fn list_mounts(&self) -> &HashMap<String, String> {
        &self.mounts
    }

    // ---- Host-delegating method variants ------------------------------------

    /// Read a file, delegating to `HostFs` if the path is under a mount point.
    pub fn read_file_with_host(
        &self,
        path: &str,
        host_fs: Option<&dyn HostFs>,
    ) -> Result<String, String> {
        let resolved = self.resolve_path(path)?;
        if let Some(hfs) = host_fs {
            if let Some((_mount, relative)) = self.find_mount(&resolved) {
                return hfs.read_file(relative);
            }
        }
        self.read_file(&resolved)
    }

    /// Read a range of lines, delegating to `HostFs` if mounted.
    pub fn read_file_lines_with_host(
        &self,
        path: &str,
        start: usize,
        count: usize,
        host_fs: Option<&dyn HostFs>,
    ) -> Result<String, String> {
        let resolved = self.resolve_path(path)?;
        if let Some(hfs) = host_fs {
            if let Some((_mount, relative)) = self.find_mount(&resolved) {
                return hfs.read_file_lines(relative, start, count);
            }
        }
        self.read_file_lines(&resolved, start, count)
    }

    /// Return line count, delegating to `HostFs` if mounted.
    pub fn file_line_count_with_host(
        &self,
        path: &str,
        host_fs: Option<&dyn HostFs>,
    ) -> Result<usize, String> {
        let resolved = self.resolve_path(path)?;
        if let Some(hfs) = host_fs {
            if let Some((_mount, relative)) = self.find_mount(&resolved) {
                return hfs.file_line_count(relative);
            }
        }
        self.file_line_count(&resolved)
    }

    /// Return file size, delegating to `HostFs` if mounted.
    pub fn file_size_with_host(
        &self,
        path: &str,
        host_fs: Option<&dyn HostFs>,
    ) -> Result<usize, String> {
        let resolved = self.resolve_path(path)?;
        if let Some(hfs) = host_fs {
            if let Some((_mount, relative)) = self.find_mount(&resolved) {
                return hfs.file_size(relative);
            }
        }
        self.file_size(&resolved)
    }

    /// Write a file, delegating to `HostFs` if mounted.
    pub fn write_file_with_host(
        &mut self,
        path: &str,
        content: &str,
        host_fs: Option<&dyn HostFs>,
    ) -> Result<String, String> {
        let resolved = self.resolve_path(path)?;
        if let Some(hfs) = host_fs {
            if let Some((_mount, relative)) = self.find_mount(&resolved) {
                return hfs.write_file(relative, content);
            }
        }
        self.write_file(&resolved, content)
    }

    /// List a directory, delegating to `HostFs` if mounted.
    ///
    /// When reading from the host, entries are converted to `FsNode` so
    /// callers get a uniform type.
    pub fn list_dir_with_host(
        &self,
        path: &str,
        host_fs: Option<&dyn HostFs>,
    ) -> Result<Vec<FsNode>, String> {
        let resolved = self.resolve_path(path)?;
        if let Some(hfs) = host_fs {
            if let Some((_mount, relative)) = self.find_mount(&resolved) {
                let entries = hfs.list_dir(relative)?;
                return Ok(entries
                    .into_iter()
                    .map(|e| {
                        if e.is_dir {
                            FsNode::Directory(DirNode {
                                name: e.name,
                                children: HashMap::new(),
                                meta: default_dir_meta(),
                            })
                        } else {
                            FsNode::File(FileNode {
                                name: e.name,
                                content: ChunkedContent::new(),
                                meta: default_file_meta(),
                            })
                        }
                    })
                    .collect());
            }
        }
        self.list_dir(&resolved)
    }

    /// Check existence, delegating to `HostFs` if mounted.
    pub fn exists_with_host(
        &self,
        path: &str,
        host_fs: Option<&dyn HostFs>,
    ) -> Result<bool, String> {
        let resolved = self.resolve_path(path)?;
        if let Some(hfs) = host_fs {
            if let Some((_mount, relative)) = self.find_mount(&resolved) {
                return hfs.exists(relative);
            }
        }
        Ok(self.exists(&resolved))
    }

    /// Check if path is a directory, delegating to `HostFs` if mounted.
    pub fn is_dir_with_host(
        &self,
        path: &str,
        host_fs: Option<&dyn HostFs>,
    ) -> Result<bool, String> {
        let resolved = self.resolve_path(path)?;
        if let Some(hfs) = host_fs {
            if let Some((_mount, relative)) = self.find_mount(&resolved) {
                return hfs.is_dir(relative);
            }
        }
        Ok(self.is_dir(&resolved))
    }

    /// Create a directory, delegating to `HostFs` if mounted.
    pub fn mkdir_with_host(
        &mut self,
        path: &str,
        host_fs: Option<&dyn HostFs>,
    ) -> Result<String, String> {
        self.mkdir_with_host_and_owner(path, host_fs, 0, 0)
    }

    /// Create a directory with owner, delegating to `HostFs` if mounted.
    pub fn mkdir_with_host_and_owner(
        &mut self,
        path: &str,
        host_fs: Option<&dyn HostFs>,
        uid: u32,
        gid: u32,
    ) -> Result<String, String> {
        let resolved = self.resolve_path(path)?;
        if let Some(hfs) = host_fs {
            if let Some((_mount, relative)) = self.find_mount(&resolved) {
                return hfs.mkdir(relative);
            }
        }
        self.mkdir_with_owner(&resolved, uid, gid)
    }

    /// Touch a file, delegating to `HostFs` if mounted.
    pub fn touch_with_host(
        &mut self,
        path: &str,
        host_fs: Option<&dyn HostFs>,
    ) -> Result<String, String> {
        self.touch_with_host_and_owner(path, host_fs, 0, 0)
    }

    /// Touch a file with owner, delegating to `HostFs` if mounted.
    pub fn touch_with_host_and_owner(
        &mut self,
        path: &str,
        host_fs: Option<&dyn HostFs>,
        uid: u32,
        gid: u32,
    ) -> Result<String, String> {
        let resolved = self.resolve_path(path)?;
        if let Some(hfs) = host_fs {
            if let Some((_mount, relative)) = self.find_mount(&resolved) {
                return hfs.touch(relative);
            }
        }
        self.touch_with_owner(&resolved, uid, gid)
    }

    /// Remove a node, delegating to `HostFs` if mounted.
    ///
    /// Refuses to remove a mount root — use `unmount` first.
    pub fn rm_with_host(
        &mut self,
        path: &str,
        host_fs: Option<&dyn HostFs>,
    ) -> Result<String, String> {
        let resolved = self.resolve_path(path)?;
        if self.mounts.contains_key(&resolved) {
            return Err(format!(
                "cannot remove mount point '{}': use 'mount -u' to unmount first",
                path_display_name(&resolved)
            ));
        }
        if let Some(hfs) = host_fs {
            if let Some((_mount, relative)) = self.find_mount(&resolved) {
                return hfs.rm(relative);
            }
        }
        self.rm(&resolved)
    }

    /// Remove a node recursively, delegating to `HostFs` if mounted.
    pub fn rm_recursive_with_host(
        &mut self,
        path: &str,
        host_fs: Option<&dyn HostFs>,
    ) -> Result<String, String> {
        let resolved = self.resolve_path(path)?;
        if self.mounts.contains_key(&resolved) {
            return Err(format!(
                "cannot remove mount point '{}': use 'mount -u' to unmount first",
                path_display_name(&resolved)
            ));
        }
        if let Some(hfs) = host_fs {
            if let Some((_mount, relative)) = self.find_mount(&resolved) {
                return hfs.rm_recursive(relative);
            }
        }
        self.rm_recursive(&resolved)
    }

    // ---- Dirty tracking ------------------------------------------------------

    /// Mark a file path as modified or created since the last save.
    pub fn mark_dirty(&mut self, path: &str) {
        self.dirty_files.insert(path.to_string());
    }

    /// Mark a file path as deleted since the last save.
    fn mark_deleted(&mut self, path: &str) {
        self.deleted_files.insert(path.to_string());
        self.dirty_files.remove(path);
    }

    /// Recursively mark all files under a directory as deleted.
    fn mark_deleted_recursive(&mut self, dir_path: &str, dir: &DirNode) {
        for (name, node) in &dir.children {
            let child_path = Self::child_path(dir_path, name);
            match node {
                FsNode::File(_) => {
                    self.deleted_files.insert(child_path);
                }
                FsNode::Directory(d) => {
                    self.mark_deleted_recursive(&child_path, d);
                }
            }
        }
    }

    /// Return all paths that have been modified or created since the last save.
    pub fn get_dirty_files(&self) -> Vec<String> {
        self.dirty_files.iter().cloned().collect()
    }

    /// Return all paths that have been deleted since the last save.
    pub fn get_deleted_files(&self) -> Vec<String> {
        self.deleted_files.iter().cloned().collect()
    }

    /// Clear the dirty and deleted sets (called after a successful save).
    pub fn mark_clean(&mut self) {
        self.dirty_files.clear();
        self.deleted_files.clear();
    }

    /// Collect all file paths under a directory (for migration / mark_all_dirty).
    pub fn collect_all_file_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        self.collect_file_paths_recursive(&self.root, "/", &mut paths);
        paths
    }

    fn collect_file_paths_recursive(&self, dir: &DirNode, dir_path: &str, paths: &mut Vec<String>) {
        for (name, node) in &dir.children {
            let child_path = Self::child_path(dir_path, name);
            match node {
                FsNode::File(_) => paths.push(child_path),
                FsNode::Directory(d) => self.collect_file_paths_recursive(d, &child_path, paths),
            }
        }
    }

    // ---- Partial-read methods ------------------------------------------------

    /// Read a range of lines from a file.  Returns lines joined by `\n`.
    ///
    /// More efficient than [`read_file`] for large files when only a range of
    /// lines is needed (e.g. `head`, `tail`).
    pub fn read_file_lines(
        &self,
        path: &str,
        start_line: usize,
        count: usize,
    ) -> Result<String, String> {
        match self.get_node_at(path) {
            Some(FsNode::File(f)) => Ok(f.read_lines(start_line, count)),
            Some(FsNode::Directory(_)) => Err(format!(
                "read_file_lines: {}: Is a directory",
                path_display_name(path)
            )),
            None => Err(format!(
                "read_file_lines: {}: No such file or directory",
                path_display_name(path)
            )),
        }
    }

    /// Return the number of lines in a file without reading the full content
    /// into a single `String`.
    pub fn file_line_count(&self, path: &str) -> Result<usize, String> {
        match self.get_node_at(path) {
            Some(FsNode::File(f)) => Ok(f.line_count()),
            Some(FsNode::Directory(_)) => Err(format!(
                "file_line_count: {}: Is a directory",
                path_display_name(path)
            )),
            None => Err(format!(
                "file_line_count: {}: No such file or directory",
                path_display_name(path)
            )),
        }
    }

    /// Return the byte size of a file's content.
    pub fn file_size(&self, path: &str) -> Result<usize, String> {
        match self.get_node_at(path) {
            Some(FsNode::File(f)) => Ok(f.content.len()),
            Some(FsNode::Directory(_)) => Err(format!(
                "file_size: {}: Is a directory",
                path_display_name(path)
            )),
            None => Err(format!(
                "file_size: {}: No such file or directory",
                path_display_name(path)
            )),
        }
    }

    // ---- Tree-only JSON (for incremental persistence) -----------------------

    /// Serialise the VFS tree with empty file contents (tree structure only).
    ///
    /// Used by the incremental storage system — file contents are saved
    /// separately so that only changed files need to be written.
    pub fn to_tree_json(&self) -> String {
        let mut clone = self.clone();
        Self::clear_contents_recursive(&mut clone.root);
        serde_json::to_string(&clone).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    }

    fn clear_contents_recursive(dir: &mut DirNode) {
        for node in dir.children.values_mut() {
            match node {
                FsNode::File(f) => f.content = ChunkedContent::new(),
                FsNode::Directory(d) => Self::clear_contents_recursive(d),
            }
        }
    }

    // ---- Private helpers -----------------------------------------------------

    /// Build the absolute path for a child entry within a directory.
    ///
    /// Handles the root directory case where `dir_path == "/"` (avoids
    /// double-slash: `"/child"` instead of `"//child"`).
    pub fn child_path(dir_path: &str, child_name: &str) -> String {
        if dir_path == "/" {
            format!("/{}", child_name)
        } else {
            format!("{}/{}", dir_path, child_name)
        }
    }

    /// Split an absolute path into its parent directory path and the final
    /// component name.
    ///
    /// For `"/home/user/file.txt"` this returns `("/home/user", "file.txt")`.
    /// Returns `Err` for the root path `"/"` (no parent).
    fn split_parent_name(&self, path: &str) -> Result<(String, String), String> {
        let components = split_path(path);
        if components.is_empty() {
            return Err("cannot operate on root".to_string());
        }
        let name = components.last().unwrap().to_string();
        let parent = join_components(&components[..components.len() - 1]);
        Ok((parent, name))
    }

    /// Get an immutable reference to the node at the given absolute `path`.
    ///
    /// Returns `None` for the root path `"/"` (callers must handle the root
    /// specially because it is a bare `DirNode`, not wrapped in `FsNode`).
    /// Also returns `None` if any intermediate component is missing or is a
    /// file rather than a directory.
    fn get_node_at(&self, path: &str) -> Option<&FsNode> {
        if path == "/" || path.is_empty() {
            // Root is a DirNode, not FsNode. We handle this by treating "/"
            // specially in callers. For now, synthesize:
            return None; // callers handle root separately
        }
        let components = split_path(path);
        let mut current_dir = &self.root;
        for (i, comp) in components.iter().enumerate() {
            match current_dir.children.get(*comp) {
                Some(node) => {
                    if i == components.len() - 1 {
                        return Some(node);
                    }
                    match node {
                        FsNode::Directory(dir) => current_dir = dir,
                        _ => return None,
                    }
                }
                None => return None,
            }
        }
        None
    }

    /// Get a mutable reference to the node at the given absolute `path`.
    ///
    /// Same semantics as [`get_node_at`] but allows in-place mutation.
    fn get_node_at_mut(&mut self, path: &str) -> Option<&mut FsNode> {
        if path == "/" || path.is_empty() {
            return None;
        }
        let components = split_path(path);
        let mut current_dir = &mut self.root;
        for (i, comp) in components.iter().enumerate() {
            match current_dir.children.get_mut(*comp) {
                Some(node) => {
                    if i == components.len() - 1 {
                        return Some(node);
                    }
                    match node {
                        FsNode::Directory(dir) => current_dir = dir,
                        _ => return None,
                    }
                }
                None => return None,
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Construction --------------------------------------------------------

    /// Verify that [`Vfs::new`] creates the expected default directory layout:
    /// `/home`, `/tmp`, `/etc`, `/var`, and `/home/user`.
    #[test]
    fn new_creates_default_structure() {
        let vfs = Vfs::new();
        assert_eq!(vfs.cwd, "/");
        assert!(vfs.is_dir("/"));
        assert!(vfs.is_dir("/home"));
        assert!(vfs.is_dir("/tmp"));
        assert!(vfs.is_dir("/etc"));
        assert!(vfs.is_dir("/var"));
        assert!(vfs.is_dir("/home/user"));
    }

    // -- Path resolution ----------------------------------------------------

    /// Absolute paths should be returned unchanged.
    #[test]
    fn resolve_absolute_path() {
        let vfs = Vfs::new();
        assert_eq!(vfs.resolve_path("/").unwrap(), "/");
        assert_eq!(vfs.resolve_path("/home").unwrap(), "/home");
        assert_eq!(vfs.resolve_path("/home/user").unwrap(), "/home/user");
    }

    /// Relative paths should be resolved against the current working directory.
    #[test]
    fn resolve_relative_path() {
        let mut vfs = Vfs::new();
        vfs.cwd = "/home/user".to_string();
        assert_eq!(
            vfs.resolve_path("Documents").unwrap(),
            "/home/user/Documents"
        );
        assert_eq!(vfs.resolve_path("a/b").unwrap(), "/home/user/a/b");
    }

    /// `.` and `..` components should be handled correctly, including chains.
    #[test]
    fn resolve_dot_and_dotdot() {
        let mut vfs = Vfs::new();
        vfs.cwd = "/home/user".to_string();
        assert_eq!(vfs.resolve_path(".").unwrap(), "/home/user");
        assert_eq!(vfs.resolve_path("..").unwrap(), "/home");
        assert_eq!(vfs.resolve_path("../..").unwrap(), "/");
        assert_eq!(vfs.resolve_path("./foo/../bar").unwrap(), "/home/user/bar");
    }

    /// `~` should expand to `/home/user`.
    #[test]
    fn resolve_tilde() {
        let vfs = Vfs::new();
        assert_eq!(vfs.resolve_path("~").unwrap(), "/home/user");
        assert_eq!(
            vfs.resolve_path("~/Documents").unwrap(),
            "/home/user/Documents"
        );
    }

    /// `..` from root should stay at root (cannot go above `/`).
    #[test]
    fn resolve_dotdot_from_root_stays_at_root() {
        let vfs = Vfs::new();
        assert_eq!(vfs.resolve_path("/..").unwrap(), "/");
    }

    // -- exists / is_dir ----------------------------------------------------

    /// [`Vfs::exists`] should return `true` for root and known directories,
    /// and `false` for paths that have not been created.
    #[test]
    fn exists_returns_true_for_root_and_dirs() {
        let vfs = Vfs::new();
        assert!(vfs.exists("/"));
        assert!(vfs.exists("/home"));
        assert!(!vfs.exists("/nonexistent"));
    }

    /// [`Vfs::is_dir`] should distinguish files from directories.
    #[test]
    fn is_dir_distinguishes_files_and_dirs() {
        let mut vfs = Vfs::new();
        assert!(vfs.is_dir("/home"));
        vfs.touch("/home/user/test.txt").unwrap();
        assert!(!vfs.is_dir("/home/user/test.txt"));
    }

    // -- mkdir ---------------------------------------------------------------

    /// [`Vfs::mkdir`] should create a new directory at the given path.
    #[test]
    fn mkdir_creates_directory() {
        let mut vfs = Vfs::new();
        vfs.mkdir("/home/user/newdir").unwrap();
        assert!(vfs.is_dir("/home/user/newdir"));
    }

    /// [`Vfs::mkdir`] should fail if a node with the same name already exists.
    #[test]
    fn mkdir_fails_if_already_exists() {
        let mut vfs = Vfs::new();
        vfs.mkdir("/home/user/dir").unwrap();
        assert!(vfs.mkdir("/home/user/dir").is_err());
    }

    /// [`Vfs::mkdir`] should fail if the parent directory does not exist.
    #[test]
    fn mkdir_fails_if_parent_missing() {
        let mut vfs = Vfs::new();
        assert!(vfs.mkdir("/nonexistent/dir").is_err());
    }

    // -- touch ---------------------------------------------------------------

    /// [`Vfs::touch`] should create an empty file at the given path.
    #[test]
    fn touch_creates_empty_file() {
        let mut vfs = Vfs::new();
        vfs.touch("/home/user/file.txt").unwrap();
        assert_eq!(vfs.read_file("/home/user/file.txt").unwrap(), "");
    }

    /// [`Vfs::touch`] should be a no-op on an existing file (preserve content).
    #[test]
    fn touch_noop_on_existing_file() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/file.txt", "data").unwrap();
        vfs.touch("/home/user/file.txt").unwrap();
        assert_eq!(vfs.read_file("/home/user/file.txt").unwrap(), "data");
    }

    // -- write_file / read_file ---------------------------------------------

    /// [`Vfs::write_file`] followed by [`Vfs::read_file`] should round-trip
    /// the content correctly.
    #[test]
    fn write_and_read_file() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/hello.txt", "Hello, World!")
            .unwrap();
        assert_eq!(
            vfs.read_file("/home/user/hello.txt").unwrap(),
            "Hello, World!"
        );
    }

    /// Writing to an existing file should overwrite its content.
    #[test]
    fn write_file_overwrites() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/f.txt", "first").unwrap();
        vfs.write_file("/home/user/f.txt", "second").unwrap();
        assert_eq!(vfs.read_file("/home/user/f.txt").unwrap(), "second");
    }

    /// Reading a directory should return an error (not file content).
    #[test]
    fn read_file_returns_error_for_directory() {
        let vfs = Vfs::new();
        assert!(vfs.read_file("/home").is_err());
    }

    /// Reading a nonexistent path should return an error.
    #[test]
    fn read_file_returns_error_for_nonexistent() {
        let vfs = Vfs::new();
        assert!(vfs.read_file("/nonexistent.txt").is_err());
    }

    // -- rm ------------------------------------------------------------------

    /// [`Vfs::rm`] should remove an existing file.
    #[test]
    fn rm_removes_file() {
        let mut vfs = Vfs::new();
        vfs.touch("/home/user/f.txt").unwrap();
        vfs.rm("/home/user/f.txt").unwrap();
        assert!(!vfs.exists("/home/user/f.txt"));
    }

    /// [`Vfs::rm`] should refuse to remove a non-empty directory.
    #[test]
    fn rm_fails_for_non_empty_dir() {
        let mut vfs = Vfs::new();
        vfs.touch("/home/user/f.txt").unwrap();
        assert!(vfs.rm("/home/user").is_err());
    }

    /// [`Vfs::rm`] should refuse to remove the root directory.
    #[test]
    fn rm_fails_for_root() {
        let mut vfs = Vfs::new();
        assert!(vfs.rm("/").is_err());
    }

    /// [`Vfs::rm`] should fail when the target path does not exist.
    #[test]
    fn rm_fails_for_nonexistent() {
        let mut vfs = Vfs::new();
        assert!(vfs.rm("/nope").is_err());
    }

    // -- list_dir -----------------------------------------------------------

    /// [`Vfs::list_dir`] should return all children of a directory.
    #[test]
    fn list_dir_returns_children() {
        let mut vfs = Vfs::new();
        vfs.touch("/home/user/a.txt").unwrap();
        vfs.touch("/home/user/b.txt").unwrap();
        let entries = vfs.list_dir("/home/user").unwrap();
        assert_eq!(entries.len(), 2);
    }

    /// Listing the root should return at least the four default top-level dirs.
    #[test]
    fn list_dir_root() {
        let vfs = Vfs::new();
        let entries = vfs.list_dir("/").unwrap();
        assert!(entries.len() >= 4); // home, tmp, etc, var
    }

    // -- cp / mv ------------------------------------------------------------

    /// [`Vfs::cp`] should copy a file to a new location, leaving the original intact.
    #[test]
    fn cp_file() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/src.txt", "content").unwrap();
        vfs.cp("/home/user/src.txt", "/tmp/dst.txt").unwrap();
        assert_eq!(vfs.read_file("/tmp/dst.txt").unwrap(), "content");
        assert_eq!(vfs.read_file("/home/user/src.txt").unwrap(), "content"); // original intact
    }

    /// [`Vfs::mv`] should move a file, removing the original.
    #[test]
    fn mv_file() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/src.txt", "content").unwrap();
        vfs.mv("/home/user/src.txt", "/tmp/moved.txt").unwrap();
        assert_eq!(vfs.read_file("/tmp/moved.txt").unwrap(), "content");
        assert!(!vfs.exists("/home/user/src.txt"));
    }

    /// [`Vfs::cp`] should copy a file into an existing directory, preserving its name.
    #[test]
    fn cp_into_existing_dir() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/f.txt", "data").unwrap();
        vfs.mkdir("/tmp/dest").unwrap();
        vfs.cp("/home/user/f.txt", "/tmp/dest").unwrap();
        assert_eq!(vfs.read_file("/tmp/dest/f.txt").unwrap(), "data");
    }

    // -- JSON roundtrip -----------------------------------------------------

    /// Serialising to JSON and back should preserve all VFS data.
    #[test]
    fn json_roundtrip_preserves_data() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/test.txt", "hello").unwrap();
        vfs.mkdir("/home/user/subdir").unwrap();
        vfs.write_file("/home/user/subdir/nested.txt", "world")
            .unwrap();
        vfs.cwd = "/home/user".to_string();

        let json = vfs.to_json();
        let restored = Vfs::from_json(&json).unwrap();

        assert_eq!(restored.cwd, "/home/user");
        assert_eq!(restored.read_file("/home/user/test.txt").unwrap(), "hello");
        assert_eq!(
            restored.read_file("/home/user/subdir/nested.txt").unwrap(),
            "world"
        );
    }

    /// [`Vfs::from_json`] should reject malformed JSON input.
    #[test]
    fn from_json_rejects_invalid() {
        assert!(Vfs::from_json("not json").is_err());
    }

    // -- Nested operations --------------------------------------------------

    /// Operations on deeply nested paths should work correctly.
    #[test]
    fn deeply_nested_dir_and_file() {
        let mut vfs = Vfs::new();
        vfs.mkdir("/home/user/a").unwrap();
        vfs.mkdir("/home/user/a/b").unwrap();
        vfs.mkdir("/home/user/a/b/c").unwrap();
        vfs.write_file("/home/user/a/b/c/deep.txt", "deep content")
            .unwrap();
        assert_eq!(
            vfs.read_file("/home/user/a/b/c/deep.txt").unwrap(),
            "deep content"
        );
    }

    /// A path that traverses through a file (not a directory) should fail at
    /// the `is_dir` / `get_dir` level.
    #[test]
    fn resolve_path_through_existing_file() {
        let mut vfs = Vfs::new();
        vfs.touch("/home/user/f.txt").unwrap();
        // Trying to resolve a path that goes through a file should fail
        // at the is_dir / get_dir level, not resolve_path (which is pure string manipulation)
        assert!(!vfs.is_dir("/home/user/f.txt/sub"));
    }

    /// Paths containing null bytes should be rejected to prevent injection attacks.
    #[test]
    fn resolve_path_rejects_null_bytes() {
        let vfs = Vfs::new();
        assert!(vfs.resolve_path("\0etc/passwd").is_err());
        assert!(vfs.resolve_path("/home/user/\0hidden").is_err());
        assert!(vfs.resolve_path("file\0.txt").is_err());
    }

    // -- Dirty tracking -------------------------------------------------------

    #[test]
    fn write_file_marks_dirty() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/f.txt", "data").unwrap();
        assert!(vfs
            .get_dirty_files()
            .contains(&"/home/user/f.txt".to_string()));
    }

    #[test]
    fn touch_marks_dirty() {
        let mut vfs = Vfs::new();
        vfs.touch("/home/user/f.txt").unwrap();
        assert!(vfs
            .get_dirty_files()
            .contains(&"/home/user/f.txt".to_string()));
    }

    #[test]
    fn rm_marks_deleted() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/f.txt", "data").unwrap();
        vfs.mark_clean(); // clear dirty from write
        vfs.rm("/home/user/f.txt").unwrap();
        assert!(vfs
            .get_deleted_files()
            .contains(&"/home/user/f.txt".to_string()));
    }

    #[test]
    fn rm_recursive_marks_all_deleted() {
        let mut vfs = Vfs::new();
        vfs.mkdir("/home/user/dir").unwrap();
        vfs.write_file("/home/user/dir/a.txt", "a").unwrap();
        vfs.write_file("/home/user/dir/b.txt", "b").unwrap();
        vfs.mark_clean();
        vfs.rm_recursive("/home/user/dir").unwrap();
        let deleted = vfs.get_deleted_files();
        assert!(deleted.contains(&"/home/user/dir/a.txt".to_string()));
        assert!(deleted.contains(&"/home/user/dir/b.txt".to_string()));
    }

    #[test]
    fn cp_marks_destination_dirty() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/src.txt", "data").unwrap();
        vfs.mark_clean();
        vfs.cp("/home/user/src.txt", "/tmp/dst.txt").unwrap();
        assert!(vfs.get_dirty_files().contains(&"/tmp/dst.txt".to_string()));
    }

    #[test]
    fn mv_marks_dirty_and_deleted() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/src.txt", "data").unwrap();
        vfs.mark_clean();
        vfs.mv("/home/user/src.txt", "/tmp/moved.txt").unwrap();
        assert!(vfs
            .get_dirty_files()
            .contains(&"/tmp/moved.txt".to_string()));
        assert!(vfs
            .get_deleted_files()
            .contains(&"/home/user/src.txt".to_string()));
    }

    #[test]
    fn mark_clean_clears_all() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/f.txt", "data").unwrap();
        vfs.rm("/home/user/f.txt").unwrap();
        assert!(!vfs.get_dirty_files().is_empty() || !vfs.get_deleted_files().is_empty());
        vfs.mark_clean();
        assert!(vfs.get_dirty_files().is_empty());
        assert!(vfs.get_deleted_files().is_empty());
    }

    #[test]
    fn collect_all_file_paths_finds_files() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/a.txt", "a").unwrap();
        vfs.write_file("/home/user/b.txt", "b").unwrap();
        vfs.mkdir("/home/user/sub").unwrap();
        vfs.write_file("/home/user/sub/c.txt", "c").unwrap();
        let paths = vfs.collect_all_file_paths();
        assert!(paths.contains(&"/home/user/a.txt".to_string()));
        assert!(paths.contains(&"/home/user/b.txt".to_string()));
        assert!(paths.contains(&"/home/user/sub/c.txt".to_string()));
    }

    // -- Partial-read methods -------------------------------------------------

    #[test]
    fn read_file_lines_basic() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/f.txt", "a\nb\nc\nd\ne").unwrap();
        let result = vfs.read_file_lines("/home/user/f.txt", 0, 3).unwrap();
        assert_eq!(result, "a\nb\nc");
    }

    #[test]
    fn read_file_lines_with_offset() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/f.txt", "a\nb\nc\nd\ne").unwrap();
        let result = vfs.read_file_lines("/home/user/f.txt", 2, 2).unwrap();
        assert_eq!(result, "c\nd");
    }

    #[test]
    fn read_file_lines_past_end() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/f.txt", "a\nb").unwrap();
        let result = vfs.read_file_lines("/home/user/f.txt", 0, 100).unwrap();
        assert_eq!(result, "a\nb");
    }

    #[test]
    fn read_file_lines_empty_file() {
        let mut vfs = Vfs::new();
        vfs.touch("/home/user/empty.txt").unwrap();
        let result = vfs.read_file_lines("/home/user/empty.txt", 0, 10).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn read_file_lines_nonexistent() {
        let vfs = Vfs::new();
        assert!(vfs.read_file_lines("/nope.txt", 0, 1).is_err());
    }

    #[test]
    fn file_line_count_basic() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/f.txt", "a\nb\nc").unwrap();
        assert_eq!(vfs.file_line_count("/home/user/f.txt").unwrap(), 3);
    }

    #[test]
    fn file_line_count_empty() {
        let mut vfs = Vfs::new();
        vfs.touch("/home/user/empty.txt").unwrap();
        assert_eq!(vfs.file_line_count("/home/user/empty.txt").unwrap(), 0);
    }

    #[test]
    fn file_line_count_trailing_newline() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/f.txt", "a\nb\n").unwrap();
        assert_eq!(vfs.file_line_count("/home/user/f.txt").unwrap(), 2);
    }

    #[test]
    fn file_size_basic() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/f.txt", "hello").unwrap();
        assert_eq!(vfs.file_size("/home/user/f.txt").unwrap(), 5);
    }

    #[test]
    fn file_size_empty() {
        let mut vfs = Vfs::new();
        vfs.touch("/home/user/empty.txt").unwrap();
        assert_eq!(vfs.file_size("/home/user/empty.txt").unwrap(), 0);
    }

    // -- Tree-only JSON -------------------------------------------------------

    #[test]
    fn to_tree_json_has_structure_but_empty_contents() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/test.txt", "hello world")
            .unwrap();
        vfs.mkdir("/home/user/sub").unwrap();
        vfs.write_file("/home/user/sub/nested.txt", "content")
            .unwrap();

        let tree_json = vfs.to_tree_json();
        let restored = Vfs::from_json(&tree_json).unwrap();

        // Structure is preserved.
        assert!(restored.is_dir("/home/user"));
        assert!(restored.is_dir("/home/user/sub"));
        assert!(restored.exists("/home/user/test.txt"));
        assert!(restored.exists("/home/user/sub/nested.txt"));

        // Contents are empty.
        assert_eq!(restored.read_file("/home/user/test.txt").unwrap(), "");
        assert_eq!(restored.read_file("/home/user/sub/nested.txt").unwrap(), "");
    }

    // -- Mount management ------------------------------------------------------

    #[test]
    fn add_and_find_mount_exact() {
        let mut vfs = Vfs::new();
        vfs.add_mount("/mnt/host".to_string(), "mydir".to_string());
        let (mount, rel) = vfs.find_mount("/mnt/host").unwrap();
        assert_eq!(mount, "/mnt/host");
        assert_eq!(rel, "");
    }

    #[test]
    fn find_mount_nested_path() {
        let mut vfs = Vfs::new();
        vfs.add_mount("/mnt/host".to_string(), "mydir".to_string());
        let (mount, rel) = vfs.find_mount("/mnt/host/src/main.rs").unwrap();
        assert_eq!(mount, "/mnt/host");
        assert_eq!(rel, "src/main.rs");
    }

    #[test]
    fn find_mount_no_match() {
        let mut vfs = Vfs::new();
        vfs.add_mount("/mnt/host".to_string(), "mydir".to_string());
        assert!(vfs.find_mount("/home/user").is_none());
        // "/mnt/hostname" should NOT match "/mnt/host"
        assert!(vfs.find_mount("/mnt/hostname").is_none());
    }

    #[test]
    fn remove_mount() {
        let mut vfs = Vfs::new();
        vfs.add_mount("/mnt/host".to_string(), "mydir".to_string());
        assert!(vfs.remove_mount("/mnt/host"));
        assert!(!vfs.remove_mount("/mnt/host")); // already gone
        assert!(vfs.find_mount("/mnt/host").is_none());
    }

    #[test]
    fn mount_roundtrip_in_json() {
        let mut vfs = Vfs::new();
        vfs.add_mount("/mnt/project".to_string(), "project".to_string());
        let json = vfs.to_json();
        let restored = Vfs::from_json(&json).unwrap();
        assert!(restored.mounts.contains_key("/mnt/project"));
        assert_eq!(restored.mounts["/mnt/project"], "project");
    }

    #[test]
    fn rm_mount_point_refused() {
        let mut vfs = Vfs::new();
        vfs.mkdir("/mnt").unwrap();
        vfs.add_mount("/mnt/host".to_string(), "host".to_string());
        // Manually create the dir so rm has something to try to remove
        vfs.mkdir("/mnt/host").unwrap();
        let result = vfs.rm_with_host("/mnt/host", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("mount point"));
    }
}
