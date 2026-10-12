//! Semantic analysis: symbol tables, types, data layout and
//! parameter-passing conventions (CLAUDE.md section 4.2).  This phase owns
//! every ABI-relevant decision that comes from the *language*; the backend
//! only executes them.
//!
//! Stub: filled in by the semantics work.

use dpc_syntax::ast::CompilationUnit;
use dpc_syntax::Diagnostic;

/// The checked program handed to IR lowering.
#[derive(Debug, Default)]
pub struct Program {}

/// Check a parsed compilation unit.  Stub: accepts everything.
pub fn analyze(_unit: &CompilationUnit) -> Result<Program, Vec<Diagnostic>> {
    Ok(Program::default())
}

#[cfg(test)]
mod tests {
    #[test]
    fn analyze_empty_unit() {
        let unit = dpc_syntax::ast::CompilationUnit::default();
        assert!(super::analyze(&unit).is_ok());
    }
}
