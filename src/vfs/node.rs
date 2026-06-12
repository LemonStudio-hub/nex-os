//! VFS node types: files and directories.
//!
//! The filesystem tree is built from [`FsNode`] values.  Each node is either a
//! [`FileNode`] (leaf, stores text content) or a [`DirNode`] (branch, stores a
//! map of child names to nodes).  Nodes are serialisable so the entire tree can
//! be persisted as JSON in the browser's OPFS.
//!
//! Large file content is stored in fixed-size chunks ([`ChunkedContent`]) so
//! that only the relevant portion needs to be read for line-range queries
//! (e.g. `head`, `tail`).

use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;

/// Size of each content chunk in bytes (64 KiB).
const CHUNK_SIZE: usize = 65_536;

/// Threshold below which content is serialised as a plain JSON string
/// (backward-compatible with the legacy format).  Above this size the
/// chunked object format is used.
const INLINE_THRESHOLD: usize = 4_096;

// ---------------------------------------------------------------------------
// ChunkedContent
// ---------------------------------------------------------------------------

/// A string split into fixed-size chunks for efficient partial reads.
///
/// Small content (below [`INLINE_THRESHOLD`]) is stored as a single chunk.
/// The custom [`Serialize`]/[`Deserialize`] impls transparently handle both
/// the legacy plain-string format and the new chunked-object format.
#[derive(Debug, Clone)]
pub struct ChunkedContent {
    /// Content split into chunks of up to [`CHUNK_SIZE`] bytes each.
    chunks: Vec<String>,
    /// Total byte length across all chunks.
    total_len: usize,
}

impl ChunkedContent {
    /// Create empty content.
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            total_len: 0,
        }
    }

    /// Build content from a string slice, splitting into chunks as needed.
    pub fn from_str(s: &str) -> Self {
        if s.is_empty() {
            return Self::new();
        }

        if s.len() <= CHUNK_SIZE {
            return Self {
                chunks: vec![s.to_string()],
                total_len: s.len(),
            };
        }

        let mut chunks = Vec::new();
        let mut remaining = s;
        while !remaining.is_empty() {
            let end = if remaining.len() <= CHUNK_SIZE {
                remaining.len()
            } else {
                // Find the last char boundary at or before CHUNK_SIZE.
                let mut end = CHUNK_SIZE;
                while !remaining.is_char_boundary(end) {
                    end -= 1;
                }
                end
            };
            chunks.push(remaining[..end].to_string());
            remaining = &remaining[end..];
        }

        let total_len = s.len();
        Self { chunks, total_len }
    }

    /// Concatenate all chunks into a single `String`.
    pub fn to_string(&self) -> String {
        self.chunks.concat()
    }

    /// Total byte length of the content.
    pub fn len(&self) -> usize {
        self.total_len
    }

    /// Whether the content is empty.
    pub fn is_empty(&self) -> bool {
        self.total_len == 0
    }

    /// Count the number of lines in the content.
    ///
    /// A trailing newline does **not** add an extra empty line (matching
    /// `str::lines()` behaviour).  An empty file has zero lines.
    pub fn line_count(&self) -> usize {
        if self.total_len == 0 {
            return 0;
        }
        let mut count = 0usize;
        for chunk in &self.chunks {
            count += chunk.bytes().filter(|&b| b == b'\n').count();
        }
        // If the content does not end with '\n', the last line has no trailing
        // newline but still counts.
        if !self.ends_with_newline() {
            count += 1;
        }
        count
    }

    /// Return a range of lines as a `Vec<String>`.
    ///
    /// `start_line` is zero-based.  Lines that span chunk boundaries are
    /// reassembled transparently.  If the requested range extends past the
    /// end of the file, only the available lines are returned.
    pub fn lines(&self, start_line: usize, count: usize) -> Vec<String> {
        if count == 0 || self.total_len == 0 {
            return Vec::new();
        }

        // Flatten all chunks into one string for line extraction.
        // For typical files this is the same allocation read_file already did;
        // for very large files we could optimise further by scanning chunks
        // directly, but the chunk boundary handling is complex.
        let full = self.to_string();
        full.lines()
            .skip(start_line)
            .take(count)
            .map(|s| s.to_string())
            .collect()
    }

    /// Append content, creating new chunks as needed.
    pub fn push_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }

        // Try to fit into the last chunk if there is room.
        if let Some(last) = self.chunks.last_mut() {
            let room = CHUNK_SIZE.saturating_sub(last.len());
            if room > 0 {
                let take = s.len().min(room);
                // Find a char boundary at or before `take`.
                let mut end = take;
                while !s.is_char_boundary(end) {
                    end -= 1;
                }
                if end > 0 {
                    last.push_str(&s[..end]);
                    self.total_len += end;
                    let remaining = &s[end..];
                    if remaining.is_empty() {
                        return;
                    }
                    // Fall through to chunk the rest.
                    let rest = ChunkedContent::from_str(remaining);
                    self.chunks.extend(rest.chunks);
                    self.total_len += rest.total_len;
                    return;
                }
            }
        }

        // No last chunk or no room — create fresh chunks.
        let new = ChunkedContent::from_str(s);
        self.total_len += new.total_len;
        self.chunks.extend(new.chunks);
    }

    /// Reset to empty content.
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.total_len = 0;
    }

    /// Whether the content ends with a newline character.
    fn ends_with_newline(&self) -> bool {
        self.chunks
            .last()
            .map_or(false, |last| last.ends_with('\n'))
    }
}

