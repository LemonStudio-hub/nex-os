//! Virtual File System implementation
//!
//! Provides a tree-structured in-memory filesystem with POSIX-style paths,
//! serialized to JSON for OPFS persistence.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

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

/// The virtual file system – holds the root tree and tracks the current working
/// directory as an absolute POSIX path (e.g. "/" or "/home/user").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vfs {
    pub root: DirNode,
    pub cwd: String,
}

// ---------------------------------------------------------------------------
// Helpers (pure functions)
// ---------------------------------------------------------------------------

/// Split a path string into its non-empty components.
///
/// `split_path("/")`  → `[]`
/// `split_path("/a/b")` → `["a", "b"]`
/// `split_path("a/b")` → `["a", "b"]`
fn split_path(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

/// Join components back into an absolute path.
fn join_components(components: &[&str]) -> String {
    if components.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", components.join("/"))
    }
}

/// Get the last component of a path for display / error messages.
fn path_display_name(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) if i + 1 < path.len() => &path[i + 1..],
        _ => path,
    }
}

// ---------------------------------------------------------------------------
// Vfs implementation
// ---------------------------------------------------------------------------

impl Default for Vfs {
    fn default() -> Self {
        Self::new()
    }
}

impl Vfs {
    // ---- Construction --------------------------------------------------------

    /// Create a default VFS seeded with the standard top-level directories.
    pub fn new() -> Self {
        let mut root = DirNode {
            name: String::new(), // root's name is empty for convenience
            children: HashMap::new(),
        };

        // Helper to create an empty dir quickly
        fn empty_dir(name: &str) -> FsNode {
            FsNode::Directory(DirNode {
                name: name.to_string(),
                children: HashMap::new(),
            })
        }

        root.children.insert("home".to_string(), empty_dir("home"));
        root.children.insert("tmp".to_string(), empty_dir("tmp"));
        root.children.insert("etc".to_string(), empty_dir("etc"));
        root.children.insert("var".to_string(), empty_dir("var"));

        // Create /home/user
        if let Some(FsNode::Directory(ref mut home)) = root.children.get_mut("home") {
            home.children.insert("user".to_string(), empty_dir("user"));
        }

        Vfs {
            root,
            cwd: "/".to_string(),
        }
    }

