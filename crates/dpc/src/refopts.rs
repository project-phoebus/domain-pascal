//! The compiler options of the reference: ref 6.4 "Compiler Options",
//! Table 6-2 (pp. 6-6..6-8) and sections 6.4.1..6.4.31 (pp. 6-9..6-36).
//! `docs/compiler-options.md` lists every option with what dpc does with
//! it; this module is that table in code.
//!
//! Each option is spelt as in the reference: one dash, lower case, not
//! abbreviated ("Note that you cannot abbreviate these options", 6.3,
//! p. 6-4).  An option is one of:
//!
//! - implemented: `-config`, `-cond`, `-comchk`, `-idir`, `-natural`,
//!   `-warn`/`-nwarn`, `-msgs`/`-nmsgs`, `-b`/`-nb`, `-version` (in
//!   `main`);
//! - accepted, with nothing to do: the defaults dpc already follows
//!   (`-ac`, `-ncond`, `-xrs`, ...) and options without effect on dpc's
//!   output (`-compress`, `-nclines`, `-prasm`, ...);
//! - accepted with a note on stderr: dpc does not give what the reference
//!   describes, but the program means the same (`-map`, `-l`, `-dbs`,
//!   `-cpu mathchip`, ...);
//! - refused: the option would change the generated code in a way the
//!   reference does not define precisely, or that dpc does not implement
//!   (`-pic`, `-subchk`, `-opt 0`, `-iso`, ...).
//!
//! Options of one pair (`-x`/`-nx`) or group (`-db`/`-ndb`/`-dba`/`-dbs`)
//! may be given once between them: a second one is an error, as dpc's own
//! options are (the reference does not say which would win).  `-config`
//! and `-idir` may be repeated; their lists add up.
//!
//! Options with a list (`-config var1 ... varN`, `-idir dir1 ... dirN`)
//! take every following argument up to the next one that begins with `-`;
//! options with an optional operand (`-b [pathname]`, `-l [pathname]`)
//! take the next argument if it does not begin with `-`, and `-opt [n]`,
//! `-info [n]` if it is a number.  So the source file goes first, as in the
//! reference's `pas source_pathname [options]` (6.3, p. 6-4), or after
//! `--`.

use std::iter::Peekable;
use std::path::PathBuf;

use dpc_m68k::Cpu;

/// The object (here: assembly) file, `-b [pathname]` / `-nb` (ref 6.4.3,
/// pp. 6-9..6-10).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Binary {
    /// `-b` (the default): the source pathname with `.s`.
    #[default]
    Default,
    /// `-b pathname`: `pathname.s` (`pathname` itself if it ends in `.s`),
    /// as the reference names `pathname.bin` (p. 6-10).
    Path(PathBuf),
    /// `-nb`: compile, but write no output file.
    Suppress,
}