impl Default for ChunkedContent {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for ChunkedContent {
    fn eq(&self, other: &Self) -> bool {
        self.total_len == other.total_len && self.to_string() == other.to_string()
    }
}

impl Eq for ChunkedContent {}

// -- Custom Serialize / Deserialize ------------------------------------------

impl Serialize for ChunkedContent {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.total_len <= INLINE_THRESHOLD {
            // Small content: plain string (backward-compatible).
            serializer.serialize_str(&self.to_string())
        } else {
            // Large content: structured object.
            let mut state = serializer.serialize_struct("ChunkedContent", 3)?;
            state.serialize_field("__chunked", &true)?;
            state.serialize_field("chunks", &self.chunks)?;
            state.serialize_field("total_len", &self.total_len)?;
            state.end()
        }
    }
}

impl<'de> Deserialize<'de> for ChunkedContent {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ChunkedVisitor;

        impl<'de> Visitor<'de> for ChunkedVisitor {
            type Value = ChunkedContent;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string or a chunked content object")
            }

            /// Legacy plain-string format.
            fn visit_str<E: de::Error>(self, v: &str) -> Result<ChunkedContent, E> {
                Ok(ChunkedContent::from_str(v))
            }

            /// Legacy plain-string format (owned).
            fn visit_string<E: de::Error>(self, v: String) -> Result<ChunkedContent, E> {
                Ok(ChunkedContent::from_str(&v))
            }

            /// Null / missing content → empty.
            fn visit_none<E: de::Error>(self) -> Result<ChunkedContent, E> {
                Ok(ChunkedContent::new())
            }

            /// Unit → empty.
            fn visit_unit<E: de::Error>(self) -> Result<ChunkedContent, E> {
                Ok(ChunkedContent::new())
            }

            /// New chunked-object format.
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<ChunkedContent, M::Error> {
                let mut chunks: Option<Vec<String>> = None;
                let mut total_len: Option<usize> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "__chunked" => {
                            let _: bool = map.next_value()?;
                        }
                        "chunks" => {
                            chunks = Some(map.next_value()?);
                        }
                        "total_len" => {
                            total_len = Some(map.next_value()?);
                        }
                        _ => {
                            let _: serde_json::Value = map.next_value()?;
                        }
                    }
                }

                let chunks = chunks.unwrap_or_default();
                let total_len = total_len.unwrap_or_else(|| chunks.iter().map(|c| c.len()).sum());
                Ok(ChunkedContent { chunks, total_len })
            }
        }

        deserializer.deserialize_any(ChunkedVisitor)
    }
}

// ---------------------------------------------------------------------------
// FsNode / FileNode / DirNode
// ---------------------------------------------------------------------------

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

