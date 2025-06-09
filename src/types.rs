use std::path::{Path, PathBuf};

/// Represents a successfully resolved file, ready for inclusion.
///
/// It stores the path intended for display to the user (and in the Markdown header)
/// and the canonicalized, absolute path for robust duplicate checking and file reading.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResolvedFile {
    // Path to show to the user and use in Markdown header (e.g., relative to PWD).
    pub(crate) display_path: PathBuf,
    // Absolute, canonicalized path for uniqueness checks and reading the file.
    pub(crate) canonical_path: PathBuf,
}

impl ResolvedFile {
    /// Creates a new ResolvedFile.
    /// Used by `file_resolver.rs` after successful canonicalization and path diffing.
    pub(crate) fn new(display_path: PathBuf, canonical_path: PathBuf) -> Self {
        Self {
            display_path,
            canonical_path,
        }
    }

    /// Returns the path suitable for display to the user.
    pub fn display_path(&self) -> &Path {
        &self.display_path
    }

    /// Returns the canonical, absolute path to the file.
    pub fn canonical_path(&self) -> &Path {
        &self.canonical_path
    }
}

/// Represents a single, tagged symbol extracted from a source file.
/// This structure is designed to mirror the kind of information provided
/// by the `tree-sitter tags` CLI command.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Tag {
    /// The name of the symbol (e.g., the function or struct name).
    pub name: String,
    /// The kind of symbol (e.g., "function", "method", "class").
    pub kind: String,
    /// The byte offset where the symbol's definition starts. Used for sorting.
    pub start_byte: usize,
    /// The full first line of the symbol's definition.
    pub line_text: String,
    /// An optional docstring associated with the symbol.
    pub doc_string: Option<String>,
}

// Implement ordering traits to allow sorting by position in the source file.
impl Ord for Tag {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.start_byte.cmp(&other.start_byte)
    }
}

impl PartialOrd for Tag {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Represents the outcome of processing a single user input string.
/// It is generic over a lifetime `'a` to borrow the input string, avoiding allocations.
#[derive(Debug, Clone)]
pub enum InputResolution<'a> {
    /// Successfully resolved to one or more files.
    /// This could be a single file match or the expansion of a directory.
    Success(Vec<ResolvedFile>),

    /// The input string led to multiple conflicting matches, making it ambiguous.
    Ambiguous {
        input_string: &'a str,
        /// Paths (typically relative to PWD for display) that caused the ambiguity.
        conflicting_paths: Vec<PathBuf>,
    },

    /// The input string could not be found after searching.
    NotFound { input_string: &'a str },

    /// The input string was treated as an explicit path, but it does not exist on the filesystem.
    PathDoesNotExist {
        input_string: &'a str,
        /// The absolute or relative path that was checked.
        path_tried: PathBuf,
    },
    // Consider adding a more generic `ResolutionError` variant if finer-grained
    // error reporting from the resolver becomes necessary, e.g., for permission errors
    // encountered when trying to resolve a specific file that wasn't a general WalkDir error.
    // For V1, the above should cover the main scenarios.
}
