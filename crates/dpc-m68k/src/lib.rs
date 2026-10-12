//! m68k backend: target description, instruction selection and the
//! gas-syntax emitter.  Output must follow the code patterns of Apollo's
//! SR10.2 compiler; see `docs/codegen-patterns.md`.
//!
//! - [`isel`]: IR -> [`insn::Insn`] over virtual registers, pattern by
//!   pattern, body first;
//! - [`regs`]: liveness and register assignment (codegen-patterns 12.4
//!   R1-R3), and so which callee-saved registers the body writes;
//! - [`frame`]: frame layout, prologue and epilogue around the body;
//! - [`insn`]: instructions and their gas text;
//! - [`emit`]: a whole module as an assembly file.

pub mod emit;
pub mod frame;
pub mod insn;
pub mod isel;
pub mod regs;

use dpc_target::Target;

/// The m68k target as used by the SR10.2 kernel (68010 instructions only,
/// CLAUDE.md 3.16).
#[derive(Debug, Default, Clone, Copy)]
pub struct M68k;

impl M68k {
    /// The `link.w A6,#-n` size for a frame of `raw` bytes
    /// (`dpc_sema::Routine::frame_size`): the total rounded up to a
    /// multiple of 4.
    ///
    /// Evidence (codegen-patterns 2): all 1171 `link.w A6,#d` at routine
    /// starts named in the map have `d mod 4 = 0`; PROC1_$TST_LOCK
    /// 0xE148CA has `link #-4` for a 1-byte frame (its boolean result
    /// variable), and TIME_$Q_INIT_QUEUE 0xE16C5E / LOG_$READ 0xE17800
    /// have `link #0` with no locals.  **Uncertain:** whether the original
    /// rounded the total or gave each frame object a 4-byte-aligned slot;
    /// the two differ once a routine has several small locals.
    pub fn link_size(raw: u32) -> u32 {
        raw.div_ceil(4) * 4
    }
}

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

    /// An 8-bit value pushed with `move.b x,-(SP)` lands in the HIGH
    /// (even-address) byte of its 2-byte slot: TIME_$Q_INIT_QUEUE 0xE16C62
    /// `move.b (0x8,A6),D0b` (codegen-patterns 1.3).
    fn small_arg_offset_in_slot(&self, _size: u32) -> u32 {
        0
    }

    /// Return address (4) + saved A6 (4): argument 1 is at `(8,A6)`
    /// (codegen-patterns 1.1; PROC1_$TST_LOCK 0xE148D4 `move.w (0x8,A6),D0w`).
    fn first_arg_offset(&self) -> i32 {
        8
    }

    /// Ref 5.5.7, p. 5-19: "four bytes or less in size (on 680x0
    /// workstations)".
    fn val_param_max_size(&self) -> u32 {
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
        assert_eq!(t.small_arg_offset_in_slot(1), 0);
        assert_eq!(t.first_arg_offset(), 8);
        assert_eq!(t.val_param_max_size(), 4);
    }

    #[test]
    fn link_size_rounds_total_to_4() {
        assert_eq!(M68k::link_size(0), 0);
        assert_eq!(M68k::link_size(1), 4);
        assert_eq!(M68k::link_size(4), 4);
        assert_eq!(M68k::link_size(6), 8);
    }
}
