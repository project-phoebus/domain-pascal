//! Diagnostics.  Rendered as `file:line:col: error: msg`.

use std::fmt;

use crate::source::{SourceMap, Span};

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
    /// message` when there is no location.
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
}
