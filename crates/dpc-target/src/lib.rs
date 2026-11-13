//! Target description (CLAUDE.md section 4.5).  ABI decisions that are not
//! fixed by the language (argument slot sizes, result registers, module-data
//! base, ...) are answered here, so the front end and the IR stay
//! target-neutral.
//!
//! Semantic analysis asks these questions when it lays out argument areas
//! and decides which parameters travel by value (crate `dpc-sema`).

/// What the rest of the compiler may ask about a target.
pub trait Target {
    /// Short name, e.g. `"m68k"`.
    fn name(&self) -> &'static str;

    /// Size of a data pointer, in bytes.  Also the size of an argument
    /// passed by address.
    fn pointer_size(&self) -> u32;

    /// Smallest stack slot an argument occupies, in bytes.  A by-value
    /// argument's slot is its size rounded up to a multiple of this.
    fn arg_slot_size(&self) -> u32;

    /// The total size of pushed arguments is padded to a multiple of this.
    fn arg_area_align(&self) -> u32;

    /// Where a by-value argument smaller than its slot sits inside the
    /// slot, as a byte offset from the slot's lowest address.
    fn small_arg_offset_in_slot(&self, size: u32) -> u32;

    /// Frame offset of the first (leftmost) argument's slot, relative to
    /// the callee's frame pointer after its prologue.  Later arguments
    /// follow at increasing offsets (arguments are pushed right to left).
    fn first_arg_offset(&self) -> i32;

    /// Largest value and `in` parameter that `val_param` / `internal`
    /// routines receive by value (ref 5.5.7, p. 5-19: "four bytes or less
    /// in size (on 680x0 workstations)").
    fn val_param_max_size(&self) -> u32;

    /// The predeclared conditional variables (ref 4.7, p. 4-46) and their
    /// values, e.g. `("_ISP__M68K", true)`.  Names are case-insensitive,
    /// like all conditional variables.  The driver hands them to the
    /// preprocessor (`dpc_syntax::preprocess::Options::predeclared`),
    /// which refuses `%enable` and `-config` of them (p. 4-48).
    fn predeclared_cond_vars(&self) -> &'static [(&'static str, bool)];

    /// Whether a word or longword may be referenced at an odd address.
    /// The MC68010 traps on one, so a reference to a byte-aligned object
    /// (`aligned(0)`, ref 3.15.6.3) of more than one byte at an address
    /// not known to be even is refused (`dpc-ir` `check_even`); the
    /// MC68020 allows it, and the SAU9 kernel has such references
    /// (codegen-patterns 14, "Odd addresses"; bead domain-pascal-gkrx).
    fn unaligned_access(&self) -> bool {
        false
    }

    /// Whether the target stores multi-byte integers most significant byte
    /// first.  The m68k is big-endian; aarch64 is little-endian (owner,
    /// 2026-11-12; docs/aarch64.md).  The language fixes some byte images
    /// whatever the target (a set larger than 4 bytes, ref 3.9.3 Figure
    /// 3-6; packed record fields, ref 3.10.7; strings); integers, pointers
    /// and register-sized sets follow this.  Default: big-endian.
    fn big_endian(&self) -> bool {
        true
    }

    /// Whether a main program reaches other units' variables through
    /// address cells below its data base, as Apollo's compiler does
    /// (codegen-patterns 4, "Main programs"; `dpc-ir` `extern_cell`),
    /// rather than by their symbols.  Default: true (m68k).
    fn program_extern_cells(&self) -> bool {
        true
    }

    /// The file name of the target's prebuilt run-time library, the
    /// object compiled code is linked with (like gcc's `libgcc.a`), or
    /// None when the target has none.  The driver looks for it in the
    /// runtime directory under [`Target::name`] (`dpc --print-runtime`;
    /// runtime/README.md).  Default: none.
    fn runtime_library(&self) -> Option<&'static str> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::Target;

    struct Dummy;

    impl Target for Dummy {
        fn name(&self) -> &'static str {
            "dummy"
        }
        fn pointer_size(&self) -> u32 {
            8
        }
        fn arg_slot_size(&self) -> u32 {
            8
        }
        fn arg_area_align(&self) -> u32 {
            16
        }
        fn small_arg_offset_in_slot(&self, _size: u32) -> u32 {
            0
        }
        fn first_arg_offset(&self) -> i32 {
            16
        }
        fn val_param_max_size(&self) -> u32 {
            8
        }
        fn predeclared_cond_vars(&self) -> &'static [(&'static str, bool)] {
            &[("_dummy", true)]
        }
    }

    #[test]
    fn trait_is_object_safe() {
        let t: &dyn Target = &Dummy;
        assert_eq!(t.name(), "dummy");
        assert_eq!(t.pointer_size(), 8);
        assert_eq!(t.first_arg_offset(), 16);
        assert_eq!(t.predeclared_cond_vars(), [("_dummy", true)]);
        assert_eq!(t.runtime_library(), None);
        assert!(t.big_endian());
    }
}
