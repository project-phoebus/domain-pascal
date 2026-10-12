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
    }

    #[test]
    fn trait_is_object_safe() {
        let t: &dyn Target = &Dummy;
        assert_eq!(t.name(), "dummy");
        assert_eq!(t.pointer_size(), 8);
        assert_eq!(t.first_arg_offset(), 16);
    }
}