/// A file node with chunked text content.
///
/// Files in NexOS are simple text blobs; there is no binary support.
/// The `name` field holds the file's basename (e.g. `"readme.txt"`),
/// while `content` holds the body as a [`ChunkedContent`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    /// Basename of the file (no path separators).
    pub name: String,
    /// File content stored in fixed-size chunks.
    pub content: ChunkedContent,
}

impl FileNode {
    /// Return a range of lines from this file, joined by `\n`.
    ///
    /// This is more efficient than reading the full content when only a
    /// prefix or suffix of the file is needed (e.g. `head`, `tail`).
    pub fn read_lines(&self, start: usize, count: usize) -> String {
        self.content.lines(start, count).join("\n")
    }

    /// Return the number of lines in this file.
    pub fn line_count(&self) -> usize {
        self.content.line_count()
    }
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

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- ChunkedContent::from_str / to_string --------------------------------

    #[test]
    fn chunked_empty() {
        let c = ChunkedContent::from_str("");
        assert!(c.is_empty());
        assert_eq!(c.len(), 0);
        assert_eq!(c.to_string(), "");
    }

    #[test]
    fn chunked_short_string() {
        let c = ChunkedContent::from_str("hello world");
        assert_eq!(c.to_string(), "hello world");
        assert_eq!(c.len(), 11);
        assert_eq!(c.chunks.len(), 1);
    }

    #[test]
    fn chunked_exact_one_chunk() {
        let s = "x".repeat(CHUNK_SIZE);
        let c = ChunkedContent::from_str(&s);
        assert_eq!(c.chunks.len(), 1);
        assert_eq!(c.to_string(), s);
    }

    #[test]
    fn chunked_multiple_chunks() {
        let s = "y".repeat(CHUNK_SIZE * 3 + 100);
        let c = ChunkedContent::from_str(&s);
        assert_eq!(c.chunks.len(), 4);
        assert_eq!(c.to_string(), s);
        assert_eq!(c.len(), s.len());
    }

    #[test]
    fn chunked_respects_char_boundaries() {
        // 3-byte UTF-8 char (é = 0xC3 0xA9).  Fill to exactly CHUNK_SIZE
        // bytes with ASCII, then append multi-byte chars that would cross the
        // boundary.
        let prefix = "a".repeat(CHUNK_SIZE - 1);
        let s = format!("{}ééé", prefix);
        let c = ChunkedContent::from_str(&s);
        assert_eq!(c.to_string(), s);
        // Ensure no chunk exceeds CHUNK_SIZE.
        for chunk in &c.chunks {
            assert!(chunk.len() <= CHUNK_SIZE);
        }
    }

    // -- line_count -----------------------------------------------------------

    #[test]
    fn line_count_empty() {
        assert_eq!(ChunkedContent::new().line_count(), 0);
    }

    #[test]
    fn line_count_single_line_no_newline() {
        let c = ChunkedContent::from_str("hello");
        assert_eq!(c.line_count(), 1);
    }

    #[test]
    fn line_count_trailing_newline() {
        let c = ChunkedContent::from_str("hello\n");
        assert_eq!(c.line_count(), 1);
    }

    #[test]
    fn line_count_multiple_lines() {
        let c = ChunkedContent::from_str("a\nb\nc\n");
        assert_eq!(c.line_count(), 3);
    }

    #[test]
    fn line_count_no_trailing_newline() {
        let c = ChunkedContent::from_str("a\nb\nc");
        assert_eq!(c.line_count(), 3);
    }

    #[test]
    fn line_count_across_chunks() {
        // Build a string with lines that span chunk boundaries.
        let line = "x".repeat(CHUNK_SIZE / 2) + "\n";
        let s = line.repeat(5);
        let c = ChunkedContent::from_str(&s);
        assert_eq!(c.line_count(), 5);
    }

    // -- lines() --------------------------------------------------------------

