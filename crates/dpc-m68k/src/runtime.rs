//! The m68k run-time library (runtime/README.md), built into
//! `target/runtime/m68k/libm_arith.a`: transcriptions of the image's
//! compiler run-time segments ([`MEMBERS`]).
//!
//! - M$ARITH, the Domain Pascal compiler's arithmetic routines (CLAUDE.md
//!   3.11; codegen-patterns 8), `runtime/m68k/m_arith.s`.  The instruction
//!   selector names an entry only through [`marith_entry`], so it can name
//!   only routines the shipped module defines.
//! - PAS_$SET_BUILD, the set builder (codegen-patterns 5.1),
//!   `runtime/m68k/pas_set_build.s`.  dpc never calls it ([`UNCALLED`]): a
//!   set constructor with a run-time range is refused (dpc-ir `set_value`).
//!   It is shipped for links of the kernel (bead domain-pascal-jzm5).

/// The map module: `I E0ABD4 M$ARITH size = 1DC` (SAU2 map).
pub const MODULE: &str = "M$ARITH";

/// The library file under `<runtime dir>/m68k/` ([`dpc_target::Target::runtime_library`]).
/// It keeps its first member's name; it holds every member of [`MEMBERS`].
pub const LIBRARY: &str = "libm_arith.a";

/// The library's members, in archive order: (object, map segment, image
/// address, size), each the whole SAU2 map segment `I <address> <segment>
/// size = <size>`, in the object's section `.text.<segment>`.
pub const MEMBERS: [(&str, &str, u32, u32); 2] = [
    ("m_arith.o", "M$ARITH", 0xE0ABD4, 0x1DC),
    ("pas_set_build.o", "PAS_$SET_BUILD", 0xE11FA8, 0x4C),
];

/// Global names of the library that dpc never emits a call to, with their
/// map addresses: `I E11FA8 PAS_$SET_BUILD size = 4C` has no caller in the
/// image, so the code a compiler generates for its call is not catalogued
/// and the set constructor that would need it is refused.
pub const UNCALLED: [(&str, u32); 1] = [("PAS_$SET_BUILD", 0xE11FA8)];

/// The module's entry points in image order, with their map addresses.
/// `M$OIS$LLL` and `M$OIU$LLL` are one routine (both at 0xE0AD7C).
/// `M$OIS$WWL` has no caller in the image's compiled code but is part of
/// the module, which was linked whole.
pub const ENTRIES: [(&str, u32); 13] = [
    ("M$MIS$LLL", 0xE0ABD4),
    ("M$MIU$LLW", 0xE0AC02),
    ("M$MIU$LLL", 0xE0AC1C),
    ("M$MIS$LLW", 0xE0AC44),
    ("M$DIU$LLW", 0xE0AC68),
    ("M$DIS$LLW", 0xE0AC84),
    ("M$OIU$WLW", 0xE0ACC0),
    ("M$DIS$LLL", 0xE0ACD8),
    ("M$DIU$LLL", 0xE0AD0C),
    ("M$OIS$WLW", 0xE0AD54),
    ("M$OIS$WWL", 0xE0AD68),
    ("M$OIS$LLL", 0xE0AD7C),
    ("M$OIU$LLL", 0xE0AD7C),
];

/// The operations the selector asks for: `MI` multiply, `DI` divide, `OI`
/// modulo.
pub const KINDS: [&str; 3] = ["MI", "DI", "OI"];

/// The operand forms the selector asks for, result-operand1-operand2
/// (`L` longword, `W` word): `LLW`, `LLL`, `WLW`.
pub const FORMS: [&str; 3] = ["LLW", "LLL", "WLW"];

