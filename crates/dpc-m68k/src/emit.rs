//! Assembly emitter: gas syntax for `m68k-elf-as -m68010` (CLAUDE.md 4.4).
//!
//! One output section per Pascal code section, in order of first use
//! ([`code_section_name`]: `.text`, or `.text.<UNIT>.<SECTION>`), and one
//! NOBITS data area per Pascal data section ([`data_section_name`]).
//! Global routines get `.globl` with their upper-cased, `$`-keeping symbol
//! (codegen-patterns 11); `internal` routines are plain (local) labels.
//!
//! **Fragments.**  The compiler aligns loop heads, and pads the end of
//! each module's code, to 4 bytes with the no-op `movea.l A0,A0` (0x2048;
//! codegen-patterns 7).  The alignment is relative to the module's code
//! base, which the linker places 4-aligned, so in effect it is absolute.
//! gas aligns relative to the section start, which is the same thing for
//! a whole module.  An image-conformance unit, though, is often a few
//! routines out of a module, starting at an address that may be 2 mod 4
//! (PROC1_$TST_LOCK 0xE148CA).  [`Options::fragment_origin`] describes
//! such a unit; then:
//!
//! - the code section starts with `origin mod 4` bytes of `.space`, so
//!   that section-relative `.balignw 4` equals absolute alignment once the
//!   section is linked at `origin & !3`.  These bytes are not part of the
//!   unit; the absolute local symbol [`FRAGMENT_ORIGIN_SYMBOL`] records
//!   `origin` so that `tools/conform.py` can link and compare accordingly;
//! - no module-end padding is emitted, unless
//!   [`Options::fragment_ends_module`] says that the unit is the tail of
//!   its module (then the padding, absolute like the rest, is part of the
//!   unit's last routine in the image).
//!
//! **Loop alignment on and off.**  In the image, alignment is on for a
//! leading run of the routines of a map segment and off for the rest;
//! the switch goes one way only and its cause is not recorded
//! (codegen-patterns 7: AST_ after the module-local routine at 0xE01502,
//! VTOC_ from VTOC_$DISMOUNT 0xE38764, RING from RING_$SVC_CLOSE
//! 0xE76E22; PMAP_ and PROC1_ never align).  [`Options::loop_align`]
//! reproduces that from outside the source: a routine that does not align
//! has no filler before its loop heads, and the end of the module (or of
//! a module-tail fragment) is padded with 0x2048 only if its last routine
//! aligns, else with zero words (`rts; 0x0000`: AST_ 0xE0736E, RING,
//! ACL_, VTOC_ .TEXT; whether the compiler or the binder wrote the
//! `0x0000` is Uncertain).

use dpc_ir::Module;
use dpc_syntax::Diagnostic;

use crate::insn::Insn;
use crate::isel;

/// The local label at the start of the unit's static data area: what
/// `lea <label>.l,A5` loads (codegen-patterns 4).  A `.L` label is not
/// kept in the object's symbol table: the relocation names the data
/// section, at the area's offset in it.
pub const DATA_BASE_LABEL: &str = ".Ldata";

/// Name of the absolute local symbol that records the image address of
/// a fragment's first byte (`tools/conform.py` reads it).
pub const FRAGMENT_ORIGIN_SYMBOL: &str = "__dpc_fragment_origin";

/// Which routines align their loop heads (codegen-patterns 7; `dpc
/// --loop-align`, `--loop-align-off-from`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum LoopAlign {
    /// Every routine aligns (the default).
    #[default]
    On,
    /// No routine aligns.
    Off,
    /// In the code section of each named routine, the routines before it
    /// align and it and every later one do not; the routines of a code
    /// section no name is in align.  At most one name per code section
    /// (each map segment has its own switch, codegen-patterns 7).  Names
    /// are matched case-insensitively against each routine's symbol,
    /// which is its Pascal name upper-cased (codegen-patterns 11), so
    /// either spelling works.
    OffFrom(Vec<String>),
}

/// Emitter options.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Options {
    /// The output is a fragment of a module whose first byte belongs at
    /// this absolute address (test-oriented: `dpc --fragment-origin`).
    /// `None`: a whole module.
    pub fragment_origin: Option<u32>,
    /// With `fragment_origin`: the fragment ends its module, so it gets
    /// the module-end padding (`dpc --fragment-ends-module`).  Ignored
    /// for a whole module, which always gets it.
    pub fragment_ends_module: bool,
    /// Which routines align their loop heads.
    pub loop_align: LoopAlign,
    /// With `fragment_origin`: routines of the same module that lie
    /// outside the fragment, by symbol and image address (`dpc
    /// --module-routine`).  The original compiled whole modules and
    /// called such a routine with `bsr` (codegen-patterns 1.6:
    /// DISK_$UNASSIGN_ALL 0xE6BE34 `bsr.w` to DISK_$UNASSIGN 0xE6BDA8,
    /// the routine before it); the fragment declares it `extern`, and its
    /// `jsr` becomes a `bsr` sized from the address, as for a routine of
    /// the fragment.  The reference resolves to the map name when the
    /// fragment is linked.  An unnamed module-local routine is declared
    /// `internal; extern` and named by its local symbol, which the
    /// conformance harness defines (`--defsym`; bead domain-pascal-3h9).
    pub module_routines: Vec<(String, u32)>,
    /// The scheduling policy for the bodies (`dpc --schedule`;
    /// `sched.rs`, codegen-patterns 12.4 R9-R12).
    pub schedule: crate::sched::Schedule,
}

/// For each routine of `module` (indexed as `module.functions`): does it
/// align its loop heads?  [`LoopAlign::OffFrom`] switches each named
/// routine's code section off from it on, in the unit's order within the
/// section (its emission order): each map segment has its own switch
/// (codegen-patterns 7), and a code section of a unit is one segment.
/// Fails if a name is no routine of the module, or if two names are in
/// one code section.
fn aligning(module: &Module, la: &LoopAlign) -> Result<Vec<bool>, Diagnostic> {
    let fs = &module.functions;
    match la {
        LoopAlign::On => Ok(vec![true; fs.len()]),
        LoopAlign::Off => Ok(vec![false; fs.len()]),
        LoopAlign::OffFrom(names) => {
            let mut aligns = vec![true; fs.len()];
            let mut switched: Vec<&str> = Vec::new();
            for name in names {
                let Some(k) = fs.iter().position(|f| f.symbol.eq_ignore_ascii_case(name)) else {
                    return Err(Diagnostic::error_nopos(format!(
                        "--loop-align-off-from: no routine `{name}` in this unit"
                    )));
                };
                let sec = fs[k].code_section.as_str();
                if switched.contains(&sec) {
                    return Err(Diagnostic::error_nopos(format!(
                        "--loop-align-off-from: two routines of code section `{sec}`; each \
                         code section switches at most once"
                    )));
                }
                switched.push(sec);
                for (f, a) in fs.iter().zip(aligns.iter_mut()).skip(k) {
                    if f.code_section == sec {
                        *a = false;
                    }
                }
            }
            Ok(aligns)
        }
    }
}

