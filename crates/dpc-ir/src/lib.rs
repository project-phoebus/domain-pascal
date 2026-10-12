//! Low-level, per-routine IR (CLAUDE.md section 4.3).  Frame slots,
//! argument slots, pads and module-data statics are explicit here.
//!
//! Target seam (CLAUDE.md 4.5): this crate must not name machine registers
//! or otherwise assume m68k.
//!
//! Stub: filled in by the codegen work.

use dpc_sema::Program;

/// One compilation unit's worth of IR.
#[derive(Debug, Default)]
pub struct Module {}

/// Lower a checked program to IR.  Stub: produces an empty module.
pub fn lower(_program: &Program) -> Module {
    Module::default()
}

#[cfg(test)]
mod tests {
    #[test]
    fn lower_empty_program() {
        let _m = super::lower(&dpc_sema::Program::default());
    }
}