    // ---- JSON (de)serialization ----------------------------------------------

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Failed to parse VFS JSON: {}", e))
    }

    // ---- Path resolution -----------------------------------------------------

    /// Resolve a path (absolute or relative) to a canonical absolute path string.
    /// Handles `.`, `..`, and `~` (mapped to /home/user).
    pub fn resolve_path(&self, path: &str) -> Result<String, String> {
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

    /// Check whether a node exists at `path` (absolute).
    /// Handles root "/" as always existing.
    pub fn exists(&self, path: &str) -> bool {
        if path == "/" || path.is_empty() {
            return true;
        }
        self.get_node_at(path).is_some()
    }

    /// Check whether `path` points to a directory.
    /// Handles root "/" as always being a directory.
    pub fn is_dir(&self, path: &str) -> bool {
        if path == "/" || path.is_empty() {
            return true;
        }
        matches!(self.get_node_at(path), Some(FsNode::Directory(_)))
    }

    /// Internal: get an immutable reference to the *directory* at `path`.
    /// Returns the root when `path` is "/".
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

    /// Internal: get a mutable reference to the *directory* at `path`.
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

    // ---- File / directory operations ------------------------------------------

    /// Create a directory at `path` (absolute). Does **not** create intermediates –
    /// callers (the `mkdir -p` command) are responsible for that.
    pub fn mkdir(&mut self, path: &str) -> Result<String, String> {
        let (parent_path, name) = self.split_parent_name(path)?;
        let parent = self.get_dir_mut(&parent_path).ok_or_else(|| {
            format!(
                "mkdir: cannot create directory '{}': No such file or directory",
                path_display_name(path)
            )
        })?;

        if parent.children.contains_key(&name) {
            return Err(format!(
                "mkdir: cannot create directory '{}': File exists",
                path_display_name(path)
            ));
        }

        parent.children.insert(
            name.clone(),
            FsNode::Directory(DirNode {
                name,
                children: HashMap::new(),
            }),
        );
        Ok(String::new())
    }

    /// Create an empty file if it doesn't exist. If it already exists, this is
    /// a no-op (matches POSIX `touch` semantics for a VFS without timestamps).
    pub fn touch(&mut self, path: &str) -> Result<String, String> {
        if self.exists(path) {
            return Ok(String::new());
        }
        let (parent_path, name) = self.split_parent_name(path)?;
        let parent = self.get_dir_mut(&parent_path).ok_or_else(|| {
            format!(
                "touch: cannot touch '{}': No such file or directory",
                path_display_name(path)
            )
        })?;

        parent.children.insert(
            name.clone(),
            FsNode::File(FileNode {
                name,
                content: String::new(),
            }),
        );
        Ok(String::new())
    }

    /// Remove the node at `path` (absolute). Works for both files and
    /// directories. If `recursive` is true, non-empty directories are removed
    /// along with all their contents.
    pub fn rm(&mut self, path: &str) -> Result<String, String> {
        self.rm_inner(path, false)
    }

    /// Like `rm` but allows removing non-empty directories recursively.
    pub fn rm_recursive(&mut self, path: &str) -> Result<String, String> {
        self.rm_inner(path, true)
    }

    fn rm_inner(&mut self, path: &str, recursive: bool) -> Result<String, String> {
        if path == "/" {
            return Err("rm: cannot remove '/'".to_string());
        }
        let (parent_path, name) = self.split_parent_name(path)?;
        let parent = self.get_dir_mut(&parent_path).ok_or_else(|| {
            format!(
                "rm: cannot remove '{}': No such file or directory",
                path_display_name(path)
            )
        })?;

        if let Some(FsNode::Directory(dir)) = parent.children.get(&name) {
            if !dir.children.is_empty() && !recursive {
                return Err(format!(
                    "rm: cannot remove '{}': Is a directory",
                    path_display_name(path)
                ));
            }
        }

        parent.children.remove(&name).ok_or_else(|| {
            format!(
                "rm: cannot remove '{}': No such file or directory",
                path_display_name(path)
            )
        })?;
        Ok(String::new())
    }

    /// Read the text content of a file at `path`.
    pub fn read_file(&self, path: &str) -> Result<String, String> {
        match self.get_node_at(path) {
            Some(FsNode::File(f)) => Ok(f.content.clone()),
            Some(FsNode::Directory(_)) => {
                Err(format!("cat: {}: Is a directory", path_display_name(path)))
            }
            None => Err(format!(
                "cat: {}: No such file or directory",
                path_display_name(path)
            )),
        }
    }

    /// Write (overwrite or append) text to a file at `path`. Creates the file
    /// if it does not exist.
    pub fn write_file(&mut self, path: &str, content: &str) -> Result<String, String> {
        // If file already exists, update in place
        if let Some(FsNode::File(f)) = self.get_node_at_mut(path) {
            f.content = content.to_string();
            return Ok(String::new());
        }

        // Otherwise create it
        let (parent_path, name) = self.split_parent_name(path)?;
        let parent = self.get_dir_mut(&parent_path).ok_or_else(|| {
            format!(
                "write: cannot create '{}': No such file or directory",
                path_display_name(path)
            )
        })?;

        parent.children.insert(
            name.clone(),
            FsNode::File(FileNode {
                name,
                content: content.to_string(),
            }),
        );
        Ok(String::new())
    }

    /// List the children of the directory at `path`.
    pub fn list_dir(&self, path: &str) -> Result<Vec<FsNode>, String> {
        let dir = self.get_dir(path).ok_or_else(|| {
            format!(
                "ls: cannot access '{}': No such file or directory",
                path_display_name(path)
            )
        })?;
        Ok(dir.children.values().cloned().collect())
    }

    /// Copy a file or directory from `src` to `dst`. Both paths are absolute.
    pub fn cp(&mut self, src: &str, dst: &str) -> Result<String, String> {
        let node = self
            .get_node_at(src)
            .ok_or_else(|| {
                format!(
                    "cp: cannot stat '{}': No such file or directory",
                    path_display_name(src)
                )
            })?
            .clone();

        // If dst is an existing directory, copy into it
        let actual_dst = if self.is_dir(dst) {
            let name = path_display_name(src);
            if dst == "/" {
                format!("/{}", name)
            } else {
                format!("{}/{}", dst, name)
            }
        } else {
            dst.to_string()
        };

        // Ensure parent directory exists
        let (parent_path, name) = self.split_parent_name(&actual_dst)?;
        let parent = self.get_dir_mut(&parent_path).ok_or_else(|| {
            format!(
                "cp: cannot create '{}': No such file or directory",
                path_display_name(&actual_dst)
            )
        })?;

        // Rename the top-level node if needed
        let mut new_node = node;
        match &mut new_node {
            FsNode::File(f) => f.name = name,
            FsNode::Directory(d) => d.name = name,
        }

        parent
            .children
            .insert(path_display_name(&actual_dst).to_string(), new_node);
        Ok(String::new())
    }

    /// Move / rename from `src` to `dst`. Both paths are absolute.
    pub fn mv(&mut self, src: &str, dst: &str) -> Result<String, String> {
        // Copy then remove source
        self.cp(src, dst)?;
        // Remove the original – need to handle directory removal without -r check
        let (parent_path, name) = self.split_parent_name(src)?;
        if let Some(parent) = self.get_dir_mut(&parent_path) {
            parent.children.remove(&name);
        }
        Ok(String::new())
    }

    // ---- Private helpers -----------------------------------------------------

    /// Split an absolute path into (parent_absolute_path, final_component_name).
    fn split_parent_name(&self, path: &str) -> Result<(String, String), String> {
        let components = split_path(path);
        if components.is_empty() {
            return Err("cannot operate on root".to_string());
        }
        let name = components.last().unwrap().to_string();
        let parent = join_components(&components[..components.len() - 1]);
        Ok((parent, name))
    }

    /// Get node at an absolute path (handles "/" → synthetic root).
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

    /// Get mutable node at an absolute path.
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

    #[test]
    fn resolve_absolute_path() {
        let vfs = Vfs::new();
        assert_eq!(vfs.resolve_path("/").unwrap(), "/");
        assert_eq!(vfs.resolve_path("/home").unwrap(), "/home");
        assert_eq!(vfs.resolve_path("/home/user").unwrap(), "/home/user");
    }

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

    #[test]
    fn resolve_dot_and_dotdot() {
        let mut vfs = Vfs::new();
        vfs.cwd = "/home/user".to_string();
        assert_eq!(vfs.resolve_path(".").unwrap(), "/home/user");
        assert_eq!(vfs.resolve_path("..").unwrap(), "/home");
        assert_eq!(vfs.resolve_path("../..").unwrap(), "/");
        assert_eq!(vfs.resolve_path("./foo/../bar").unwrap(), "/home/user/bar");
    }

    #[test]
    fn resolve_tilde() {
        let vfs = Vfs::new();
        assert_eq!(vfs.resolve_path("~").unwrap(), "/home/user");
        assert_eq!(
            vfs.resolve_path("~/Documents").unwrap(),
            "/home/user/Documents"
        );
    }

    #[test]
    fn resolve_dotdot_from_root_stays_at_root() {
        let vfs = Vfs::new();
        assert_eq!(vfs.resolve_path("/..").unwrap(), "/");
    }

    // -- exists / is_dir ----------------------------------------------------

    #[test]
    fn exists_returns_true_for_root_and_dirs() {
        let vfs = Vfs::new();
        assert!(vfs.exists("/"));
        assert!(vfs.exists("/home"));
        assert!(!vfs.exists("/nonexistent"));
    }

    #[test]
    fn is_dir_distinguishes_files_and_dirs() {
        let mut vfs = Vfs::new();
        assert!(vfs.is_dir("/home"));
        vfs.touch("/home/user/test.txt").unwrap();
        assert!(!vfs.is_dir("/home/user/test.txt"));
    }

    // -- mkdir ---------------------------------------------------------------

    #[test]
    fn mkdir_creates_directory() {
        let mut vfs = Vfs::new();
        vfs.mkdir("/home/user/newdir").unwrap();
        assert!(vfs.is_dir("/home/user/newdir"));
    }

    #[test]
    fn mkdir_fails_if_already_exists() {
        let mut vfs = Vfs::new();
        vfs.mkdir("/home/user/dir").unwrap();
        assert!(vfs.mkdir("/home/user/dir").is_err());
    }

    #[test]
    fn mkdir_fails_if_parent_missing() {
        let mut vfs = Vfs::new();
        assert!(vfs.mkdir("/nonexistent/dir").is_err());
    }

    // -- touch ---------------------------------------------------------------

    #[test]
    fn touch_creates_empty_file() {
        let mut vfs = Vfs::new();
        vfs.touch("/home/user/file.txt").unwrap();
        assert_eq!(vfs.read_file("/home/user/file.txt").unwrap(), "");
    }

    #[test]
    fn touch_noop_on_existing_file() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/file.txt", "data").unwrap();
        vfs.touch("/home/user/file.txt").unwrap();
        assert_eq!(vfs.read_file("/home/user/file.txt").unwrap(), "data");
    }

    // -- write_file / read_file ---------------------------------------------

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

    #[test]
    fn write_file_overwrites() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/f.txt", "first").unwrap();
        vfs.write_file("/home/user/f.txt", "second").unwrap();
        assert_eq!(vfs.read_file("/home/user/f.txt").unwrap(), "second");
    }

    #[test]
    fn read_file_returns_error_for_directory() {
        let vfs = Vfs::new();
        assert!(vfs.read_file("/home").is_err());
    }

    #[test]
    fn read_file_returns_error_for_nonexistent() {
        let vfs = Vfs::new();
        assert!(vfs.read_file("/nonexistent.txt").is_err());
    }

    // -- rm ------------------------------------------------------------------

    #[test]
    fn rm_removes_file() {
        let mut vfs = Vfs::new();
        vfs.touch("/home/user/f.txt").unwrap();
        vfs.rm("/home/user/f.txt").unwrap();
        assert!(!vfs.exists("/home/user/f.txt"));
    }

    #[test]
    fn rm_fails_for_non_empty_dir() {
        let mut vfs = Vfs::new();
        vfs.touch("/home/user/f.txt").unwrap();
        assert!(vfs.rm("/home/user").is_err());
    }

    #[test]
    fn rm_fails_for_root() {
        let mut vfs = Vfs::new();
        assert!(vfs.rm("/").is_err());
    }

    #[test]
    fn rm_fails_for_nonexistent() {
        let mut vfs = Vfs::new();
        assert!(vfs.rm("/nope").is_err());
    }

    // -- list_dir -----------------------------------------------------------

    #[test]
    fn list_dir_returns_children() {
        let mut vfs = Vfs::new();
        vfs.touch("/home/user/a.txt").unwrap();
        vfs.touch("/home/user/b.txt").unwrap();
        let entries = vfs.list_dir("/home/user").unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn list_dir_root() {
        let vfs = Vfs::new();
        let entries = vfs.list_dir("/").unwrap();
        assert!(entries.len() >= 4); // home, tmp, etc, var
    }

    // -- cp / mv ------------------------------------------------------------

    #[test]
    fn cp_file() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/src.txt", "content").unwrap();
        vfs.cp("/home/user/src.txt", "/tmp/dst.txt").unwrap();
        assert_eq!(vfs.read_file("/tmp/dst.txt").unwrap(), "content");
        assert_eq!(vfs.read_file("/home/user/src.txt").unwrap(), "content"); // original intact
    }

    #[test]
    fn mv_file() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/src.txt", "content").unwrap();
        vfs.mv("/home/user/src.txt", "/tmp/moved.txt").unwrap();
        assert_eq!(vfs.read_file("/tmp/moved.txt").unwrap(), "content");
        assert!(!vfs.exists("/home/user/src.txt"));
    }

    #[test]
    fn cp_into_existing_dir() {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/f.txt", "data").unwrap();
        vfs.mkdir("/tmp/dest").unwrap();
        vfs.cp("/home/user/f.txt", "/tmp/dest").unwrap();
        assert_eq!(vfs.read_file("/tmp/dest/f.txt").unwrap(), "data");
    }

    // -- JSON roundtrip -----------------------------------------------------

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

    #[test]
    fn from_json_rejects_invalid() {
        assert!(Vfs::from_json("not json").is_err());
    }

    // -- Nested operations --------------------------------------------------

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

    #[test]
    fn resolve_path_through_existing_file() {
        let mut vfs = Vfs::new();
        vfs.touch("/home/user/f.txt").unwrap();
        // Trying to resolve a path that goes through a file should fail
        // at the is_dir / get_dir level, not resolve_path (which is pure string manipulation)
        assert!(!vfs.is_dir("/home/user/f.txt/sub"));
    }
}
