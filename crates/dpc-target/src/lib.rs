//! Target description (CLAUDE.md section 4.5).  ABI decisions that are not
//! fixed by the language (argument slot sizes, result registers, module-data
//! base, ...) are answered here, so the front end and the IR stay
//! target-neutral.
//!
//! Minimal for now; hooks are added as the backend needs them.

/// What the rest of the compiler may ask about a target.
pub trait Target {
    /// Short name, e.g. `"m68k"`.
    fn name(&self) -> &'static str;

    /// Size of a data pointer, in bytes.
    fn pointer_size(&self) -> u32;

    /// Smallest stack slot an argument occupies, in bytes.
    fn arg_slot_size(&self) -> u32;

    /// The total size of pushed arguments is padded to a multiple of this.
    fn arg_area_align(&self) -> u32;
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
    }

    #[test]
    fn trait_is_object_safe() {
        let t: &dyn Target = &Dummy;
        assert_eq!(t.name(), "dummy");
        assert_eq!(t.pointer_size(), 8);
    }
}
