//! Source files and positions.  Every token and AST node carries a [`Span`]
//! so diagnostics can point at the source (CLAUDE.md 4.1).

use std::path::{Path, PathBuf};

/// Index of a file in a [`SourceMap`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FileId(u32);

/// A half-open byte range `[lo, hi)` in one source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    pub file: FileId,
    pub lo: u32,
    pub hi: u32,
}

impl Span {
    pub fn new(file: FileId, lo: u32, hi: u32) -> Self {
        debug_assert!(lo <= hi);
        Span { file, lo, hi }
    }
}

/// 1-based line and column (column counts bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

/// One loaded source file.
#[derive(Debug)]
pub struct SourceFile {
    pub path: PathBuf,
    pub text: String,
    /// Byte offset of the start of each line.
    line_starts: Vec<u32>,
}

impl SourceFile {
    fn new(path: PathBuf, text: String) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(
            text.bytes()
                .enumerate()
                .filter(|&(_, b)| b == b'\n')
                .map(|(i, _)| i as u32 + 1),
        );
        SourceFile {
            path,
            text,
            line_starts,
        }
    }

    /// Line and column of a byte offset.
    pub fn line_col(&self, offset: u32) -> LineCol {
        let line = self.line_starts.partition_point(|&s| s <= offset) - 1;
        LineCol {
            line: line as u32 + 1,
            col: offset - self.line_starts[line] + 1,
        }
    }
}

/// All files of one compilation (the main file and its `%include`s).
#[derive(Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a file whose text is already in memory.
    pub fn add(&mut self, path: impl Into<PathBuf>, text: String) -> FileId {
        self.files.push(SourceFile::new(path.into(), text));
        FileId(self.files.len() as u32 - 1)
    }

    /// Read a file from disk and add it.
    pub fn load(&mut self, path: &Path) -> std::io::Result<FileId> {
        let text = std::fs::read_to_string(path)?;
        Ok(self.add(path, text))
    }

    pub fn file(&self, id: FileId) -> &SourceFile {
        &self.files[id.0 as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_lookup() {
        let mut sm = SourceMap::new();
        let id = sm.add("t.pas", "ab\ncd\n\nx".to_string());
        let f = sm.file(id);
        assert_eq!(f.line_col(0), LineCol { line: 1, col: 1 });
        assert_eq!(f.line_col(1), LineCol { line: 1, col: 2 });
        assert_eq!(f.line_col(3), LineCol { line: 2, col: 1 });
        assert_eq!(f.line_col(6), LineCol { line: 3, col: 1 });
        assert_eq!(f.line_col(7), LineCol { line: 4, col: 1 });
    }
}
