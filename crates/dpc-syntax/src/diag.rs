//! Diagnostics.  Rendered as `file:line:col: error: msg`, optionally
//! followed by the source line with the span marked, rustc/clang style
//! ([`Diagnostic::render_with_source`]).

use std::cell::Cell;
use std::fmt;

use crate::source::{SourceFile, SourceMap, Span};

/// Tab stops in the rendered source line.
const TAB_WIDTH: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
        })
    }
}

thread_local! {
    static CONTEXT: Cell<Option<Span>> = const { Cell::new(None) };
    static CONTEXT_HOOK: Cell<Option<fn(Span)>> = const { Cell::new(None) };
}

/// Records `span` as the construct being compiled (a routine, by the
/// checker, the lowering and the selector) and returns the one recorded
/// before.  Each phase restores the one before when the routine is done,
/// so outside every routine it is `None`.  Only the driver's
/// internal-error backstop reads it ([`context`]): a panic is reported at
/// the routine it happened in.
pub fn set_context(span: Option<Span>) -> Option<Span> {
    let outer = CONTEXT.with(|c| c.replace(span));
    if let (Some(span), Some(hook)) = (span, CONTEXT_HOOK.with(Cell::get)) {
        hook(span);
    }
    outer
}

/// Test only: `hook` is called with each span [`set_context`] records
/// from now on, on this thread, after recording it.  The driver's
/// `DPC_INTERNAL_PANIC_FOR_TEST` uses it to panic inside a routine, to
/// test its internal-error backstop end to end.
pub fn set_context_hook(hook: Option<fn(Span)>) {
    CONTEXT_HOOK.with(|h| h.set(hook));
}

/// The span [`set_context`] last recorded on this thread.
pub fn context() -> Option<Span> {
    CONTEXT.with(Cell::get)
}

/// A message, optionally tied to a source location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub span: Option<Span>,
    pub message: String,
}

impl Diagnostic {
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Error,
            span: Some(span),
            message: message.into(),
        }
    }

    pub fn warning(span: Span, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            span: Some(span),
            message: message.into(),
        }
    }

    /// An error with no source location (e.g. a file that cannot be read).
    pub fn error_nopos(message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Error,
            span: None,
            message: message.into(),
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Render as `file:line:col: severity: message`, or `prog: severity:
    /// message` when there is no location.  One line, for tests and
    /// tools; the driver prints [`render_with_source`](Self::render_with_source).
    pub fn render(&self, sm: &SourceMap, prog: &str) -> String {
        match self.span {
            Some(span) => {
                let file = sm.file(span.file);
                let lc = file.line_col(span.lo);
                format!(
                    "{}:{}:{}: {}: {}",
                    file.path.display(),
                    lc.line,
                    lc.col,
                    self.severity,
                    self.message
                )
            }
            None => format!("{}: {}: {}", prog, self.severity, self.message),
        }
    }

    /// [`render`](Self::render), then (when there is a location) the
    /// source line with a gutter and a marker line, `^` under the span's
    /// first column and `~` across the rest of it, and one `note: in file
    /// included from FILE:LINE:COL` line per enclosing `%include`,
    /// innermost first.  A span running past its first line is marked to
    /// the end of that line.  The first line is unchanged, so tools can
    /// keep parsing it.
    pub fn render_with_source(&self, sm: &SourceMap, prog: &str) -> String {
        let mut out = self.render(sm, prog);
        let Some(span) = self.span else {
            return out;
        };
        out.push('\n');
        out.push_str(&snippet(sm.file(span.file), span));
        for inc in sm.include_chain(span.file) {
            let file = sm.file(inc.file);
            let lc = file.line_col(inc.lo);
            out.push_str(&format!(
                "\nnote: in file included from {}:{}:{}",
                file.path.display(),
                lc.line,
                lc.col
            ));
        }
        out
    }
}

