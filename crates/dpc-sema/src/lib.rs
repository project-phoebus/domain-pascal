//! Semantic analysis: symbol tables, types, data layout and
//! parameter-passing conventions (CLAUDE.md section 4.2).  This phase owns
//! every ABI-relevant decision that comes from the *language*; the backend
//! only executes them.  Numbers that come from the ABI rather than the
//! language are asked of [`dpc_target::Target`] (CLAUDE.md 4.5).
//!
//! - [`types`]: types and their layout (ref chapter 3);
//! - [`conv`]: parameter and result conventions (ref 5.2 Table 5-1, 5.5);
//! - [`program`]: the annotated program handed to IR lowering;
//! - `check`: the checker that builds it.

mod check;
pub mod consts;
pub mod conv;
pub mod program;
pub mod types;

#[cfg(test)]
mod tests;

use dpc_syntax::ast::CompilationUnit;
use dpc_syntax::Diagnostic;
use dpc_target::Target;

pub use program::*;

/// Check a parsed compilation unit for `target`.  On success the program
/// carries any warnings; on failure every error is returned, with its
/// source position.
pub fn analyze(unit: &CompilationUnit, target: &dyn Target) -> Result<Program, Vec<Diagnostic>> {
    check::Checker::new(target).unit(unit)
}
