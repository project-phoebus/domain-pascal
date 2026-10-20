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
    analyze_with(unit, target, &Options::default())
}

/// Compiler options that semantic analysis follows.
#[derive(Debug, Default, Clone)]
pub struct Options {
    /// `-natural` (ref 6.4.22, pp. 6-25..6-26): the unit starts in natural
    /// alignment, "the same effect as the %natural_alignment compiler
    /// directive" (p. 6-26) at its start.  Default off (`-nnatural`).
    pub natural: bool,
}

/// [`analyze`] with compiler options.
pub fn analyze_with(
    unit: &CompilationUnit,
    target: &dyn Target,
    opts: &Options,
) -> Result<Program, Vec<Diagnostic>> {
    let mut c = check::Checker::new(target);
    c.set_natural(opts.natural);
    c.unit(unit)
}