/// The reference options given, with dpc's defaults.
#[derive(Debug, Clone)]
pub struct RefOptions {
    /// `-config var1 ... varN` (ref 6.4.8, p. 6-12): `None` when not given.
    pub config: Option<Vec<String>>,
    /// `-cond` (ref 6.4.7, p. 6-12); default `-ncond`.
    pub cond: bool,
    /// `-comchk` (ref 6.4.5, p. 6-11); default `-ncomchk`.
    pub comchk: bool,
    /// `-natural` (ref 6.4.22, pp. 6-25..6-26); default `-nnatural`.
    pub natural: bool,
    /// `-warn` (default) / `-nwarn` (ref 6.4.30, pp. 6-35..6-36).
    pub warn: bool,
    /// `-msgs` / `-nmsgs` (ref 6.4.21, p. 6-25).  The default is
    /// `-nmsgs`, no report (owner ruling 2026-10-20; the text says
    /// `-msgs`, Table 6-2 `-nmsgs`; docs/reference-clarifications.md).
    pub msgs: bool,
    /// `-b [pathname]` / `-nb`.
    pub binary: Binary,
    /// `-cpu id` (ref 6.4.9, pp. 6-14..6-17): the processor whose code is
    /// generated.  Default MC68010 (not the reference's `mathlib_sr10`:
    /// see `dpc_m68k::Cpu`).
    pub cpu: Cpu,
    /// `-idir dir1 ... dirN` (ref 6.4.13, pp. 6-20..6-21), in order.
    pub include_dirs: Vec<PathBuf>,
    /// One line each for options accepted although dpc does not give what
    /// the reference describes.
    pub notes: Vec<String>,
    /// Options that took operands from the following arguments (for the
    /// "no input file" hint).
    pub took_operands: Vec<String>,
    /// The option groups given so far, with the spelling used.
    groups: Vec<(&'static str, String)>,
}

impl Default for RefOptions {
    fn default() -> Self {
        RefOptions {
            config: None,
            cond: false,
            comchk: false,
            natural: false,
            warn: true,
            msgs: false,
            binary: Binary::Default,
            cpu: Cpu::M68010,
            include_dirs: Vec::new(),
            notes: Vec::new(),
            took_operands: Vec::new(),
            groups: Vec::new(),
        }
    }
}

/// "You can put up to 63 pathnames following an -idir option" (ref
/// 6.4.13, p. 6-21).
const MAX_IDIRS: usize = 63;

/// The `-cpu` arguments (ref 6.4.9, Table 6-3, p. 6-16) that ask for
/// generic MC680x0 code, or for the MC68010-class workstations (DSP160,
/// DN460, DN660; the PEB machines DN100..DN600): dpc's MC68010 code, the
/// SAU2..SAU5 kernels' (CLAUDE.md 3.16).
const CPU_68010: [&str; 6] = ["any", "m68k", "160", "460", "660", "peb"];

/// The `-cpu` arguments of Table 6-3 that ask for code for an MC68020 or
/// later (the `mathchip` group, "MC68020 or MC68030 microprocessor and an
/// MC68881 or MC68882", p. 6-15; `mathlib_sr10` and `mathlib`, "MC68040,
/// or MC68020 or MC68030"; `fpa1` and `fpx`, the DN3000/DN4000/DN4500 and
/// DN5xx accelerators): dpc's MC68020 code, the SAU6..SAU9 kernels'
/// (`dpc_m68k::Cpu`, codegen-patterns 14).  Noted, since the floating-point
/// side of each argument has nothing to select in dpc (no reals).
const CPU_68020: [&str; 11] = [
    "mathlib_sr10", "mathlib", "mathchip", "3000", "580", "570", "560", "330", "90", "fpa1", "fpx",
];

/// The names of every reference option: Table 6-2 (pp. 6-6..6-8), plus
/// `-ninfo` (6.4.16, p. 6-21) and the obsolete `-nopt` (6.4.24, p. 6-26),
/// which only the sections name.
pub const NAMES: &[&str] = &[
    "-ac",
    "-pic",
    "-alnchk",
    "-b",
    "-nb",
    "-bounds_violation",
    "-no_bounds_violation",
    "-comchk",
    "-ncomchk",
    "-compress",
    "-ncompress",
    "-cond",
    "-ncond",
    "-config",
    "-cpu",
    "-db",
    "-ndb",
    "-dbs",
    "-dba",
    "-exp",
    "-nexp",
    "-frnd",
    "-nfrnd",
    "-idir",
    "-imap",
    "-nimap",
    "-indexl",
    "-nindexl",
    "-info",
    "-ninfo",
    "-inlib",
    "-iso",
    "-niso",
    "-l",
    "-nl",
    "-map",
    "-nmap",
    "-msgs",
    "-nmsgs",
    "-natural",
    "-nnatural",
    "-nclines",
    "-opt",
    "-nopt",
    "-prasm",
    "-nprasm",
    "-slib",
    "-std",
    "-nstd",
    "-subchk",
    "-nsubchk",
    "-version",
    "-warn",
    "-nwarn",
    "-xrs",
    "-nxrs",
];

impl RefOptions {
    /// Record that `opt` (of `group`) was given; a second option of the
    /// same group is an error.
    fn claim(&mut self, group: &'static str, opt: &str) -> Result<(), String> {
        if let Some((_, prev)) = self.groups.iter().find(|(g, _)| *g == group) {
            return Err(if prev == opt {
                format!("{opt} given more than once")
            } else {
                format!("{opt} conflicts with {prev}")
            });
        }
        self.groups.push((group, opt.to_string()));
        Ok(())
    }

