//! m68k backend: target description, instruction selection (to come) and
//! the gas-syntax emitter.  Output must follow the code patterns of Apollo's
//! SR10.2 compiler; see `docs/codegen-patterns.md`.

pub mod emit;

use dpc_target::Target;

/// The m68k target as used by the SR10.2 kernel (68010 instructions only,
/// CLAUDE.md 3.16).
#[derive(Debug, Default, Clone, Copy)]
pub struct M68k;

impl Target for M68k {
    fn name(&self) -> &'static str {
        "m68k"
    }

    fn pointer_size(&self) -> u32 {
        4
    }

    /// 16-bit values are pushed as 2 bytes; 8-bit values occupy the high
    /// byte of a 2-byte slot (CLAUDE.md 3.3; ../source/docs/pascal-abi.md).
    fn arg_slot_size(&self) -> u32 {
        2
    }

    /// A total that is not a multiple of 4 gets `subq.l #2,SP` before the
    /// pushes (CLAUDE.md 3.3).
    fn arg_area_align(&self) -> u32 {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_description() {
        let t: &dyn Target = &M68k;
        assert_eq!(t.name(), "m68k");
        assert_eq!(t.pointer_size(), 4);
        assert_eq!(t.arg_slot_size(), 2);
        assert_eq!(t.arg_area_align(), 4);
    }
}
