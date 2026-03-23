use std::path::Path;
use crate::error::IndexError;

/// Known binary file extensions to skip during indexing.
const BINARY_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "ico", "svg", "webp",
    "wasm", "exe", "dll", "so", "dylib", "o", "a", "lib",
    "zip", "gz", "tar", "bz2", "xz", "zst", "7z", "rar",
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
    "class", "pyc", "pyo", "whl", "egg",
    "ttf", "otf", "woff", "woff2", "eot",
    "mp3", "mp4", "avi", "mkv", "mov", "wav", "flac",
    "bin", "dat", "db", "sqlite", "sqlite3",
    "DS_Store",
];

/// Check if a file path has a known binary extension.
pub fn is_binary_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| BINARY_EXTENSIONS.iter().any(|&b| b.eq_ignore_ascii_case(ext)))
        .unwrap_or(false)
}

/// Check if content appears to be binary by looking for NUL bytes in the first 8KB.
pub fn is_binary_content(content: &[u8]) -> bool {
    let check_len = content.len().min(8192);
    content[..check_len].contains(&0u8)
}

/// Collected file entry from walking the directory tree.
pub struct FileEntry {
    pub relative_path: String,
    pub content: Vec<u8>,
}

/// Walk a directory tree, respecting .gitignore, and collect text file contents.
pub fn walk_files(root: &Path) -> Result<Vec<FileEntry>, IndexError> {
    let walker = ignore::WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    let mut entries = Vec::new();

    for result in walker {
        let dir_entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Skip directories
        if dir_entry.file_type().map_or(true, |ft| !ft.is_file()) {
            continue;
        }

        let path = dir_entry.path();

        // Skip binary extensions
        if is_binary_extension(path) {
            continue;
        }

        // Skip the .trigrep directory itself
        if path.components().any(|c| c.as_os_str() == ".trigrep") {
            continue;
        }

        // Read content
        let content = match std::fs::read(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Skip binary content
        if is_binary_content(&content) {
            continue;
        }

        let relative_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();

        entries.push(FileEntry {
            relative_path,
            content,
        });
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_extension_detection() {
        assert!(is_binary_extension(Path::new("image.png")));
        assert!(is_binary_extension(Path::new("lib.so")));
        assert!(is_binary_extension(Path::new("archive.ZIP")));
        assert!(!is_binary_extension(Path::new("code.rs")));
        assert!(!is_binary_extension(Path::new("readme.md")));
    }

    #[test]
    fn test_binary_content_detection() {
        assert!(is_binary_content(&[0x00, 0x01, 0x02]));
        assert!(!is_binary_content(b"hello world"));
        assert!(!is_binary_content(b"fn main() {}"));
    }
}