/// The Pascal default sections (ref 5.7.1, p. 5-24), as `dpc_sema` names
/// them when nothing names another.
const DEFAULT_CODE_SECTION: &str = ".text";
const DEFAULT_DATA_SECTION: &str = ".data";

/// The output section of unit `unit`'s code in Pascal code section
/// `sect`: `.text` for the default section, else
/// `.text.<UNIT>.<SECT>`, both names upper-cased like every external name
/// (codegen-patterns 11; the map's sections and segments are upper case:
/// `I E1471C PROC1_` in WIRED_PROC).
///
/// The `.text.` prefix is what `../source`'s link takes for code
/// (`tools/gen_layout_ld.py`, `section_kind`), which orders a code input
/// section by the earliest map symbol it defines, whatever its name, and
/// fails a section whose symbols straddle another's: one input section
/// per unit and code section is exactly the image's one segment per
/// module and section.  The unit name keeps two units' sections of the
/// same Pascal section apart, and the `.` between the two names keeps
/// them apart from every `.text.<symbol>` of `-ffunction-sections` and
/// of the hand-written assembly there.
pub fn code_section_name(unit: &str, sect: &str) -> String {
    if sect == DEFAULT_CODE_SECTION {
        DEFAULT_CODE_SECTION.into()
    } else {
        format!(".text.{}.{}", unit.to_uppercase(), sect.to_uppercase())
    }
}

/// The section directive for unit `unit`'s Pascal code section `sect`.
fn section_directive(unit: &str, sect: &str) -> String {
    if sect == DEFAULT_CODE_SECTION {
        "\t.text".into()
    } else {
        format!(
            "\t.section\t\"{}\",\"ax\",@progbits",
            code_section_name(unit, sect)
        )
    }
}

