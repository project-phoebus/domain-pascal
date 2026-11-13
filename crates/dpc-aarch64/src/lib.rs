//! The aarch64 target: Mach-O assembly that runs in user mode on macOS
//! (owner, 2026-11-12; CLAUDE.md 4.5; docs/aarch64.md).
//!
//! Unlike `dpc-m68k`, this backend reproduces nothing of Apollo's: it aims
//! at correct code under Apple's arm64 calling convention, so that Pascal
//! and C call each other directly.  The owner's decisions: pointers are 8
//! bytes, data is little-endian, arguments travel in x0-x7 and the stack
//! and results in x0.

pub mod emit;

use dpc_target::Target;

/// The aarch64 target description (macOS, Mach-O).
#[derive(Debug, Default, Clone, Copy)]
pub struct Aarch64;

impl Target for Aarch64 {
    fn name(&self) -> &'static str {
        "aarch64"
    }

    fn pointer_size(&self) -> u32 {
        8
    }

    /// Every argument takes one 8-byte slot, which is one of x0-x7 or a
    /// stack slot (docs/aarch64.md, "Calls").
    fn arg_slot_size(&self) -> u32 {
        8
    }

    fn arg_area_align(&self) -> u32 {
        8
    }

    /// Little-endian: a small value sits at its slot's lowest address.
    fn small_arg_offset_in_slot(&self, _size: u32) -> u32 {
        0
    }

    /// Argument 1's slot is right above the frame record (saved x29, x30):
    /// the callee stores its register arguments there on entry
    /// (docs/aarch64.md, "Frames").
    fn first_arg_offset(&self) -> i32 {
        16
    }

    /// Ref 5.5.7, p. 5-19: "four bytes or less in size (on 680x0
    /// workstations)".  Kept at 4 on aarch64 (Uncertain: the reference
    /// gives no other target's figure), so that `dpc-sema`'s result rules
    /// (a record of up to 4 bytes in a register, a larger one through a
    /// hidden address) carry over.  A pointer (8 bytes) still goes by
    /// value under `val_param`, as on the 680x0 (`dpc-sema` `param_conv`).
    fn val_param_max_size(&self) -> u32 {
        4
    }

    /// The reference's predeclared names (ref 4.7, p. 4-46), all false:
    /// this is neither an MC680x0 nor an Apollo a88k nor COFF.
    fn predeclared_cond_vars(&self) -> &'static [(&'static str, bool)] {
        &[
            ("_ISP__M68K", false),
            ("_ISP__A88K", false),
            ("_BFMT__COFF", false),
            ("isp_$m68k", false),
            ("isp_$a88k", false),
            ("obj_$coff", false),
            ("obj_$bin", false),
        ]
    }

    fn unaligned_access(&self) -> bool {
        true
    }

    fn big_endian(&self) -> bool {
        false
    }

    fn program_extern_cells(&self) -> bool {
        false
    }

    fn runtime_library(&self) -> Option<&'static str> {
        Some("libdpcrt.a")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_description() {
        let t: &dyn Target = &Aarch64;
        assert_eq!(t.name(), "aarch64");
        assert_eq!(t.pointer_size(), 8);
        assert_eq!(t.first_arg_offset(), 16);
        assert!(!t.big_endian());
        assert!(!t.program_extern_cells());
    }
}
