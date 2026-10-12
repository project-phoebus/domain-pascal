//! Recursive-descent parser.  Stub: returns an empty compilation unit.

use crate::ast::CompilationUnit;
use crate::diag::Diagnostic;
use crate::lexer::Token;

/// Parse the preprocessor's output ([`crate::preprocess::Output::tokens`],
/// which ends with `Eof`).
pub fn parse(_tokens: &[Token]) -> Result<CompilationUnit, Vec<Diagnostic>> {
    Ok(CompilationUnit::default())
}