    #[test]
    fn lines_basic() {
        let c = ChunkedContent::from_str("a\nb\nc\nd\ne");
        let result = c.lines(0, 3);
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn lines_with_offset() {
        let c = ChunkedContent::from_str("a\nb\nc\nd\ne");
        let result = c.lines(2, 2);
        assert_eq!(result, vec!["c", "d"]);
    }

    #[test]
    fn lines_past_end() {
        let c = ChunkedContent::from_str("a\nb");
        let result = c.lines(0, 100);
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn lines_empty_content() {
        let c = ChunkedContent::new();
        let result = c.lines(0, 10);
        assert!(result.is_empty());
    }

    #[test]
    fn lines_count_zero() {
        let c = ChunkedContent::from_str("a\nb\nc");
        let result = c.lines(0, 0);
        assert!(result.is_empty());
    }

    // -- push_str / clear -----------------------------------------------------

    #[test]
    fn push_str_to_empty() {
        let mut c = ChunkedContent::new();
        c.push_str("hello");
        assert_eq!(c.to_string(), "hello");
    }

    #[test]
    fn push_str_appends() {
        let mut c = ChunkedContent::from_str("hello");
        c.push_str(" world");
        assert_eq!(c.to_string(), "hello world");
    }

    #[test]
    fn push_str_creates_new_chunks() {
        let mut c = ChunkedContent::from_str(&"a".repeat(CHUNK_SIZE));
        c.push_str(&"b".repeat(CHUNK_SIZE));
        assert_eq!(c.chunks.len(), 2);
        assert_eq!(c.to_string(), format!("{}{}", "a".repeat(CHUNK_SIZE), "b".repeat(CHUNK_SIZE)));
    }

    #[test]
    fn clear_resets() {
        let mut c = ChunkedContent::from_str("hello");
        c.clear();
        assert!(c.is_empty());
        assert_eq!(c.len(), 0);
        assert_eq!(c.to_string(), "");
    }

    // -- Serde round-trip (small content) -------------------------------------

    #[test]
    fn serde_roundtrip_small() {
        let c = ChunkedContent::from_str("hello world");
        let json = serde_json::to_string(&c).unwrap();
        // Small content serialises as a plain JSON string.
        assert_eq!(json, "\"hello world\"");
        let restored: ChunkedContent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.to_string(), "hello world");
    }

    // -- Serde round-trip (large content) -------------------------------------

    #[test]
    fn serde_roundtrip_large() {
        let s = "x".repeat(INLINE_THRESHOLD + 1);
        let c = ChunkedContent::from_str(&s);
        let json = serde_json::to_string(&c).unwrap();
        // Large content serialises as a structured object.
        assert!(json.contains("__chunked"));
        let restored: ChunkedContent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.to_string(), s);
    }

    // -- Serde: legacy plain string -------------------------------------------

    #[test]
    fn serde_deserialize_legacy_string() {
        let json = "\"legacy content\"";
        let c: ChunkedContent = serde_json::from_str(json).unwrap();
        assert_eq!(c.to_string(), "legacy content");
    }

    // -- Serde: null / missing ------------------------------------------------

    #[test]
    fn serde_deserialize_null() {
        let c: ChunkedContent = serde_json::from_str("null").unwrap();
        assert!(c.is_empty());
    }

    // -- FileNode helpers -----------------------------------------------------

    #[test]
    fn file_node_read_lines() {
        let f = FileNode {
            name: "test.txt".to_string(),
            content: ChunkedContent::from_str("a\nb\nc\nd\ne"),
        };
        assert_eq!(f.read_lines(1, 3), "b\nc\nd");
    }

    #[test]
    fn file_node_line_count() {
        let f = FileNode {
            name: "test.txt".to_string(),
            content: ChunkedContent::from_str("a\nb\nc"),
        };
        assert_eq!(f.line_count(), 3);
    }

    // -- FsNode helpers -------------------------------------------------------

    #[test]
    fn fs_node_name_and_set_name() {
        let mut node = FsNode::File(FileNode {
            name: "old.txt".to_string(),
            content: ChunkedContent::new(),
        });
        assert_eq!(node.name(), "old.txt");
        node.set_name("new.txt".to_string());
        assert_eq!(node.name(), "new.txt");
    }

    #[test]
    fn fs_node_is_dir() {
        let file = FsNode::File(FileNode {
            name: "f".to_string(),
            content: ChunkedContent::new(),
        });
        let dir = FsNode::Directory(DirNode {
            name: "d".to_string(),
            children: HashMap::new(),
        });
        assert!(!file.is_dir());
        assert!(dir.is_dir());
    }
}