    fn note(&mut self, opt: &str, text: &str, cite: &str) {
        self.notes.push(format!("{opt}: {text} (ref {cite})"));
    }
}

/// A refusal: the option is known but not supported.
fn refuse(opt: &str, why: &str, cite: &str) -> Result<(), String> {
    Err(format!("{opt} is not supported: {why} (ref {cite})"))
}

/// The next argument, if it does not begin with `-` (an optional operand).
fn optional<I: Iterator<Item = String>>(rest: &mut Peekable<I>) -> Option<String> {
    rest.next_if(|a| !a.starts_with('-'))
}

/// The next argument, if it is a number (`-opt [n]`, `-info [n]`).
fn optional_number<I: Iterator<Item = String>>(rest: &mut Peekable<I>) -> Option<String> {
    rest.next_if(|a| !a.is_empty() && a.bytes().all(|b| b.is_ascii_digit()))
}

/// Every following argument up to one that begins with `-`.
fn list<I: Iterator<Item = String>>(rest: &mut Peekable<I>) -> Vec<String> {
    let mut v = Vec::new();
    while let Some(a) = rest.next_if(|a| !a.starts_with('-')) {
        v.push(a);
    }
    v
}

/// A conditional variable's name as the preprocessor reads one (ref 4.7,
/// p. 4-50; `dpc_syntax::lexer::Lexer::set_cond_names`): a letter or `_`,
/// then letters, digits, `_` and `$`, with single `.`s inside (`code.sr9`).
fn is_cond_name(s: &str) -> bool {
    let ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'$';
    let b = s.as_bytes();
    match b.first() {
        Some(c) if c.is_ascii_alphabetic() || *c == b'_' => {}
        _ => return false,
    }
    b.windows(2).all(|w| w[0] != b'.' || ident(w[1]))
        && b.iter().all(|&c| ident(c) || c == b'.')
        && b.last() != Some(&b'.')
}

/// Handle `arg` if it is a reference option, taking its operands from
/// `rest`; `None` if it is not one (the caller tries dpc's own options).
pub fn parse<I: Iterator<Item = String>>(
    arg: &str,
    rest: &mut Peekable<I>,
    o: &mut RefOptions,
) -> Option<Result<(), String>> {
    if !NAMES.contains(&arg) {
        return None;
    }
    Some(parse_known(arg, rest, o))
}

fn parse_known<I: Iterator<Item = String>>(
    arg: &str,
    rest: &mut Peekable<I>,
    o: &mut RefOptions,
) -> Result<(), String> {
    match arg {
        // ---- 6.4.1 -ac and -pic, p. 6-9 ----
        "-ac" => o.claim("ac", arg),
        "-pic" => refuse(
            arg,
            "dpc generates absolute code (-ac) only",
            "6.4.1, p. 6-9",
        ),
        // ---- 6.4.2 -alnchk, p. 6-9 ----
        "-alnchk" => {
            o.claim("alnchk", arg)?;
            o.note(
                arg,
                "dpc issues no messages about alignment",
                "6.4.2, p. 6-9",
            );
            Ok(())
        }
        // ---- 6.4.3 -b and -nb, pp. 6-9..6-10 ----
        "-b" => {
            o.claim("b", arg)?;
            if let Some(p) = optional(rest) {
                o.took_operands.push(arg.to_string());
                let p = if p.ends_with(".s") {
                    p
                } else {
                    format!("{p}.s")
                };
                o.binary = Binary::Path(PathBuf::from(p));
            }
            Ok(())
        }
        "-nb" => {
            o.claim("b", arg)?;
            o.binary = Binary::Suppress;
            Ok(())
        }
        // ---- 6.4.4 -bounds_violation, p. 6-10 ----
        "-bounds_violation" => o.claim("bounds", arg),
        "-no_bounds_violation" => refuse(
            arg,
            "the reference does not say what code it changes; dpc generates the \
                 default (-bounds_violation) code",
            "6.4.4, p. 6-10",
        ),
        // ---- 6.4.5 -comchk and -ncomchk, p. 6-11 ----
        "-comchk" | "-ncomchk" => {
            o.claim("comchk", arg)?;
            o.comchk = arg == "-comchk";
            Ok(())
        }
        // ---- 6.4.6 -compress and -ncompress, p. 6-12: object file
        // storage; dpc's output is assembly, its object file the
        // assembler's ----
        "-compress" | "-ncompress" => o.claim("compress", arg),
        // ---- 6.4.7 -cond and -ncond, p. 6-12 ----
        "-cond" | "-ncond" => {
            o.claim("cond", arg)?;
            o.cond = arg == "-cond";
            Ok(())
        }
        // ---- 6.4.8 -config, pp. 6-12..6-14 ----
        "-config" => {
            // "-config var1 [... varN]" (p. 6-12): at least one name.
            let names = list(rest);
            if names.is_empty() {
                return Err("-config needs at least one conditional variable name \
                                (ref 6.4.8, p. 6-12)"
                    .into());
            }
            if let Some(bad) = names.iter().find(|n| !is_cond_name(n)) {
                return Err(format!(
                    "-config: `{bad}` is not a conditional variable name (ref 4.7, p. 4-50)"
                ));
            }
            o.took_operands.push(arg.to_string());
            o.config.get_or_insert_with(Vec::new).extend(names);
            Ok(())
        }
        // ---- 6.4.9 -cpu, pp. 6-14..6-17, Table 6-3 p. 6-16 ----
        "-cpu" => {
            o.claim("cpu", arg)?;
            let id = rest
                .next_if(|a| !a.starts_with('-'))
                .ok_or("-cpu needs an argument (ref 6.4.9, Table 6-3, p. 6-16)")?;
            o.took_operands.push(arg.to_string());
            let lower = id.to_ascii_lowercase();
            if CPU_68010.contains(&lower.as_str()) {
                o.cpu = Cpu::M68010;
                Ok(())
            } else if CPU_68020.contains(&lower.as_str()) {
                o.cpu = Cpu::M68020;
                o.note(
                    &format!("-cpu {id}"),
                    "dpc generates MC68020 integer code for this argument; it has no \
                         floating-point code to select",
                    "6.4.9, Table 6-3, p. 6-16",
                );
                Ok(())
            } else if lower == "a88k" {
                refuse(
                    "-cpu a88k",
                    "dpc generates MC680x0 code only",
                    "6.4.9, Table 6-3, p. 6-16",
                )
            } else {
                Err(format!(
                    "-cpu: unknown argument `{id}` (ref 6.4.9, Table 6-3, p. 6-16)"
                ))
            }
        }
        // ---- 6.4.10 -db, -ndb, -dba, -dbs, p. 6-18 ----
        "-ndb" => o.claim("db", arg),
        "-db" | "-dbs" => {
            o.claim("db", arg)?;
            o.note(
                arg,
                "dpc puts no debugging information in its output",
                "6.4.10, p. 6-18",
            );
            Ok(())
        }
        "-dba" => refuse(
            arg,
            "it sets -opt 0 and keeps variables in memory after every statement, \
                 which dpc does not generate",
            "6.4.10, p. 6-18; 6.4.24, p. 6-27",
        ),
        // ---- 6.4.11 -exp and -nexp, p. 6-18 ----
        "-nexp" => o.claim("exp", arg),
        "-exp" => {
            o.claim("exp", arg)?;
            o.note(arg, "dpc writes no listing file", "6.4.11, p. 6-18");
            Ok(())
        }
        // ---- 6.4.12 -frnd and -nfrnd, pp. 6-19..6-20 ----
        "-nfrnd" => o.claim("frnd", arg),
        "-frnd" => refuse(arg, "dpc has no floating point", "6.4.12, p. 6-19"),
        // ---- 6.4.13 -idir, pp. 6-20..6-21 ----
        "-idir" => {
            let dirs = list(rest);
            if dirs.is_empty() {
                return Err("-idir needs at least one directory (ref 6.4.13, p. 6-20)".into());
            }
            if dirs.len() > MAX_IDIRS {
                return Err(format!(
                    "-idir: at most {MAX_IDIRS} pathnames per -idir option, got {} \
                         (ref 6.4.13, p. 6-21)",
                    dirs.len()
                ));
            }
            o.took_operands.push(arg.to_string());
            o.include_dirs.extend(dirs.into_iter().map(PathBuf::from));
            Ok(())
        }
        // ---- 6.4.14 -imap and -nimap, p. 6-21 ----
        "-nimap" => o.claim("imap", arg),
        "-imap" => {
            o.claim("imap", arg)?;
            o.note(arg, "dpc writes no symbol table map", "6.4.14, p. 6-21");
            Ok(())
        }
        // ---- 6.4.15 -indexl and -nindexl, p. 6-21 ----
        "-nindexl" => o.claim("indexl", arg),
        "-indexl" => refuse(
            arg,
            "the reference does not say which optimizations it disables",
            "6.4.15, p. 6-21",
        ),
        // ---- 6.4.16 -info n and -ninfo, pp. 6-21..6-22 ----
        "-info" | "-ninfo" => {
            o.claim("info", arg)?;
            let level = if arg == "-ninfo" {
                0
            } else {
                match optional_number(rest) {
                    // "If n is omitted ... level 2" (p. 6-21).
                    None => 2,
                    Some(n) => {
                        o.took_operands.push(arg.to_string());
                        match n.parse::<u8>() {
                            Ok(l) if l <= 4 => l,
                            _ => {
                                return Err(format!(
                                    "-info: the level must be between 0 and 4, got `{n}` \
                                         (ref 6.4.16, p. 6-21)"
                                ))
                            }
                        }
                    }
                }
            };
            if level > 0 {
                o.note(
                    &format!("-info {level}"),
                    "dpc issues no information messages",
                    "6.4.16, p. 6-22",
                );
            }
            Ok(())
        }
        // ---- 6.4.17 -inlib, pp. 6-22..6-23 ----
        "-inlib" => refuse(
            arg,
            "dpc's output carries no library pathname for the loader",
            "6.4.17, p. 6-23",
        ),
        // ---- 6.4.18 -iso and -niso, p. 6-23 ----
        "-niso" => o.claim("iso", arg),
        "-iso" => refuse(
            arg,
            "the ISO rules for mod, comments and goto are not implemented",
            "6.4.18, p. 6-23",
        ),
        // ---- 6.4.19 -l and -nl, p. 6-24 ----
        "-nl" => o.claim("l", arg),
        "-l" => {
            o.claim("l", arg)?;
            if optional(rest).is_some() {
                o.took_operands.push(arg.to_string());
            }
            o.note(arg, "dpc writes no listing file", "6.4.19, p. 6-24");
            Ok(())
        }
        // ---- 6.4.20 -map and -nmap, pp. 6-24..6-25 ----
        "-nmap" => o.claim("map", arg),
        "-map" => {
            o.claim("map", arg)?;
            o.note(arg, "dpc writes no symbol table map", "6.4.20, p. 6-24");
            Ok(())
        }
        // ---- 6.4.21 -msgs and -nmsgs, p. 6-25 ----
        "-msgs" | "-nmsgs" => {
            o.claim("msgs", arg)?;
            o.msgs = arg == "-msgs";
            Ok(())
        }
        // ---- 6.4.22 -natural and -nnatural, pp. 6-25..6-26 ----
        "-natural" | "-nnatural" => {
            o.claim("natural", arg)?;
            o.natural = arg == "-natural";
            Ok(())
        }
        // ---- 6.4.23 -nclines, p. 6-26: COFF line number tables are
        // made only with -dba or -dbs, and dpc makes none ----
        "-nclines" => o.claim("nclines", arg),
        // ---- 6.4.24 -opt, pp. 6-26..6-32 ----
        "-opt" => {
            o.claim("opt", arg)?;
            // "If you specify -opt and omit the optimization level, the
            // level defaults to -opt 3" (p. 6-26).
            let n = optional_number(rest);
            if n.is_some() {
                o.took_operands.push(arg.to_string());
            }
            match n.as_deref().map(str::parse::<u8>) {
                None | Some(Ok(3)) => Ok(()),
                Some(Ok(l)) if l <= 4 => refuse(
                    &format!("-opt {l}"),
                    "dpc generates the code of one level, the kernel's, which is \
                         taken to be the default -opt 3 (docs/opt-levels.md)",
                    "6.4.24, p. 6-26",
                ),
                _ => Err(format!(
                    "-opt: the level must be between 0 and 4, got `{}` (ref 6.4.24, p. 6-26)",
                    n.unwrap_or_default()
                )),
            }
        }
        "-nopt" => refuse(
            arg,
            "it is -opt 0; dpc generates the code of one level, the kernel's, which \
                 is taken to be the default -opt 3 (docs/opt-levels.md)",
            "6.4.24, p. 6-26",
        ),
        // ---- 6.4.25 -prasm and -nprasm, p. 6-33: "If you use these
        // options with the compiler variants that generate MC680x0
        // code, they have no effect." ----
        "-prasm" | "-nprasm" => o.claim("prasm", arg),
        // ---- 6.4.26 -slib, pp. 6-33..6-34 ----
        "-slib" => refuse(
            arg,
            "precompiled libraries are not supported (nor is %slibrary)",
            "6.4.26, p. 6-33",
        ),
        // ---- 6.4.27 -std and -nstd, p. 6-35 ----
        "-nstd" => o.claim("std", arg),
        "-std" => {
            o.claim("std", arg)?;
            o.note(
                arg,
                "dpc does not report nonstandard language elements",
                "6.4.27, p. 6-35",
            );
            Ok(())
        }
        // ---- 6.4.28 -subchk and -nsubchk, p. 6-35 ----
        "-nsubchk" => o.claim("subchk", arg),
        "-subchk" => refuse(
            arg,
            "the reference does not say what the checking code is",
            "6.4.28, p. 6-35",
        ),
        // ---- 6.4.29 -version, p. 6-35 (alone: handled by main) ----
        "-version" => Err("-version is used alone: `dpc -version` (ref 6.4.29, p. 6-35)".into()),
        // ---- 6.4.30 -warn and -nwarn, pp. 6-35..6-36 ----
        "-warn" | "-nwarn" => {
            o.claim("warn", arg)?;
            o.warn = arg == "-warn";
            Ok(())
        }
        // ---- 6.4.31 -xrs and -nxrs, p. 6-36 ----
        "-xrs" => o.claim("xrs", arg),
        "-nxrs" => refuse(
            arg,
            "dpc's calls assume that external routines save registers (-xrs)",
            "6.4.31, p. 6-36",
        ),
        _ => unreachable!("{arg} is in NAMES"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_all(v: &[&str]) -> Result<(RefOptions, Vec<String>), String> {
        let mut o = RefOptions::default();
        let mut it = v.iter().map(|s| s.to_string()).peekable();
        let mut other = Vec::new();
        while let Some(a) = it.next() {
            match parse(&a, &mut it, &mut o) {
                Some(r) => r?,
                None => other.push(a),
            }
        }
        Ok((o, other))
    }

    #[test]
    fn every_reference_option_is_known() {
        // Every Table 6-2 name is handled here (accepted, refused, or a
        // usage error for a missing operand), never passed on.
        for n in NAMES {
            let mut o = RefOptions::default();
            let mut it = std::iter::empty::<String>().peekable();
            assert!(parse(n, &mut it, &mut o).is_some(), "{n}");
        }
        assert!(
            parse_all(&["-o", "-I", "--strict", "-Idir", "m.pas"])
                .unwrap()
                .1
                .len()
                == 5
        );
    }

    #[test]
    fn defaults() {
        let (o, _) = parse_all(&[]).unwrap();
        assert_eq!(o.config, None);
        assert!(!o.cond && !o.comchk && !o.natural && o.warn && !o.msgs);
        assert_eq!(o.binary, Binary::Default);
        assert!(o.notes.is_empty());
    }

    #[test]
    fn config_lists() {
        let (o, rest) = parse_all(&["m.pas", "-config", "first", "third", "-cond"]).unwrap();
        assert_eq!(o.config, Some(vec!["first".into(), "third".into()]));
        assert!(o.cond);
        assert_eq!(rest, ["m.pas"]);
        // repeated: the lists add up
        let (o, _) = parse_all(&["-config", "a", "-config", "code.sr9", "B$x"]).unwrap();
        assert_eq!(
            o.config,
            Some(vec!["a".into(), "code.sr9".into(), "B$x".into()])
        );
        // the list ends at the next option
        let (o, rest) = parse_all(&["-config", "a", "--", "m.pas"]).unwrap();
        assert_eq!(o.config, Some(vec!["a".into()]));
        assert_eq!(rest, ["--", "m.pas"]);
        assert!(parse_all(&["-config"]).is_err());
        assert!(parse_all(&["-config", "-cond"]).is_err());
        for bad in ["1a", "a.", ".a", "a..b", "a-b", ""] {
            assert!(parse_all(&["-config", bad]).is_err(), "{bad}");
        }
        assert!(parse_all(&["-config", "_isp__m68k"]).is_ok());
    }

    #[test]
    fn idir_lists() {
        let (o, _) = parse_all(&["-idir", "a", "b", "-idir", "c"]).unwrap();
        assert_eq!(o.include_dirs, ["a", "b", "c"].map(PathBuf::from));
        assert!(parse_all(&["-idir"]).is_err());
        let many: Vec<String> = (0..64).map(|i| format!("d{i}")).collect();
        let mut v = vec!["-idir"];
        v.extend(many.iter().map(String::as_str));
        assert!(parse_all(&v).is_err());
        assert!(parse_all(&v[..64]).is_ok());
    }

    #[test]
    fn pairs_are_given_once() {
        assert!(parse_all(&["-warn", "-nwarn"]).is_err());
        assert!(parse_all(&["-nwarn", "-nwarn"]).is_err());
        assert!(parse_all(&["-db", "-dbs"]).is_err());
        assert!(parse_all(&["-b", "-nb"]).is_err());
        assert!(parse_all(&["-info", "-ninfo"]).is_err());
        let (o, _) = parse_all(&["-nwarn", "-msgs", "-natural", "-comchk"]).unwrap();
        assert!(!o.warn && o.msgs && o.natural && o.comchk);
    }

    #[test]
    fn binary_option() {
        let (o, rest) = parse_all(&["m.pas", "-b", "jest"]).unwrap();
        assert_eq!(o.binary, Binary::Path("jest.s".into()));
        assert_eq!(rest, ["m.pas"]);
        let (o, _) = parse_all(&["-b", "jest.s"]).unwrap();
        assert_eq!(o.binary, Binary::Path("jest.s".into()));
        let (o, _) = parse_all(&["-b", "-cond"]).unwrap();
        assert_eq!(o.binary, Binary::Default);
        let (o, _) = parse_all(&["-nb"]).unwrap();
        assert_eq!(o.binary, Binary::Suppress);
    }

    #[test]
    fn levels() {
        assert!(parse_all(&["-opt"]).is_ok());
        let (_, rest) = parse_all(&["-opt", "3", "m.pas"]).unwrap();
        assert_eq!(rest, ["m.pas"]);
        let (_, rest) = parse_all(&["-opt", "m.pas"]).unwrap();
        assert_eq!(rest, ["m.pas"]);
        for l in ["0", "1", "2", "4"] {
            let e = parse_all(&["-opt", l]).unwrap_err();
            assert!(e.contains("not supported"), "{e}");
        }
        assert!(parse_all(&["-opt", "5"])
            .unwrap_err()
            .contains("between 0 and 4"));
        assert!(parse_all(&["-nopt"]).is_err());
        let (o, _) = parse_all(&["-info"]).unwrap();
        assert_eq!(o.notes.len(), 1);
        assert!(o.notes[0].starts_with("-info 2: "), "{:?}", o.notes);
        let (o, _) = parse_all(&["-info", "0"]).unwrap();
        assert!(o.notes.is_empty());
        let (o, _) = parse_all(&["-ninfo"]).unwrap();
        assert!(o.notes.is_empty());
        assert!(parse_all(&["-info", "7"]).is_err());
    }

    #[test]
    fn cpu_arguments() {
        assert_eq!(parse_all(&[]).unwrap().0.cpu, Cpu::M68010);
        for id in CPU_68010 {
            let (o, _) = parse_all(&["-cpu", id]).unwrap();
            assert!(o.notes.is_empty(), "{id}");
            assert_eq!(o.cpu, Cpu::M68010, "{id}");
        }
        for id in CPU_68020 {
            let (o, _) = parse_all(&["-cpu", id]).unwrap();
            assert_eq!(o.notes.len(), 1, "{id}");
            assert_eq!(o.cpu, Cpu::M68020, "{id}");
        }
        assert_eq!(parse_all(&["-cpu", "MathChip"]).unwrap().0.cpu, Cpu::M68020);
        assert!(parse_all(&["-cpu", "a88k"])
            .unwrap_err()
            .contains("not supported"));
        assert!(parse_all(&["-cpu", "z80"])
            .unwrap_err()
            .contains("unknown argument"));
        assert!(parse_all(&["-cpu"]).is_err());
        assert!(parse_all(&["-cpu", "any", "-cpu", "m68k"]).is_err());
    }

    #[test]
    fn accepted_refused_and_noted() {
        for silent in [
            "-ac",
            "-bounds_violation",
            "-ncomchk",
            "-compress",
            "-ncompress",
            "-ncond",
            "-ndb",
            "-nexp",
            "-nfrnd",
            "-nimap",
            "-nindexl",
            "-niso",
            "-nl",
            "-nmap",
            "-nmsgs",
            "-nnatural",
            "-nclines",
            "-prasm",
            "-nprasm",
            "-nstd",
            "-nsubchk",
            "-warn",
            "-xrs",
        ] {
            let (o, _) = parse_all(&[silent]).unwrap();
            assert!(o.notes.is_empty(), "{silent}");
        }
        for noted in [
            "-alnchk", "-db", "-dbs", "-exp", "-imap", "-l", "-map", "-std",
        ] {
            let (o, _) = parse_all(&[noted]).unwrap();
            assert_eq!(o.notes.len(), 1, "{noted}");
            assert!(o.notes[0].contains("(ref 6.4."), "{:?}", o.notes);
        }
        for refused in [
            "-pic",
            "-no_bounds_violation",
            "-dba",
            "-frnd",
            "-indexl",
            "-inlib",
            "-iso",
            "-nopt",
            "-slib",
            "-subchk",
            "-nxrs",
        ] {
            let e = parse_all(&[refused]).unwrap_err();
            assert!(
                e.starts_with(&format!("{refused} is not supported: ")),
                "{e}"
            );
            assert!(e.contains("(ref 6.4."), "{e}");
        }
        // -l takes an optional pathname.
        let (_, rest) = parse_all(&["-l", "listing", "-cond", "m.pas"]).unwrap();
        assert_eq!(rest, ["m.pas"]);
        assert!(parse_all(&["-version"]).is_err());
    }
}
