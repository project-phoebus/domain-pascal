//! Recursive-descent parser.  Stub: returns an empty compilation unit.

use crate::ast::CompilationUnit;
use crate::diag::Diagnostic;
use crate::source::FileId;

pub fn parse(_file: FileId, _text: &str) -> Result<CompilationUnit, Vec<Diagnostic>> {
    Ok(CompilationUnit::default())
}
