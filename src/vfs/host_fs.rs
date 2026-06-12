//! Host filesystem abstraction for mounting real directories.
//!
//! The [`HostFs`] trait defines synchronous operations that delegate to the
//! real filesystem (via JS callbacks in WASM, or in-memory maps for testing).
//! When a VFS path falls under a mount point, the `_with_host` methods on
//! [`Vfs`](super::Vfs) forward to this trait instead of the in-memory tree.

/// A single entry returned by directory listing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HostEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: usize,
}

/// Trait abstracting host filesystem operations.
///
/// All methods are synchronous because WASM is single-threaded — async JS
/// operations are pre-resolved into a cache before entering WASM, and writes
/// are queued for flush after WASM returns.
pub trait HostFs {
    /// List the immediate children of a host directory.
    fn list_dir(&self, host_path: &str) -> Result<Vec<HostEntry>, String>;

    /// Read the full text content of a host file.
    fn read_file(&self, host_path: &str) -> Result<String, String>;

    /// Read a range of lines from a host file.
    fn read_file_lines(
        &self,
        host_path: &str,
        start: usize,
        count: usize,
    ) -> Result<String, String>;

    /// Return the number of lines in a host file.
    fn file_line_count(&self, host_path: &str) -> Result<usize, String>;

    /// Write text content to a host file (overwrite).
    fn write_file(&self, host_path: &str, content: &str) -> Result<String, String>;

    /// Append text content to a host file.
    fn append_file(&self, host_path: &str, content: &str) -> Result<String, String>;

    /// Create a directory on the host.
    fn mkdir(&self, host_path: &str) -> Result<String, String>;

    /// Create an empty file on the host (no-op if exists).
    fn touch(&self, host_path: &str) -> Result<String, String>;

    /// Remove a file or empty directory on the host.
    fn rm(&self, host_path: &str) -> Result<String, String>;

    /// Remove a file or directory recursively on the host.
    fn rm_recursive(&self, host_path: &str) -> Result<String, String>;

    /// Return the size of a file in bytes.
    fn file_size(&self, host_path: &str) -> Result<usize, String>;

    /// Check if a path exists on the host.
    fn exists(&self, host_path: &str) -> Result<bool, String>;

    /// Check if a path is a directory on the host.
    fn is_dir(&self, host_path: &str) -> Result<bool, String>;
}

// ---------------------------------------------------------------------------
// Mock implementation for testing
// ---------------------------------------------------------------------------

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;

    /// In-memory mock of the host filesystem for unit testing.
    #[derive(Debug, Default)]
    pub struct MockHostFs {
        /// Maps path -> content (files) or path -> "" (directories).
        entries: HashMap<String, MockEntry>,
    }

    #[derive(Debug, Clone)]
    enum MockEntry {
        File(String),
        Dir,
    }

    impl MockHostFs {
        pub fn new() -> Self {
            Self {
                entries: HashMap::new(),
            }
        }

        /// Pre-populate a directory.
        pub fn add_dir(&mut self, path: &str) {
            self.entries.insert(path.to_string(), MockEntry::Dir);
        }

        /// Pre-populate a file with content.
        pub fn add_file(&mut self, path: &str, content: &str) {
            self.entries
                .insert(path.to_string(), MockEntry::File(content.to_string()));
        }
    }

    impl HostFs for MockHostFs {
        fn list_dir(&self, host_path: &str) -> Result<Vec<HostEntry>, String> {
            let prefix = if host_path.ends_with('/') {
                host_path.to_string()
            } else {
                format!("{}/", host_path)
            };
            let mut entries = Vec::new();
            for (path, entry) in &self.entries {
                if path.starts_with(&prefix) {
                    let rest = &path[prefix.len()..];
                    // Only immediate children (no further '/')
                    if !rest.contains('/') && !rest.is_empty() {
                        entries.push(HostEntry {
                            name: rest.to_string(),
                            is_dir: matches!(entry, MockEntry::Dir),
                            size: match entry {
                                MockEntry::File(c) => c.len(),
                                MockEntry::Dir => 0,
                            },
                        });
                    }
                }
            }
            Ok(entries)
        }

        fn read_file(&self, host_path: &str) -> Result<String, String> {
            match self.entries.get(host_path) {
                Some(MockEntry::File(c)) => Ok(c.clone()),
                Some(MockEntry::Dir) => Err(format!("{}: Is a directory", host_path)),
                None => Err(format!("{}: No such file or directory", host_path)),
            }
        }

        fn read_file_lines(
            &self,
            host_path: &str,
            start: usize,
            count: usize,
        ) -> Result<String, String> {
            let content = self.read_file(host_path)?;
            let lines: Vec<&str> = content.lines().collect();
            let end = (start + count).min(lines.len());
            if start >= lines.len() {
                return Ok(String::new());
            }
            Ok(lines[start..end].join("\n"))
        }

        fn file_line_count(&self, host_path: &str) -> Result<usize, String> {
            let content = self.read_file(host_path)?;
            if content.is_empty() {
                return Ok(0);
            }
            Ok(content.lines().count())
        }

        fn write_file(&self, host_path: &str, content: &str) -> Result<String, String> {
            // Note: MockHostFs uses interior mutability pattern in real usage;
            // for tests, mutations happen through the &mut MockHostFs directly.
            // This method is a no-op in the trait impl since tests use &mut.
            // The actual mock writes are handled by the test harness.
            let _ = (host_path, content);
            Ok(String::new())
        }

        fn append_file(&self, host_path: &str, content: &str) -> Result<String, String> {
            let _ = (host_path, content);
            Ok(String::new())
        }

        fn mkdir(&self, host_path: &str) -> Result<String, String> {
            let _ = host_path;
            Ok(String::new())
        }

        fn touch(&self, host_path: &str) -> Result<String, String> {
            let _ = host_path;
            Ok(String::new())
        }

        fn rm(&self, host_path: &str) -> Result<String, String> {
            let _ = host_path;
            Ok(String::new())
        }

        fn rm_recursive(&self, host_path: &str) -> Result<String, String> {
            let _ = host_path;
            Ok(String::new())
        }

        fn file_size(&self, host_path: &str) -> Result<usize, String> {
            match self.entries.get(host_path) {
                Some(MockEntry::File(c)) => Ok(c.len()),
                Some(MockEntry::Dir) => Err(format!("{}: Is a directory", host_path)),
                None => Err(format!("{}: No such file or directory", host_path)),
            }
        }

        fn exists(&self, host_path: &str) -> Result<bool, String> {
            Ok(self.entries.contains_key(host_path))
        }

        fn is_dir(&self, host_path: &str) -> Result<bool, String> {
            match self.entries.get(host_path) {
                Some(MockEntry::Dir) => Ok(true),
                Some(MockEntry::File(_)) => Ok(false),
                None => Err(format!("{}: No such file or directory", host_path)),
            }
        }
    }
}
