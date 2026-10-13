//! Source files and positions.  Every token and AST node carries a [`Span`]
//! so diagnostics can point at the source (CLAUDE.md 4.1).

use std::path::{Path, PathBuf};
use std::rc::Rc;

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
    /// The file's text.  Shared so a lexer can hold it while the
    /// [`SourceMap`] grows (the preprocessor loads `%include`s mid-lex).
    pub text: Rc<str>,
    /// Byte offset of the start of each line.
    line_starts: Vec<u32>,
    /// The `%include` directive that loaded this file, `None` for the main
    /// file (for "in file included from" notes).
    pub included_from: Option<Span>,
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
            text: text.into(),
            line_starts,
            included_from: None,
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

    /// Byte range `[start, end)` of 1-based line `line`, without its line
    /// terminator (`\n` or `\r\n`).
    pub fn line_range(&self, line: u32) -> (usize, usize) {
        let i = line as usize - 1;
        let start = self.line_starts[i] as usize;
        let mut end = self
            .line_starts
            .get(i + 1)
            .map_or(self.text.len(), |&s| s as usize - 1);
        if end > start && self.text.as_bytes()[end - 1] == b'\r' {
            end -= 1;
        }
        (start, end)
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

    /// Record that file `id` was loaded by the `%include` directive at
    /// `from`.
    pub fn set_included_from(&mut self, id: FileId, from: Span) {
        self.files[id.0 as usize].included_from = Some(from);
    }

    /// The `%include` directives that led to file `id`, innermost first
    /// (empty for the main file).
    pub fn include_chain(&self, id: FileId) -> Vec<Span> {
        let mut chain = Vec::new();
        let mut cur = self.file(id).included_from;
        while let Some(span) = cur {
            chain.push(span);
            cur = self.file(span.file).included_from;
        }
        chain
    }

    pub fn file(&self, id: FileId) -> &SourceFile {
        &self.files[id.0 as usize]
    }

    /// The source text a span covers (e.g. an identifier's original
    /// spelling).
    pub fn span_text(&self, span: Span) -> &str {
        &self.file(span.file).text[span.lo as usize..span.hi as usize]
    }

    /// 1-based line of a span's start.
    pub fn line_of(&self, span: Span) -> u32 {
        self.file(span.file).line_col(span.lo).line
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

    #[test]
    fn line_ranges() {
        let mut sm = SourceMap::new();
        let id = sm.add("t.pas", "ab\r\ncd\n\nx".to_string());
        let f = sm.file(id);
        assert_eq!(f.line_range(1), (0, 2));
        assert_eq!(f.line_range(2), (4, 6));
        assert_eq!(f.line_range(3), (7, 7));
        assert_eq!(f.line_range(4), (8, 9));
    }

    #[test]
    fn include_chain_innermost_first() {
        let mut sm = SourceMap::new();
        let m = sm.add("m.pas", "%include 'a';".to_string());
        let a = sm.add("a", "%include 'b';".to_string());
        let b = sm.add("b", String::new());
        sm.set_included_from(a, Span::new(m, 0, 8));
        sm.set_included_from(b, Span::new(a, 0, 8));
        assert!(sm.include_chain(m).is_empty());
        assert_eq!(
            sm.include_chain(b),
            [Span::new(a, 0, 8), Span::new(m, 0, 8)]
        );
    }
}
