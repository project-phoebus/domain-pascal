//! Front end of the Domain Pascal compiler: source positions, diagnostics,
//! the `%` directive preprocessor, the lexer, the parser and the AST
//! (CLAUDE.md section 4.1).

pub mod ast;
pub mod diag;
pub mod lexer;
pub mod parser;
pub mod preprocess;
pub mod source;

pub use diag::{Diagnostic, Severity};
pub use source::{FileId, LineCol, SourceFile, SourceMap, Span};