/// The entry `M$<kind><S|U>$<form>`, if the module has it: an operation the
/// module lacks has no name (the selector refuses it).
pub fn marith_entry(kind: &str, signed: bool, form: &str) -> Option<&'static str> {
    let s = if signed { 'S' } else { 'U' };
    let name = format!("M${kind}{s}${form}");
    ENTRIES.iter().map(|&(n, _)| n).find(|&n| n == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped module's source.
    const SOURCE: &str = include_str!("../../../runtime/m68k/m_arith.s");

    /// PAS_$SET_BUILD's source.
    const SET_BUILD: &str = include_str!("../../../runtime/m68k/pas_set_build.s");

    /// The `.global` names of a source, in source order.
    fn globals_of(src: &'static str) -> Vec<&'static str> {
        src.lines()
            .filter_map(|l| l.trim().strip_prefix(".global"))
            .map(str::trim)
            .collect()
    }

    /// The `.global` names of the shipped M$ARITH module, in source order.
    fn source_globals() -> Vec<&'static str> {
        globals_of(SOURCE)
    }

    #[test]
    fn members_are_the_shipped_sources() {
        assert_eq!(MEMBERS[0].1, MODULE);
        for ((obj, seg, ..), src) in MEMBERS.iter().zip([SOURCE, SET_BUILD]) {
            assert!(obj.ends_with(".o"), "{obj}");
            assert!(
                src.contains(&format!(".section \".text.{seg}\"")),
                "{obj}: section .text.{seg}"
            );
        }
        // the Makefile archives exactly these, in this order
        let srcs: Vec<String> = MEMBERS
            .iter()
            .map(|(obj, ..)| format!("runtime/m68k/{}.s", obj.trim_end_matches(".o")))
            .collect();
        let line = format!("RUNTIME_SRCS := {}", srcs.join(" "));
        assert!(
            include_str!("../../../Makefile").lines().any(|l| l == line),
            "Makefile: {line}"
        );
    }

    #[test]
    fn uncalled_entries_are_set_builds_globals() {
        let names: Vec<_> = UNCALLED.iter().map(|&(n, _)| n).collect();
        assert_eq!(globals_of(SET_BUILD), names);
        assert_eq!(UNCALLED[0].1, MEMBERS[1].2);
        // never a name the selector can ask for, nor one it spells itself
        for (n, _) in UNCALLED {
            assert!(ENTRIES.iter().all(|&(e, _)| e != n));
            assert!(!include_str!("isel.rs").contains(n), "isel.rs names {n}");
        }
    }

    #[test]
    fn entries_are_the_shipped_modules_globals() {
        let names: Vec<_> = ENTRIES.iter().map(|&(n, _)| n).collect();
        assert_eq!(source_globals(), names);
        assert!(SOURCE.contains(".section \".text.M$ARITH\""));
    }

    #[test]
    fn entries_are_in_image_order_within_the_module() {
        for w in ENTRIES.windows(2) {
            assert!(w[0].1 <= w[1].1, "{} {}", w[0].0, w[1].0);
        }
        assert_eq!(ENTRIES[0].1, 0xE0ABD4);
        assert!(ENTRIES.iter().all(|&(_, a)| a < 0xE0ABD4 + 0x1DC));
    }

    /// Every name the selector asks for is in the module.  isel's M$ARITH
    /// calls take a `'static` name from `marith_entry` (the selector's
    /// `marith_name`), asked for as multiply (`MI`) and divide (`DI`) by
    /// `LLW` / `LLL` (`marith_op`) and modulo (`OI`) by `WLW` / `LLL`,
    /// signed and unsigned; any other combination is refused there.
    #[test]
    fn selector_names_are_in_the_module() {
        for kind in ["MI", "DI"] {
            for form in ["LLW", "LLL"] {
                for signed in [false, true] {
                    assert!(marith_entry(kind, signed, form).is_some(), "{kind}{form}");
                }
            }
        }
        for form in ["WLW", "LLL"] {
            for signed in [false, true] {
                assert!(marith_entry("OI", signed, form).is_some(), "OI{form}");
            }
        }
        // and nothing outside the module is ever named
        for kind in KINDS {
            for form in FORMS {
                for signed in [false, true] {
                    if let Some(n) = marith_entry(kind, signed, form) {
                        assert!(source_globals().contains(&n));
                    }
                }
            }
        }
    }

    #[test]
    fn missing_combinations_have_no_name() {
        assert_eq!(marith_entry("MI", true, "WLW"), None);
        assert_eq!(marith_entry("DI", false, "WLW"), None);
        assert_eq!(marith_entry("OI", false, "WWL"), None);
        assert_eq!(marith_entry("OI", true, "WWL"), Some("M$OIS$WWL"));
    }
}