/// The span's first line, with an `N | ` gutter, and a marker line under
/// it.  Tabs are expanded to [`TAB_WIDTH`] stops and every other character
/// counts one column, in both lines, so the marker lines up.
fn snippet(file: &SourceFile, span: Span) -> String {
    let len = file.text.len();
    let lo = (span.lo as usize).min(len);
    let line = file.line_col(lo as u32).line;
    let (start, end) = file.line_range(line);
    let hi = (span.hi as usize).clamp(lo, end.max(lo));
    let mut shown = String::new();
    // Visual columns (0-based) of `lo` and `hi`; past the line's end they
    // stay at its width.
    let (mut vlo, mut vhi) = (None, None);
    let mut col = 0;
    for (i, c) in file.text[start..end].char_indices() {
        // Offsets are bytes; one inside a multi-byte character (not
        // produced by the lexer, but harmless) widens to that character.
        let at = start + i;
        if vlo.is_none() && at + c.len_utf8() > lo {
            vlo = Some(col);
        }
        if vhi.is_none() && at >= hi {
            vhi = Some(col);
        }
        if c == '\t' {
            let n = TAB_WIDTH - col % TAB_WIDTH;
            shown.extend(std::iter::repeat_n(' ', n));
            col += n;
        } else {
            shown.push(c);
            col += 1;
        }
    }
    let vlo = vlo.unwrap_or(col);
    let vhi = vhi.unwrap_or(col).max(vlo + 1);
    let num = line.to_string();
    let pad = " ".repeat(num.len());
    let marker = format!("{}^{}", " ".repeat(vlo), "~".repeat(vhi - vlo - 1));
    let text = format!("{num} | {shown}");
    format!("{}\n{pad} | {marker}", text.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_with_and_without_position() {
        let mut sm = SourceMap::new();
        let id = sm.add("m.pas", "module m;\n  bad\n".to_string());
        let d = Diagnostic::error(Span::new(id, 12, 15), "unexpected identifier");
        assert_eq!(
            d.render(&sm, "dpc"),
            "m.pas:2:3: error: unexpected identifier"
        );
        let d = Diagnostic::error_nopos("no input file");
        assert_eq!(d.render(&sm, "dpc"), "dpc: error: no input file");
    }

    fn full(src: &str, lo: u32, hi: u32) -> String {
        let mut sm = SourceMap::new();
        let id = sm.add("m.pas", src.to_string());
        Diagnostic::error(Span::new(id, lo, hi), "msg").render_with_source(&sm, "dpc")
    }

    #[test]
    fn source_single_char_and_empty_span() {
        let want = "m.pas:2:3: error: msg\n2 |   x := 1;\n  |   ^";
        assert_eq!(full("module m;\n  x := 1;\n", 12, 13), want);
        // An empty span still gets one caret.
        assert_eq!(full("module m;\n  x := 1;\n", 12, 12), want);
    }

    #[test]
    fn source_multi_char_span() {
        assert_eq!(
            full("module m;\n  nosuch := 1;\n", 12, 18),
            "m.pas:2:3: error: msg\n2 |   nosuch := 1;\n  |   ^~~~~~"
        );
    }

    #[test]
    fn source_tabs_expand_in_line_and_marker() {
        // `col` stays byte-based (4); the marker is at visual column 17.
        assert_eq!(
            full("a\tb\tcd := 1;\n", 4, 6),
            "m.pas:1:5: error: msg\n1 | a       b       cd := 1;\n  |                 ^~"
        );
    }

    #[test]
    fn source_non_ascii_counts_chars() {
        // "é" is two bytes but one column.
        assert_eq!(
            full("{é} x\n", 5, 6),
            "m.pas:1:6: error: msg\n1 | {é} x\n  |     ^"
        );
    }

    #[test]
    fn source_offsets_inside_a_character_widen_to_it() {
        // Byte 2 is inside "é" (bytes 1..3); hi 4 is inside "ü" (3..5).
        assert_eq!(
            full("xéüy\n", 2, 4),
            "m.pas:1:3: error: msg\n1 | xéüy\n  |  ^~"
        );
    }

    #[test]
    fn source_never_panics_on_odd_spans() {
        // Deterministic pseudo-random spans (including past EOF and inside
        // multi-byte characters) over text with tabs, CRLF, non-ASCII and
        // no final newline.
        let pieces = ["a", "\t", "\n", "\r\n", "é", "日本", "  ", "%include", "\r"];
        let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        for _ in 0..300 {
            let n = (next() % 40) as usize;
            let src: String = (0..n)
                .map(|_| pieces[(next() % pieces.len() as u64) as usize])
                .collect();
            let mut sm = SourceMap::new();
            let id = sm.add("f.pas", src.clone());
            for _ in 0..20 {
                let lo = (next() % (src.len() as u64 + 4)) as u32;
                let hi = lo + (next() % 12) as u32;
                let d = Diagnostic::error(Span::new(id, lo, hi), "m");
                let out = d.render_with_source(&sm, "dpc");
                assert_eq!(out.lines().next(), Some(d.render(&sm, "dpc").as_str()));
            }
        }
    }

    #[test]
    fn source_multi_line_span_is_clipped() {
        assert_eq!(
            full("begin\n  if a\n  then b;\n", 8, 20),
            "m.pas:2:3: error: msg\n2 |   if a\n  |   ^~~~"
        );
    }

    #[test]
    fn source_end_of_line_and_file() {
        // At a line's end: a caret just past it.
        assert_eq!(
            full("module m\nx", 8, 8),
            "m.pas:1:9: error: msg\n1 | module m\n  |         ^"
        );
        // At EOF after a final newline: the empty last line.
        assert_eq!(
            full("module m;\n", 10, 10),
            "m.pas:2:1: error: msg\n2 |\n  | ^"
        );
        // At EOF without one, and CRLF line ends.
        assert_eq!(
            full("a\r\nend", 6, 6),
            "m.pas:2:4: error: msg\n2 | end\n  |    ^"
        );
    }

    #[test]
    fn source_gutter_widens_with_line_number() {
        let src = format!("{}bad\n", "\n".repeat(11));
        assert_eq!(
            full(&src, 11, 14),
            "m.pas:12:1: error: msg\n12 | bad\n   | ^~~"
        );
    }

    #[test]
    fn source_include_chain_and_warning() {
        let mut sm = SourceMap::new();
        let m = sm.add(
            "main.pas",
            "module m;\n\n%include 'a.ins.pas';\n".to_string(),
        );
        let a = sm.add("a.ins.pas", "  %include 'b.ins.pas';\n".to_string());
        let b = sm.add("b.ins.pas", "const k = ;\n".to_string());
        sm.set_included_from(a, Span::new(m, 11, 19));
        sm.set_included_from(b, Span::new(a, 2, 10));
        let d = Diagnostic::warning(Span::new(b, 10, 11), "odd");
        assert_eq!(
            d.render_with_source(&sm, "dpc"),
            "b.ins.pas:1:11: warning: odd\n\
             1 | const k = ;\n\
             \x20 |           ^\n\
             note: in file included from a.ins.pas:1:3\n\
             note: in file included from main.pas:3:1"
        );
        // No position: unchanged.
        let d = Diagnostic::error_nopos("no input file");
        assert_eq!(
            d.render_with_source(&sm, "dpc"),
            "dpc: error: no input file"
        );
    }
}
