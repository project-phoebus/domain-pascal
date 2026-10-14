//! The m68k run-time library: M$ARITH, the Domain Pascal compiler's
//! arithmetic routines (CLAUDE.md 3.11; codegen-patterns 8), shipped with
//! dpc as `runtime/m68k/m_arith.s`, built into
//! `target/runtime/m68k/libm_arith.a` (runtime/README.md).
//!
//! The instruction selector names an entry only through [`marith_entry`],
//! so it can name only routines the shipped module defines.

/// The map module: `I E0ABD4 M$ARITH size = 1DC` (SAU2 map).
pub const MODULE: &str = "M$ARITH";

/// The library file under `<runtime dir>/m68k/` ([`dpc_target::Target::runtime_library`]).
pub const LIBRARY: &str = "libm_arith.a";

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

    /// The `.global` names of the shipped module, in source order.
    fn source_globals() -> Vec<&'static str> {
        SOURCE
            .lines()
            .filter_map(|l| l.trim().strip_prefix(".global"))
            .map(str::trim)
            .collect()
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