/// Render `module` as assembly text.  `source_name` is recorded in the
/// header comment.  Fails if a routine needs a pattern that is not
/// implemented yet, or if a fragment has more than one code section.
pub fn emit(module: &Module, source_name: &str, opts: &Options) -> Result<String, Vec<Diagnostic>> {
    let mut out = format!("| generated by dpc from {source_name}\n");
    let mut errors = Vec::new();
    let mut sections: Vec<&str> = Vec::new();
    for f in &module.functions {
        if !sections.contains(&f.code_section.as_str()) {
            sections.push(&f.code_section);
        }
    }
    let aligns = aligning(module, &opts.loop_align).map_err(|d| vec![d])?;
    if let Some((name, _)) = opts.module_routines.iter().find(|(n, _)| {
        module
            .functions
            .iter()
            .any(|f| f.symbol.eq_ignore_ascii_case(n))
    }) {
        return Err(vec![Diagnostic::error_nopos(format!(
            "--module-routine: `{name}` has its body in this unit"
        ))]);
    }
    // ref 5.2, p. 5-4: "Internal routines can be called only within the
    // same compilation unit in which they are defined".  An `internal;
    // extern` routine is accepted only as `--module-routine` scaffolding
    // (a module-local routine outside a fragment; bead domain-pascal-3h9);
    // otherwise its `jsr` would name a local symbol nothing defines.
    let outside: Vec<Diagnostic> = module
        .outside_internal_calls
        .iter()
        .filter(|(sym, _)| {
            !opts
                .module_routines
                .iter()
                .any(|(n, _)| n.eq_ignore_ascii_case(sym))
        })
        .map(|(sym, span)| {
            Diagnostic::error(
                *span,
                format!(
                    "call of internal routine `{sym}`, which has no body in this unit \
                     (internal routines can be called only within the compilation unit \
                     that defines them, ref 5.2 p. 5-4; name it with --module-routine)"
                ),
            )
        })
        .collect();
    if !outside.is_empty() {
        return Err(outside);
    }
    if let Some(origin) = opts.fragment_origin {
        // Where a second code section of a fragment would start in its
        // module is not known, so its alignment could not be placed.
        if sections.len() > 1 {
            return Err(vec![Diagnostic::error_nopos(format!(
                "--fragment-origin: the module has {} code sections; a fragment may have only one",
                sections.len()
            ))]);
        }
        out.push_str(&format!(
            "| fragment of a module, first byte at 0x{origin:X} (dpc --fragment-origin)\n\
             \t.set\t{FRAGMENT_ORIGIN_SYMBOL},0x{origin:x}\n"
        ));
    }
    // Cell labels are numbered across the code sections: each section has
    // its own cells, but all share the one assembly file.
    let mut pool_labels = 0;
    for sec in sections {
        out.push('\n');
        out.push_str(&section_directive(&module.name, sec));
        out.push('\n');
        if let Some(origin) = opts.fragment_origin {
            if origin % 4 != 0 {
                out.push_str(&format!(
                    "\t.space\t{}\t| not part of the unit: makes 4-byte alignment absolute\n",
                    origin % 4
                ));
            }
        }
        let mut routines = Vec::new();
        let mut failed = false;
        let mut last_aligns = true;
        // The section's constant pool: cells placed so far and the next
        // label number (`Pool::place`).
        let mut pool = Pool {
            next: pool_labels,
            ..Default::default()
        };
        for (f, &al) in module
            .functions
            .iter()
            .zip(&aligns)
            .filter(|(f, _)| f.code_section == sec)
        {
            last_aligns = al;
            match isel::select_with(f, opts.schedule) {
                Ok(mut i) => {
                    for insn in i.iter_mut() {
                        if let Insn::Jsr(crate::insn::Ea::AbsL(sym, 0)) = insn {
                            // The IR's symbol (upper-cased), not the
                            // option's spelling.
                            if opts
                                .module_routines
                                .iter()
                                .any(|(n, _)| n.eq_ignore_ascii_case(sym))
                            {
                                *insn = Insn::Bsr {
                                    target: sym.clone(),
                                    short: true,
                                };
                            }
                        }
                    }
                    // Every `Align4` the selector emits is a loop head's
                    // (fill loops, block-copy loops, `repeat` heads); a
                    // routine that does not align has none (codegen-
                    // patterns 7: VTOC_$DISMOUNT's copy loop at 0xE387CE,
                    // 2 mod 4, no filler).  Removed before the branches
                    // are sized, so their sizes see the final layout.
                    if !al {
                        i.retain(|i| !matches!(i, Insn::Align4));
                    }
                    pool.place(&mut i, al);
                    routines.push((f, i))
                }
                Err(d) => {
                    errors.push(d);
                    failed = true;
                }
            }
        }
        // Branch sizes need every routine of the section laid out.
        let base = opts.fragment_origin.map_or(0, |o| o % 4);
        // The outside routines in the layout's coordinates (the section
        // starts at `origin & !3`).
        let outside: Vec<(String, i64)> = opts
            .module_routines
            .iter()
            .map(|(n, a)| {
                let o = opts.fragment_origin.unwrap_or(0) & !3;
                (n.clone(), *a as i64 - o as i64)
            })
            .collect();
        pool_labels = pool.next;
        if !failed {
            if let Err(d) = size_branches(&mut routines, base, &outside) {
                errors.push(d);
            } else {
                encode_outside_short_calls(&mut routines, base, &outside);
            }
        }
        for (f, insns) in routines {
            if f.global {
                out.push_str(&format!("\t.globl\t{}\n", f.symbol));
            }
            out.push_str(&format!("{}:\n", f.symbol));
            for i in insns {
                if !matches!(i, Insn::Label(_) | Insn::PoolLabel(_)) {
                    out.push('\t');
                }
                out.push_str(&i.to_string());
                out.push('\n');
            }
        }
        // Each module's code contribution is padded to a multiple of 4
        // with the no-op `movea.l A0,A0` (0x2048): 0xE1A9D2 before OS_TERM
        // 0xE1A9D4, 0xE1AD16 before BITPAD 0xE1AD18 (codegen-patterns 7).
        // A fragment that does not end its module gets none.  When the
        // last routine does not align, the padding is zero words instead
        // (`rts; 0x0000`: AST_ 0xE0736E; see the module comment).
        if opts.fragment_origin.is_none() || opts.fragment_ends_module {
            if last_aligns {
                out.push_str(&format!("\t{}\t| movea.l %a0,%a0 filler\n", Insn::Align4));
            } else {
                out.push_str(
                    "\t.balignw\t4,0x0000\t| zero words: the last routine does not align\n",
                );
            }
        }
    }
    for data in &module.data {
        out.push_str(&data_area(&module.name, data));
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

/// Write each `bsr.s` to a routine outside the fragment
/// ([`Options::module_routines`]) as its encoding, `.short 0x61dd`: gas
/// cannot relocate an 8-bit displacement to a symbol it does not define
/// ("value too large for field of 1 byte").  The original's `bsr.s` to
/// the next routine (MST_$REMAP 0xE439FC `610e` to MST_$REMAP_PRIVI
/// 0xE43A0C) then links as the original.  `base` and `outside` as for
/// [`size_branches`], whose sizes this keeps.
fn encode_outside_short_calls(
    routines: &mut [(&dpc_ir::Function, Vec<Insn>)],
    base: u32,
    outside: &[(String, i64)],
) {
    let mut pos = base;
    for (_, insns) in routines.iter_mut() {
        for i in insns.iter_mut() {
            let n = i.len_at(pos);
            if let Insn::Bsr {
                target,
                short: true,
            } = i
            {
                if let Some((_, t)) = outside.iter().find(|(o, _)| o.eq_ignore_ascii_case(target)) {
                    let disp = t - (pos as i64 + 2);
                    *i = Insn::Encoded {
                        words: vec![0x6100 | (disp as u8 as u16)],
                        text: format!("bsr.s {target}"),
                    };
                }
            }
            pos += n;
        }
    }
}

/// The constant pool of one code section (codegen-patterns 1.5,
/// "Constant by address"; `tools/pcsurvey.py`): the cells placed so far,
/// by their constant, and the next label number.
#[derive(Default)]
struct Pool {
    cells: std::collections::HashMap<dpc_ir::PoolConst, u32>,
    next: u32,
}

impl Pool {
    /// Resolve the constant operands of one routine's instructions
    /// (`Ea::Pool`, from the selector) to cell labels, and append after
    /// the routine the cells it is the first to use.
    ///
    /// The layout follows the survey of compiled code 0xE00800..0xE88834
    /// (`tools/pcsurvey.py`, codegen-patterns 1.5 and 11):
    ///
    /// - A cell sits right after the code of the routine that uses it
    ///   first: 733 of the 1038 `(d,PC)` sites naming a constant address a
    ///   cell after their own routine's code, before the next routine; the
    ///   other 305 one after an earlier routine, none a later one.  So the
    ///   last routine's cells come before the module-end padding (OS_TERM
    ///   0xE1AB26, TERM_$INIT 0xE3321E `0002 0001 2048`).
    /// - A later routine names the earlier cell of an equal constant
    ///   (SMD_$INVERT_S 0xE6DDE4 naming SMD_$ASSOC's cell 0xE6D92C).  Equal
    ///   means equal bytes and type ([`dpc_ir::PoolConst::key`]);
    ///   Uncertain: 25 cells repeat an earlier one's bytes (MST_$REMAP
    ///   0xE43A0A and MST_$SET_TOUCH_AHEAD_CNT 0xE4472C, both `0000`),
    ///   which a difference of type would explain.
    /// - A routine's new cells form two groups, cells whose size is 2 mod
    ///   4 first, then those of a multiple of 4; in each, the cell used
    ///   first comes first and the others follow in reverse order of first
    ///   use (TIMER_$INIT 0xE1639C: used in the order 9C, A4, A2, A0, 9E,
    ///   laid out 9C, 9E, A0, A2, A4).  All 110 routines with two or more
    ///   new cells fit (157 have one).
    /// - In a routine that aligns its loop heads, the second group starts
    ///   4-aligned, after a `0x2048` filler when needed (LOG_$SHUTDN
    ///   0xE17606; 45 fillers, none in a routine the loop survey classes
    ///   non-aligning); in one that does not, it is not aligned
    ///   (ACL_$SET_ACL_CHECK 0xE4787A; 13 of the 15 cells at 2 mod 4 are
    ///   in routines classed non-aligning, one in an unclassed one, one,
    ///   0xE74A6A, in the routine at 0xE743CE (in PROC2_$DELETE's map
    ///   extent), the last of its module, which ends with a zero-word
    ///   pad as non-aligning code does).
    fn place(&mut self, insns: &mut Vec<Insn>, aligns: bool) {
        let mut new: Vec<(dpc_ir::PoolConst, u32)> = Vec::new();
        for i in insns.iter_mut() {
            let ea = match i {
                Insn::Pea(ea) | Insn::Lea { src: ea, .. } => ea,
                Insn::Move { src, .. } | Insn::Movea { src, .. } => src,
                _ => continue,
            };
            let crate::insn::Ea::Pool(c) = ea else {
                continue;
            };
            let n = match self.cells.get(c) {
                Some(&n) => n,
                None => {
                    let n = self.next;
                    self.next += 1;
                    self.cells.insert(c.clone(), n);
                    new.push((c.clone(), n));
                    n
                }
            };
            *ea = crate::insn::Ea::PoolLabel(n);
        }
        let (two, four): (Vec<_>, Vec<_>) =
            new.into_iter().partition(|(c, _)| c.bytes.len() % 4 != 0);
        for (g, group) in [two, four].into_iter().enumerate() {
            if group.is_empty() {
                continue;
            }
            if g == 1 && aligns {
                insns.push(Insn::Align4);
            }
            // The first used first, the others in reverse order of use.
            let mut rest = group;
            let first = rest.remove(0);
            rest.reverse();
            for (c, n) in std::iter::once(first).chain(rest) {
                insns.push(Insn::PoolLabel(n));
                insns.push(Insn::Words(
                    c.bytes
                        .chunks(2)
                        .map(|w| u16::from_be_bytes([w[0], w[1]]))
                        .collect(),
                ));
            }
        }
    }
}

/// Choose the size of every `bsr` and `b<cc>` of one code section: `.s`
/// whenever the target is in reach (a displacement of -128..=127 other
/// than 0, which would mean `.w`), as the original did.  Calls: 121
/// `bsr.s`, all in reach, 1800 `bsr.w`, none in reach (codegen-patterns
/// 1.6).  Branches: 11515 `.s`, all in reach, 2793 `.w`, all but two out
/// of reach (0xE7065C, 0xE706DE, both hand assembly in SMD_UTILS;
/// codegen-patterns 7).  Survey of compiled
/// code 0xE00800..0xE88834.  The emitter decides, never gas, so the sizes
/// are explicit (codegen-patterns 13).
///
/// `base` is the section's first byte mod 4 (0, or the fragment
/// origin's: the code's 4-byte alignment is absolute, so a `0x2048`
/// filler's size depends on the position).  Every call and branch starts
/// short; one out of reach becomes long and the layout is redone, until
/// nothing changes (nothing ever shrinks again, so this ends).
fn size_branches(
    routines: &mut [(&dpc_ir::Function, Vec<Insn>)],
    base: u32,
    outside: &[(String, i64)],
) -> Result<(), Diagnostic> {
    use std::collections::HashMap;
    enum Target {
        Routine(String),
        Label(u32),
    }
    loop {
        let mut at = HashMap::new();
        // (routine, label) -> position
        let mut labels = HashMap::new();
        let mut sites = Vec::new();
        // `case` table entries: (routine, table label, target label).
        let mut entries = Vec::new();
        let mut pos = base;
        for (ri, (f, insns)) in routines.iter().enumerate() {
            at.insert(f.symbol.clone(), pos);
            for (ii, i) in insns.iter().enumerate() {
                match i {
                    Insn::Bsr { target, short } => {
                        sites.push((ri, ii, pos, Target::Routine(target.clone()), *short))
                    }
                    Insn::Branch { label, short, .. } => {
                        sites.push((ri, ii, pos, Target::Label(*label), *short))
                    }
                    Insn::Label(l) => {
                        labels.insert((ri, *l), pos);
                    }
                    Insn::TableEntry { table, target, .. } => entries.push((ri, *table, *target)),
                    _ => {}
                }
                pos += i.len_at(pos);
            }
        }
        let mut changed = false;
        for (ri, ii, pos, target, short) in sites {
            let t = match &target {
                Target::Routine(sym) => match at.get(sym) {
                    Some(&t) => t as i64,
                    None if outside.iter().any(|(n, _)| n.eq_ignore_ascii_case(sym)) => {
                        outside
                            .iter()
                            .find(|(n, _)| n.eq_ignore_ascii_case(sym))
                            .unwrap()
                            .1
                    }
                    None => {
                        return Err(Diagnostic::error(
                            routines[ri].0.span,
                            format!("a call of `{sym}`, which is not in this code section, is not supported yet"),
                        ))
                    }
                },
                Target::Label(l) => *labels
                    .get(&(ri, *l))
                    .expect("a branch to a label of its own routine")
                    as i64,
            };
            let disp = t - (pos as i64 + 2);
            if short && !((-128..=127).contains(&disp) && disp != 0) {
                match &mut routines[ri].1[ii] {
                    Insn::Bsr { short, .. } | Insn::Branch { short, .. } => *short = false,
                    _ => unreachable!(),
                }
                changed = true;
            } else if !short && !(-0x8000..0x8000).contains(&disp) {
                return Err(Diagnostic::error(
                    routines[ri].0.span,
                    "a call or branch beyond the reach of a 16-bit displacement is not supported yet",
                ));
            }
        }
        if !changed {
            // A table entry is a word offset from the table's start
            // (codegen-patterns 7.3: 0xE5C6D6).
            for (ri, table, target) in entries {
                let d = labels[&(ri, target)] as i64 - labels[&(ri, table)] as i64;
                if !(-0x8000..0x8000).contains(&d) {
                    return Err(Diagnostic::error(
                        routines[ri].0.span,
                        "a `case` arm beyond the reach of a 16-bit table offset is not supported yet",
                    ));
                }
            }
            return Ok(());
        }
    }
}

/// The output section of unit `unit`'s static data area in Pascal data
/// section `sect`: the NOBITS section `.bss.moddata.<UNIT>` for the
/// default section, `.bss.moddata.<UNIT>.<SECT>` for another, upper-cased
/// like every external name (codegen-patterns 11); for the kernel's units
/// `<UNIT>` is the map's `D` segment name (`PROC1_`, `PROC2`) and
/// `<SECT>` its section (WIRED_DATA, OS_INIT_DATA).
///
/// Only the prefix is `../source`'s: there a block is the input section
/// `.bss.moddata.<name>` / `.moddata.<name>` where `<name>` is the C
/// object of a `MODULE_DATA_DEFINE` site (`PROC1_$DATA`,
/// `PROC2_$UNWIRED_DATA`; `../source/domain_os/arch/m68k/arch.h`), and
/// `tools/gen_layout_ld.py` orders a block by the address literal of that
/// C site, never by the map's segment name.  A section of ours would get
/// no ordering key there; linking a dpc unit into `../source` needs a
/// mapping that does not exist yet (codegen-patterns 11).
pub fn data_section_name(unit: &str, sect: &str) -> String {
    if sect == DEFAULT_DATA_SECTION {
        format!(".bss.moddata.{}", unit.to_uppercase())
    } else {
        format!(
            ".bss.moddata.{}.{}",
            unit.to_uppercase(),
            sect.to_uppercase()
        )
    }
}

/// The local label at the start of the unit's data area in Pascal data
/// section `sect`: what `lea <label>.l,A5` loads in the routines of that
/// data section (codegen-patterns 4: one A5 value per segment, the block
/// of the data section paired with the code section).
/// [`DATA_BASE_LABEL`] for the default section.
pub fn data_base_label(sect: &str) -> String {
    if sect == DEFAULT_DATA_SECTION {
        DATA_BASE_LABEL.into()
    } else {
        format!("{DATA_BASE_LABEL}.{}", sect.to_uppercase())
    }
}

/// The unit's data area in one data section: one zero-filled block
/// starting at its [`data_base_label`], `define`d variables as global
/// symbols at their offsets.
///
/// The block is 4-aligned and its size a multiple of 4: all 161 `D`
/// segments of the SAU2 map start at a multiple of 4 and have such a
/// size (PROC1_ 0xE254E8, size 0xCC4).  Whether the compiler or the
/// binder rounded is not known.
fn data_area(unit: &str, data: &dpc_ir::DataArea) -> String {
    let mut out = format!(
        "\n\t.section\t\"{}\",\"aw\",@nobits\n\t.balign\t4\n{}:\n",
        data_section_name(unit, &data.section),
        data_base_label(&data.section)
    );
    let size = data.size.div_ceil(4) * 4;
    let mut at = 0;
    for (sym, off) in &data.globals {
        if *off > at {
            out.push_str(&format!("\t.space\t{}\n", off - at));
            at = *off;
        }
        out.push_str(&format!("\t.globl\t{sym}\n{sym}:\n"));
    }
    if size > at {
        out.push_str(&format!("\t.space\t{}\n", size - at));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_module_is_only_the_header() {
        let s = emit(&Module::default(), "x.pas", &Options::default()).unwrap();
        assert_eq!(s, "| generated by dpc from x.pas\n");
    }

    fn one_routine(section: &str, symbol: &str) -> dpc_ir::Function {
        let span = dpc_syntax::Span::new(
            dpc_syntax::SourceMap::new().add("t.pas", String::new()),
            0,
            0,
        );
        dpc_ir::Function {
            symbol: symbol.into(),
            global: true,
            code_section: section.into(),
            data_section: ".data".into(),
            frame_size: 0,
            result: None,
            loads_data_base: false,
            data_absolute: false,
            value_params: Vec::new(),
            address_params: Vec::new(),
            scalar_locals: Vec::new(),
            nesting: None,
            uplevel_refs: Vec::new(),
            param_copies: Vec::new(),
            result_slot: None,
            body: Vec::new(),
            span,
        }
    }

    /// An 8-byte routine, `link.w; unlk; rts`: a function with an empty
    /// body (an empty procedure is a bare `rts`, codegen-patterns 2).
    fn framed(symbol: &str) -> dpc_ir::Function {
        dpc_ir::Function {
            result: Some(dpc_ir::ResultSpec {
                class: dpc_ir::ResultClass::Data,
                ty: dpc_ir::Scalar::signed(dpc_ir::Width::Word),
            }),
            ..one_routine(".text", symbol)
        }
    }

    fn module(sections: &[&str]) -> Module {
        Module {
            functions: sections
                .iter()
                .enumerate()
                .map(|(i, s)| one_routine(s, &format!("P{i}")))
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn whole_module_gets_end_padding() {
        let s = emit(&module(&[".text"]), "m.pas", &Options::default()).unwrap();
        assert!(
            s.ends_with("\trts\n\t.balignw\t4,0x2048\t| movea.l %a0,%a0 filler\n"),
            "{s}"
        );
        assert!(!s.contains(FRAGMENT_ORIGIN_SYMBOL), "{s}");
        assert!(!s.contains(".space"), "{s}");
    }

    #[test]
    fn fragment_at_2_mod_4() {
        let opts = Options {
            fragment_origin: Some(0xE148CA),
            ..Default::default()
        };
        let s = emit(&module(&[".text"]), "m.pas", &opts).unwrap();
        assert!(
            s.contains("\t.set\t__dpc_fragment_origin,0xe148ca\n"),
            "{s}"
        );
        // the pad comes right after the section directive, before any code
        assert!(s.contains("\t.text\n\t.space\t2\t|"), "{s}");
        // no module-end padding
        assert!(!s.contains(".balignw"), "{s}");
        assert!(s.ends_with("\trts\n"), "{s}");
    }

    #[test]
    fn fragment_at_0_mod_4_has_no_pad() {
        let opts = Options {
            fragment_origin: Some(0xE00EB0),
            ..Default::default()
        };
        let s = emit(&module(&[".text"]), "m.pas", &opts).unwrap();
        assert!(
            s.contains("\t.set\t__dpc_fragment_origin,0xe00eb0\n"),
            "{s}"
        );
        assert!(!s.contains(".space"), "{s}");
        assert!(!s.contains(".balignw"), "{s}");
    }

    #[test]
    fn fragment_that_ends_its_module_gets_end_padding() {
        let opts = Options {
            fragment_origin: Some(0xE148CA),
            fragment_ends_module: true,
            ..Default::default()
        };
        let s = emit(&module(&[".text"]), "m.pas", &opts).unwrap();
        assert!(s.contains("\t.text\n\t.space\t2\t|"), "{s}");
        assert!(
            s.ends_with("\trts\n\t.balignw\t4,0x2048\t| movea.l %a0,%a0 filler\n"),
            "{s}"
        );
    }

    #[test]
    fn fragment_with_two_code_sections_is_rejected() {
        let opts = Options {
            fragment_origin: Some(0xE148CA),
            ..Default::default()
        };
        assert!(emit(&module(&[".text", "wired_proc"]), "m.pas", &opts).is_err());
    }

    #[test]
    fn data_area_is_a_zero_block_after_the_code() {
        let mut m = module(&[".text"]);
        m.name = "proc1_".into();
        m.data = vec![dpc_ir::DataArea {
            section: ".data".into(),
            size: 14,
            globals: vec![("PROC1_$X".into(), 2), ("PROC1_$Y".into(), 8)],
        }];
        let s = emit(&m, "m.pas", &Options::default()).unwrap();
        let tail = s.split("filler\n").nth(1).unwrap();
        assert_eq!(
            tail,
            "\n\t.section\t\".bss.moddata.PROC1_\",\"aw\",@nobits\n\t.balign\t4\n.Ldata:\n\
             \t.space\t2\n\t.globl\tPROC1_$X\nPROC1_$X:\n\
             \t.space\t6\n\t.globl\tPROC1_$Y\nPROC1_$Y:\n\
             \t.space\t8\n"
        );
    }

    #[test]
    fn section_directives() {
        assert_eq!(section_directive("proc1_", ".text"), "\t.text");
        assert_eq!(
            section_directive("proc1_", "wired_proc"),
            "\t.section\t\".text.PROC1_.WIRED_PROC\",\"ax\",@progbits"
        );
        assert_eq!(data_section_name("proc1_", ".data"), ".bss.moddata.PROC1_");
        assert_eq!(
            data_section_name("proc1_", "wired_data"),
            ".bss.moddata.PROC1_.WIRED_DATA"
        );
        assert_eq!(data_base_label(".data"), ".Ldata");
        assert_eq!(data_base_label("wired_data"), ".Ldata.WIRED_DATA");
    }

    /// A routine whose body is `n` calls of `P` with a long constant
    /// (14 bytes each), then optionally a call of `local`.
    fn caller(symbol: &str, n: usize, local: Option<&str>) -> dpc_ir::Function {
        use dpc_ir::{Arg, Call, Callee, Expr, Scalar, Stmt, StmtKind, Width};
        let mut f = one_routine(".text", symbol);
        let mut body: Vec<StmtKind> = (0..n)
            .map(|_| {
                StmtKind::Call(Call {
                    callee: Callee::External("P".into()),
                    args: vec![Arg::Value {
                        value: Expr::Const {
                            value: 0x12345,
                            ty: Scalar::signed(Width::Long),
                        },
                        slot: 4,
                    }],
                    pad: 0,
                    link: None,
                })
            })
            .collect();
        if let Some(l) = local {
            body.push(StmtKind::Call(Call {
                callee: Callee::Local(l.into()),
                args: vec![],
                pad: 0,
                link: None,
            }));
        }
        f.body = body
            .into_iter()
            .map(|kind| Stmt { kind, span: f.span })
            .collect();
        f
    }

    /// `b<cc>` sizes follow the distance to the label of the branch's own
    /// routine (two routines here use the same label numbers), forward
    /// and backward (codegen-patterns 7).
    #[test]
    fn branch_size_follows_the_distance() {
        use dpc_ir::{Addr, Cond, Expr, Scalar, Stmt, StmtKind, Width};
        let store = |f: &dpc_ir::Function| Stmt {
            kind: StmtKind::Store {
                addr: Addr::Symbol {
                    name: "X".into(),
                    offset: 0,
                },
                value: Expr::Const {
                    value: 0x12345,
                    ty: Scalar::signed(Width::Long),
                },
            },
            span: f.span,
        };
        let test = Cond::Test(Expr::Load {
            addr: Addr::Symbol {
                name: "B".into(),
                offset: 0,
            },
            ty: Scalar::BOOLEAN,
        });
        // A `repeat` whose body is `n` 10-byte stores.
        let looping = |sym: &str, n: usize| {
            let mut f = one_routine(".text", sym);
            let body = (0..n).map(|_| store(&f)).collect();
            f.body = vec![Stmt {
                kind: StmtKind::Repeat {
                    body,
                    until: test.clone(),
                },
                span: f.span,
            }];
            f
        };
        let m = Module {
            functions: vec![looping("NEAR", 2), looping("FAR", 13)],
            ..Default::default()
        };
        let s = emit(&m, "m.pas", &Options::default()).unwrap();
        let branches: Vec<&str> = s
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("bpl"))
            .collect();
        assert_eq!(branches, ["bpl.s\t1b", "bpl.w\t1b"], "{s}");
    }

    /// A routine whose body is `repeat X := 0x12345 until B`: one loop
    /// head, aligned when the routine aligns.
    fn one_loop(symbol: &str) -> dpc_ir::Function {
        use dpc_ir::{Addr, Cond, Expr, Scalar, Stmt, StmtKind, Width};
        let mut f = one_routine(".text", symbol);
        let store = Stmt {
            kind: StmtKind::Store {
                addr: Addr::Symbol {
                    name: "X".into(),
                    offset: 0,
                },
                value: Expr::Const {
                    value: 0x12345,
                    ty: Scalar::signed(Width::Long),
                },
            },
            span: f.span,
        };
        f.body = vec![Stmt {
            kind: StmtKind::Repeat {
                body: vec![store],
                until: Cond::Test(Expr::Load {
                    addr: Addr::Symbol {
                        name: "B".into(),
                        offset: 0,
                    },
                    ty: Scalar::BOOLEAN,
                }),
            },
            span: f.span,
        }];
        f
    }

    fn loop_align(la: LoopAlign) -> Options {
        Options {
            loop_align: la,
            ..Default::default()
        }
    }

    /// The text of each routine of `s`, by symbol.
    fn routine_text<'a>(s: &'a str, sym: &str) -> &'a str {
        let start = s.find(&format!("\n{sym}:\n")).unwrap();
        let rest = &s[start + 1..];
        let end = rest.find("\trts\n").unwrap();
        &rest[..end]
    }

    const FILLER_END: &str = "\trts\n\t.balignw\t4,0x2048\t| movea.l %a0,%a0 filler\n";
    const ZERO_END: &str =
        "\trts\n\t.balignw\t4,0x0000\t| zero words: the last routine does not align\n";

    /// codegen-patterns 7: `--loop-align=off` drops every loop-head
    /// filler, and the module end is padded with zero words.
    #[test]
    fn loop_align_off() {
        let m = Module {
            functions: vec![one_loop("A"), one_loop("B")],
            ..Default::default()
        };
        let on = emit(&m, "m.pas", &Options::default()).unwrap();
        assert_eq!(on.matches(".balignw\t4,0x2048").count(), 3, "{on}");
        assert!(on.ends_with(FILLER_END), "{on}");
        let off = emit(&m, "m.pas", &loop_align(LoopAlign::Off)).unwrap();
        assert!(!off.contains("0x2048"), "{off}");
        assert!(off.ends_with(ZERO_END), "{off}");
    }

    /// `--loop-align-off-from`: on before the named routine, off from it
    /// (in either spelling) on; the end padding follows the last routine.
    #[test]
    fn loop_align_off_from() {
        let m = Module {
            functions: vec![one_loop("P_$A"), one_loop("P_$B"), one_loop("P_$C")],
            ..Default::default()
        };
        let s = emit(
            &m,
            "m.pas",
            &loop_align(LoopAlign::OffFrom(vec!["p_$b".into()])),
        )
        .unwrap();
        assert!(routine_text(&s, "P_$A").contains("0x2048"), "{s}");
        assert!(!routine_text(&s, "P_$B").contains("0x2048"), "{s}");
        assert!(!routine_text(&s, "P_$C").contains("0x2048"), "{s}");
        assert!(s.ends_with(ZERO_END), "{s}");
        // from the first routine: the same as off
        let s = emit(
            &m,
            "m.pas",
            &loop_align(LoopAlign::OffFrom(vec!["P_$A".into()])),
        )
        .unwrap();
        assert_eq!(s, emit(&m, "m.pas", &loop_align(LoopAlign::Off)).unwrap());
        // a fragment that ends its module: the padding follows the last
        // routine too; one that does not end it has none
        let tail = Options {
            fragment_origin: Some(0x1000),
            fragment_ends_module: true,
            loop_align: LoopAlign::OffFrom(vec!["P_$C".into()]),
            ..Default::default()
        };
        let s = emit(&m, "m.pas", &tail).unwrap();
        assert!(routine_text(&s, "P_$B").contains("0x2048"), "{s}");
        assert!(s.ends_with(ZERO_END), "{s}");
        let mid = Options {
            fragment_ends_module: false,
            ..tail
        };
        let s = emit(&m, "m.pas", &mid).unwrap();
        assert!(s.ends_with("\trts\n"), "{s}");
        let e = emit(
            &m,
            "m.pas",
            &loop_align(LoopAlign::OffFrom(vec!["P_$D".into()])),
        )
        .unwrap_err();
        assert!(e[0].message.contains("no routine `P_$D`"), "{e:?}");
    }

    /// `--loop-align-off-from` switches only the named routine's code
    /// section (one switch per map segment, codegen-patterns 7): with `A`
    /// (.text), `B` (S2), `C` (.text), off from `B`, `A` and `C` align.
    /// Each section's end padding follows its own last routine.
    #[test]
    fn loop_align_off_from_follows_emission_order() {
        let mut b = one_loop("B");
        b.code_section = "S2".into();
        let m = Module {
            name: "m".into(),
            functions: vec![one_loop("A"), b, one_loop("C")],
            ..Default::default()
        };
        let s = emit(
            &m,
            "m.pas",
            &loop_align(LoopAlign::OffFrom(vec!["B".into()])),
        )
        .unwrap();
        assert!(routine_text(&s, "A").contains("0x2048"), "{s}");
        assert!(routine_text(&s, "C").contains("0x2048"), "{s}");
        assert!(!routine_text(&s, "B").contains("0x2048"), "{s}");
        let (text, s2) = s.split_once("\n\t.section\t\".text.M.S2\"").unwrap();
        assert!(text.ends_with(FILLER_END), "{s}");
        assert!(s2.ends_with(ZERO_END), "{s}");
    }

    /// One switch per code section: off from `C` in .text and from `B` in
    /// S2; `A` aligns.  Two names in one section are an error.
    #[test]
    fn loop_align_off_from_per_section() {
        let mut b = one_loop("B");
        b.code_section = "S2".into();
        let m = Module {
            name: "m".into(),
            functions: vec![one_loop("A"), b, one_loop("C")],
            ..Default::default()
        };
        let s = emit(
            &m,
            "m.pas",
            &loop_align(LoopAlign::OffFrom(vec!["c".into(), "b".into()])),
        )
        .unwrap();
        assert!(routine_text(&s, "A").contains("0x2048"), "{s}");
        assert!(!routine_text(&s, "B").contains("0x2048"), "{s}");
        assert!(!routine_text(&s, "C").contains("0x2048"), "{s}");
        let e = emit(
            &m,
            "m.pas",
            &loop_align(LoopAlign::OffFrom(vec!["A".into(), "C".into()])),
        )
        .unwrap_err();
        assert!(
            e[0].message
                .contains("two routines of code section `.text`"),
            "{e:?}"
        );
    }

    /// Without its filler a routine is 2 bytes shorter, which can bring
    /// a call over it into `bsr.s` reach: branch sizes are chosen after
    /// the fillers are dropped.  In a fragment at 0x1002, `T` is 8 bytes,
    /// `A`'s loop head is at 2 mod 4, then 11 8-byte routines, then `B`
    /// calls `T` backward: a displacement of -128 without the filler,
    /// -130 with it.
    #[test]
    fn branch_sizes_see_dropped_fillers() {
        let probe = |la: LoopAlign| {
            let mut functions = vec![framed("T"), one_loop("A")];
            functions.extend((0..11).map(|i| framed(&format!("E{i}"))));
            functions.push(caller("B", 0, Some("T")));
            let m = Module {
                functions,
                ..Default::default()
            };
            let opts = Options {
                fragment_origin: Some(0x1002),
                loop_align: la,
                ..Default::default()
            };
            let s = emit(&m, "m.pas", &opts).unwrap();
            bsr_line(&s).to_string()
        };
        assert_eq!(probe(LoopAlign::On), "bsr.w\tT");
        assert_eq!(probe(LoopAlign::Off), "bsr.s\tT");
    }

    fn bsr_line(s: &str) -> &str {
        s.lines().find(|l| l.contains("bsr")).unwrap().trim()
    }

    /// `bsr.s` when the target is in reach, `bsr.w` otherwise
    /// (codegen-patterns 1.6), backward and forward.
    #[test]
    fn bsr_size_follows_the_distance() {
        let m = |fs| Module {
            functions: fs,
            ..Default::default()
        };
        let opts = Options::default();
        // Backward, near: 0xE00F02 `bsr.s` (61ac).
        let s = emit(
            &m(vec![caller("A", 1, None), caller("B", 1, Some("A"))]),
            "m.pas",
            &opts,
        )
        .unwrap();
        assert_eq!(bsr_line(&s), "bsr.s\tA");
        // Backward, 10 calls of 14 bytes in between: out of reach.
        let s = emit(
            &m(vec![caller("A", 10, None), caller("B", 0, Some("A"))]),
            "m.pas",
            &opts,
        )
        .unwrap();
        assert_eq!(bsr_line(&s), "bsr.w\tA");
        // Forward: the target follows the caller.
        let s = emit(
            &m(vec![caller("B", 0, Some("A")), caller("A", 1, None)]),
            "m.pas",
            &opts,
        )
        .unwrap();
        assert_eq!(bsr_line(&s), "bsr.s\tA");
        // Forward and out of reach: the call first, 10 calls after it.
        let mut b = caller("B", 10, Some("A"));
        b.body.rotate_right(1);
        let s = emit(&m(vec![b, caller("A", 0, None)]), "m.pas", &opts).unwrap();
        assert_eq!(bsr_line(&s), "bsr.w\tA");
        // A target not in the section.
        let e = emit(&m(vec![caller("B", 0, Some("Z"))]), "m.pas", &opts).unwrap_err();
        assert!(e[0].message.contains("not in this code section"), "{e:?}");
    }

    /// `--module-routine`: a call of a routine of the module outside the
    /// fragment is a `bsr`, sized from its address (DISK_$UNASSIGN_ALL
    /// 0xE6BE34 `bsr.w` back 0x8E bytes to DISK_$UNASSIGN 0xE6BDA8).
    /// The constant pool (codegen-patterns 1.5): a routine's new cells
    /// after its code, those of 2 mod 4 bytes first, then the others
    /// (4-aligned with the filler in an aligning routine); in each group
    /// the first used first, the rest in reverse order of use (TIMER_$INIT
    /// 0xE1639C); cells already placed are named again, not repeated.
    #[test]
    fn pool_cells_after_the_routine() {
        use crate::insn::Ea;
        use dpc_ir::PoolConst;
        let w = |v: u8| PoolConst {
            bytes: vec![0, v],
            key: 1,
        };
        let l = |v: u8| PoolConst {
            bytes: vec![0, 0, 0, v],
            key: 2,
        };
        let pea = |c: PoolConst| Insn::Pea(Ea::Pool(c));
        let mut pool = Pool::default();
        let mut a = vec![
            pea(w(1)),
            pea(l(9)),
            pea(w(2)),
            pea(w(3)),
            pea(w(1)),
            Insn::Rts,
        ];
        pool.place(&mut a, true);
        let text: Vec<String> = a.iter().map(|i| i.to_string()).collect();
        assert_eq!(
            text,
            [
                "pea\t(.Lc0,%pc)",
                "pea\t(.Lc1,%pc)",
                "pea\t(.Lc2,%pc)",
                "pea\t(.Lc3,%pc)",
                "pea\t(.Lc0,%pc)",
                "rts",
                ".Lc0:",
                ".short\t0x0001",
                ".Lc3:",
                ".short\t0x0003",
                ".Lc2:",
                ".short\t0x0002",
                ".balignw\t4,0x2048",
                ".Lc1:",
                ".short\t0x0000,0x0009",
            ]
        );
        // a later routine: the placed cells by label, a new one (same
        // bytes, another type) after it; no filler when not aligning
        let mut b = vec![
            pea(w(2)),
            pea(PoolConst {
                bytes: vec![0, 2],
                key: 7,
            }),
            pea(l(4)),
            Insn::Rts,
        ];
        pool.place(&mut b, false);
        let text: Vec<String> = b.iter().map(|i| i.to_string()).collect();
        assert_eq!(
            text,
            [
                "pea\t(.Lc2,%pc)",
                "pea\t(.Lc4,%pc)",
                "pea\t(.Lc5,%pc)",
                "rts",
                ".Lc4:",
                ".short\t0x0002",
                ".Lc5:",
                ".short\t0x0000,0x0004",
            ]
        );
    }

    /// Branch sizes see the cells: a `bsr` over a routine with a pool.
    #[test]
    fn branch_sizes_count_pool_cells() {
        use dpc_ir::{Addr, Arg, Call, Callee, PoolConst, Stmt, StmtKind};
        // P calls R over Q; Q: n calls with a constant, then its cell.
        let q0 = one_routine(".text", "Q");
        let span = q0.span;
        let call = |c: PoolConst| Stmt {
            kind: StmtKind::Call(Call {
                callee: Callee::External("X".into()),
                args: vec![Arg::Address {
                    addr: Addr::Const(c),
                    slot: 4,
                    writable: false,
                }],
                pad: 0,
                link: None,
            }),
            span,
        };
        let mut p = one_routine(".text", "P");
        p.body = vec![Stmt {
            kind: StmtKind::Call(Call {
                callee: Callee::Local("R".into()),
                args: vec![],
                pad: 0,
                link: None,
            }),
            span,
        }];
        let r = one_routine(".text", "R");
        let module = |n: usize, cell: usize| {
            let mut q = q0.clone();
            q.body = (0..n)
                .map(|_| {
                    call(PoolConst {
                        bytes: vec![0; cell],
                        key: 0,
                    })
                })
                .collect();
            Module {
                name: "M".into(),
                functions: vec![p.clone(), q, r.clone()],
                ..Default::default()
            }
        };
        // P: link, `bsr` at 4, unlk, rts; Q at 10: link, 9 calls of 12
        // bytes less the last pop, unlk, rts, the cell; R at 124 + cell.
        let reach = |s: &str| s.lines().find(|l| l.contains("bsr")).unwrap().to_string();
        let s = emit(&module(9, 2), "m.pas", &Options::default()).unwrap();
        let short = reach(&s);
        let s = emit(&module(9, 16), "m.pas", &Options::default()).unwrap();
        let long = reach(&s);
        assert!(short.contains("bsr.s"), "{short}");
        assert!(long.contains("bsr.w"), "{long}");
    }

    /// Each code section has its own cells, but their labels are numbered
    /// across the file: two sections' cells never share a label.
    #[test]
    fn pool_labels_unique_across_sections() {
        use dpc_ir::{Addr, Arg, Call, Callee, PoolConst, Stmt, StmtKind};
        let with_const = |section: &str, symbol: &str| {
            let mut f = one_routine(section, symbol);
            let span = f.span;
            f.body = vec![Stmt {
                kind: StmtKind::Call(Call {
                    callee: Callee::External("X".into()),
                    args: vec![Arg::Address {
                        addr: Addr::Const(PoolConst {
                            bytes: vec![0, 1],
                            key: 0,
                        }),
                        slot: 4,
                        writable: false,
                    }],
                    pad: 0,
                    link: None,
                }),
                span,
            }];
            f
        };
        let m = Module {
            name: "M".into(),
            functions: vec![with_const(".text", "P"), with_const("OTHER", "Q")],
            ..Default::default()
        };
        let s = emit(&m, "m.pas", &Options::default()).unwrap();
        assert_eq!(s.matches(".Lc0:").count(), 1, "{s}");
        assert_eq!(s.matches(".Lc1:").count(), 1, "{s}");
    }

    #[test]
    fn calls_of_module_routines_outside_a_fragment() {
        use dpc_ir::{Call, Callee, Stmt, StmtKind};
        let mut f = one_routine(".text", "P");
        let call = Stmt {
            kind: StmtKind::Call(Call {
                callee: Callee::External("Q".into()),
                args: Vec::new(),
                pad: 0,
                link: None,
            }),
            span: f.span,
        };
        f.body = vec![call.clone(), call];
        let m = Module {
            name: "M".into(),
            functions: vec![f],
            ..Default::default()
        };
        let frag = |at: u32| Options {
            fragment_origin: Some(0x1000),
            module_routines: vec![("Q".into(), at)],
            ..Default::default()
        };
        // P: link at 0x1000, the calls at 0x1004 and 0x1006 (short),
        // written as their encodings: gas cannot relocate an 8-bit
        // displacement to a symbol it does not define.
        let shorts = ".short\t0x617a\t| bsr.s Q\n\t.short\t0x6178\t| bsr.s Q\n";
        let s = emit(&m, "m.pas", &frag(0x1080)).unwrap();
        assert!(s.contains(shorts), "{s}");
        // 0x1100 is out of `.s` reach from 0x1004.
        let s = emit(&m, "m.pas", &frag(0x1100)).unwrap();
        assert!(s.contains("\tbsr.w\tQ\n\tbsr.w\tQ\n"), "{s}");
        // The option's spelling does not matter: the symbol is the IR's.
        let mut lower = frag(0x1080);
        lower.module_routines[0].0 = "q".into();
        let s = emit(&m, "m.pas", &lower).unwrap();
        assert!(s.contains(shorts), "{s}");
        // without the option: `jsr`
        let s = emit(&m, "m.pas", &Options::default()).unwrap();
        assert!(s.contains("\tjsr\t(Q).l\n"), "{s}");
        let mut opts = frag(0x1100);
        opts.module_routines[0].0 = "p".into();
        let e = emit(&m, "m.pas", &opts).unwrap_err();
        assert!(e[0].message.contains("has its body in this unit"), "{e:?}");
    }
}
