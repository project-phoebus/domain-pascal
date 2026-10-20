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
//! (PROC1_$TST_LOCK 0xE148CA).  [`Options::fragment`] describes
//! such a unit; then:
//!
//! - the code section starts with `origin mod 4` bytes of `.space`, so
//!   that section-relative `.balignw 4` equals absolute alignment once the
//!   section is linked at `origin & !3`.  These bytes are not part of the
//!   unit; the absolute local symbol [`FRAGMENT_ORIGIN_SYMBOL`] records
//!   `origin` so that `tools/conform.py` can link and compare accordingly;
//! - no module-end padding is emitted, unless
//!   [`FragmentSection::ends_module`] says that the unit is the tail of
//!   its module (then the padding, absolute like the rest, is part of the
//!   unit's last routine in the image).
//!
//! A unit with several code sections (TTY: WIRED_PROC, OS_INIT_PROC and
//! the default section) is a fragment of each: every code section has its
//! own origin ([`FragmentSection`], `dpc --fragment-origin=SECTION=0xADDR`),
//! its own pad, its own module-end padding or none, and the absolute local
//! symbol [`FRAGMENT_ORIGIN_SYMBOL`] followed by its output section's name
//! (`__dpc_fragment_origin.text.TTY.WIRED_PROC`).  A routine of the module
//! outside the unit ([`ModuleRoutine`]) is reached as the whole module
//! reached it: with `bsr` from its own code section, sized from its
//! address and that section's origin, and with `jsr abs.l` from any other
//! (`bsr` never crosses a map segment, codegen-patterns 1.6, 11).
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

/// One code section of a fragment (`dpc --fragment-origin`): where its
/// first byte belongs in the image, and whether it ends its module.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FragmentSection {
    /// The Pascal code section, as the source names it (`.text` for the
    /// default; matched case-insensitively), or `None`: the unit's only
    /// code section (`dpc --fragment-origin 0xADDR`).
    pub section: Option<String>,
    /// The image address of the section's first byte (even).
    pub origin: u32,
    /// The section ends its module's segment, so it gets the module-end
    /// padding (`dpc --fragment-ends-module[=SECTION]`).
    pub ends_module: bool,
}

impl FragmentSection {
    /// The unit's only code section, at `origin`.
    pub fn only(origin: u32) -> Self {
        FragmentSection {
            section: None,
            origin,
            ends_module: false,
        }
    }
}

/// A routine of the module outside a fragment (`dpc
/// --module-routine=[SECTION:]NAME=0xADDR`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ModuleRoutine {
    /// Its Pascal code section (matched case-insensitively), or `None`:
    /// the unit's only code section.  A section the unit has no routine
    /// in is allowed: the routine is then called with `jsr abs.l`, as
    /// from any other section.
    pub section: Option<String>,
    /// Its symbol (matched case-insensitively against the IR's).
    pub symbol: String,
    /// Its image address.
    pub addr: u32,
}

impl ModuleRoutine {
    /// A routine of the unit's only code section.
    pub fn new(symbol: &str, addr: u32) -> Self {
        ModuleRoutine {
            section: None,
            symbol: symbol.into(),
            addr,
        }
    }
}

/// Emitter options.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Options {
    /// The output is a fragment of a module (test-oriented: `dpc
    /// --fragment-origin`): one entry per code section of the unit, each
    /// with the absolute address of the section's first byte.  Empty: a
    /// whole module.  An entry without a section is the only one, for a
    /// unit with at most one code section.
    pub fragment: Vec<FragmentSection>,
    /// Which routines align their loop heads.
    pub loop_align: LoopAlign,
    /// With `fragment`: routines of the same module that lie outside the
    /// fragment, with their code section and image address (`dpc
    /// --module-routine`).  The original compiled whole modules and
    /// called such a routine with `bsr` from its own code section
    /// (codegen-patterns 1.6: DISK_$UNASSIGN_ALL 0xE6BE34 `bsr.w` to
    /// DISK_$UNASSIGN 0xE6BDA8, the routine before it) and with `jsr
    /// abs.l` from another (codegen-patterns 11: TTY_$K_RESET 0xE6730E to
    /// the WIRED_PROC module-local 0xE1AED0); the fragment declares it
    /// `extern`, and its `jsr` from the routine's own section becomes a
    /// `bsr` sized from the address, as for a routine of the fragment.
    /// The reference resolves to the map name when the fragment is
    /// linked.  An unnamed module-local routine is declared `internal;
    /// extern` and named by its local symbol, which the conformance
    /// harness defines (`--defsym`; bead domain-pascal-3h9).
    pub module_routines: Vec<ModuleRoutine>,
    /// The scheduling policy for the bodies (`dpc --schedule`;
    /// `sched.rs`, codegen-patterns 12.4 R9-R12).
    pub schedule: crate::sched::Schedule,
    /// Pascal data sections (as the source names them, `.data` for the
    /// default; matched case-insensitively) whose data area another object
    /// owns, with the global symbol of its first byte (`dpc --data-base`;
    /// link-test scaffolding, bead domain-pascal-3bp,
    /// `tests/link/README.md`).  For such a section the unit allocates
    /// nothing: its [`data_base_label`] is set to the symbol, so `lea
    /// <label>.l,A5` (and `--a5-absolute`'s `lea`) is a relocation against
    /// it; its `define`d variables are not defined by the unit (the
    /// owner's to define); and the absolute local symbol
    /// [`data_size_symbol`] records the bytes the unit declares there, so
    /// a harness can check they fit in the owner's object.  The code is
    /// byte for byte the same as without the option.
    pub data_bases: Vec<(String, String)>,
    /// The unit merges pairs of word constants stored to adjacent words
    /// into one longword store (`dpc --merge-constant-stores`; ref 3.15.3
    /// p. 3-57; codegen-patterns 13, "Store merging"; bead
    /// domain-pascal-qb9q): a per-unit setting, on where the image's
    /// segment merges (`tools/storesurvey.py units`).  Off, such pairs
    /// are two stores, as in the units that do not merge.
    pub merge_constant_stores: bool,
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
/// implemented yet, or if the fragment options do not fit the unit's code
/// sections ([`Options::fragment`], [`Options::module_routines`]).
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
    // The overlay sections' symbols and sizes, for the base their arrays
    // are reached from (`isel::Selector::symbol_bias`).
    let overlays: Vec<(String, u32)> = module
        .overlays
        .iter()
        .map(|o| (dpc_ir::overlay_symbol(&o.section), o.size))
        .collect();
    for (i, (sect, _)) in opts.data_bases.iter().enumerate() {
        if opts.data_bases[..i]
            .iter()
            .any(|(s, _)| s.eq_ignore_ascii_case(sect))
        {
            return Err(vec![Diagnostic::error_nopos(format!(
                "--data-base: data section `{sect}` given more than once"
            ))]);
        }
        if !module
            .data
            .iter()
            .any(|d| d.section.eq_ignore_ascii_case(sect))
            && !module
                .functions
                .iter()
                .any(|f| f.data_section.eq_ignore_ascii_case(sect))
        {
            return Err(vec![Diagnostic::error_nopos(format!(
                "--data-base: no variable or routine of this unit is in data section `{sect}`"
            ))]);
        }
    }
    if let Some(r) = opts.module_routines.iter().find(|r| {
        module
            .functions
            .iter()
            .any(|f| f.symbol.eq_ignore_ascii_case(&r.symbol))
    }) {
        return Err(vec![Diagnostic::error_nopos(format!(
            "--module-routine: `{}` has its body in this unit",
            r.symbol
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
                .any(|r| r.symbol.eq_ignore_ascii_case(sym))
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
    let fragment = fragment_sections(&sections, opts).map_err(|d| vec![d])?;
    let homes = module_routine_sections(&sections, opts).map_err(|d| vec![d])?;
    if sections.len() <= 1 {
        // One code section (or none): the plain symbol.
        if let Some(origin) = opts.fragment.first().map(|f| f.origin) {
            out.push_str(&format!(
                "| fragment of a module, first byte at 0x{origin:X} (dpc --fragment-origin)\n\
             \t.set\t{FRAGMENT_ORIGIN_SYMBOL},0x{origin:x}\n"
            ));
        }
    } else {
        // One origin per code section, named after its output section.
        for (sec, f) in &fragment {
            let name = code_section_name(&module.name, sec);
            out.push_str(&format!(
                "| code section {name}: fragment of its module's segment, first byte at \
                 0x{origin:X} (dpc --fragment-origin)\n\
                 \t.set\t{},0x{origin:x}\n",
                fragment_origin_symbol(&name),
                origin = f.origin,
            ));
        }
    }
    // Cell labels are numbered across the code sections: each section has
    // its own cells, but all share the one assembly file.
    let mut pool_labels = 0;
    for sec in sections {
        out.push('\n');
        out.push_str(&section_directive(&module.name, sec));
        out.push('\n');
        // This section's fragment origin, if a fragment.
        let frag = fragment.iter().find(|(s, _)| *s == sec).map(|(_, f)| *f);
        if let Some(f) = frag {
            if f.origin % 4 != 0 {
                out.push_str(&format!(
                    "\t.space\t{}\t| not part of the unit: makes 4-byte alignment absolute\n",
                    f.origin % 4
                ));
            }
        }
        // The routines outside the unit that live in this code section:
        // called with `bsr`; those of other sections keep their `jsr
        // abs.l` (codegen-patterns 1.6, 11).
        let here: Vec<&ModuleRoutine> = opts
            .module_routines
            .iter()
            .zip(&homes)
            .filter(|(_, h)| h.as_deref() == Some(sec))
            .map(|(r, _)| r)
            .collect();
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
            // For the driver's internal-error backstop
            // (`diag::set_context`): the routine, up to its pool cells.
            let outer = dpc_syntax::diag::set_context(Some(f.span));
            match isel::select_in_unit(f, opts.schedule, &overlays, opts.merge_constant_stores) {
                Ok(mut i) => {
                    for insn in i.iter_mut() {
                        if let Insn::Jsr(crate::insn::Ea::AbsL(sym, 0)) = insn {
                            // The IR's symbol (upper-cased), not the
                            // option's spelling.
                            if here.iter().any(|r| r.symbol.eq_ignore_ascii_case(sym)) {
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
            dpc_syntax::diag::set_context(outer);
        }
        // Branch sizes need every routine of the section laid out.
        let base = frag.map_or(0, |f| f.origin % 4);
        // The outside routines of this section in the layout's coordinates
        // (the section starts at `origin & !3`).
        let outside: Vec<(String, i64)> = here
            .iter()
            .map(|r| {
                let o = frag.map_or(0, |f| f.origin) & !3;
                (r.symbol.clone(), r.addr as i64 - o as i64)
            })
            .collect();
        pool_labels = pool.next;
        if !failed {
            if let Err(d) = size_branches(&mut routines, base, &outside, &mut pool_labels) {
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
        if frag.is_none_or(|f| f.ends_module) {
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
        if !opts
            .data_bases
            .iter()
            .any(|(s, _)| s.eq_ignore_ascii_case(&data.section))
        {
            out.push_str(&data_area(&module.name, data));
        }
    }
    for (sect, sym) in &opts.data_bases {
        let size = module
            .data
            .iter()
            .find(|d| d.section.eq_ignore_ascii_case(sect))
            .map_or(0, |d| d.size);
        out.push_str(&external_data_area(sect, sym, size));
    }
    for ov in &module.overlays {
        out.push_str(&overlay_area(ov));
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

/// The name of the absolute local symbol that records the fragment origin
/// of output section `section` in a unit with several code sections:
/// [`FRAGMENT_ORIGIN_SYMBOL`] followed by the section's name
/// (`__dpc_fragment_origin.text.TTY.WIRED_PROC`).  A unit with one code
/// section has [`FRAGMENT_ORIGIN_SYMBOL`] alone.
pub fn fragment_origin_symbol(section: &str) -> String {
    format!("{FRAGMENT_ORIGIN_SYMBOL}{section}")
}

/// Match [`Options::fragment`] to the unit's code sections `sections`:
/// each code section with its entry, in `sections`' order; empty for a
/// whole module.  An entry without a section is the only one and is valid
/// only when the unit has at most one code section.  Every named entry
/// must be a code section of the unit, and every code section must have
/// an entry: where a section without one would start is not known.
fn fragment_sections<'a>(
    sections: &[&'a str],
    opts: &'a Options,
) -> Result<Vec<(&'a str, &'a FragmentSection)>, Diagnostic> {
    let err = |m: String| Err(Diagnostic::error_nopos(format!("--fragment-origin: {m}")));
    if opts.fragment.is_empty() {
        return Ok(Vec::new());
    }
    if let Some(f) = opts.fragment.iter().find(|f| f.section.is_none()) {
        if opts.fragment.len() > 1 {
            return err("an origin without a code section is the only one".into());
        }
        return match sections {
            [] => Ok(Vec::new()),
            [s] => Ok(vec![(*s, f)]),
            _ => err(format!(
                "the module has {} code sections; give each one's origin with \
                 --fragment-origin=SECTION=0xADDR",
                sections.len()
            )),
        };
    }
    let mut out = Vec::new();
    for (i, f) in opts.fragment.iter().enumerate() {
        let name = f.section.as_deref().unwrap_or_default();
        if opts.fragment[..i].iter().any(|g| {
            g.section
                .as_deref()
                .is_some_and(|g| g.eq_ignore_ascii_case(name))
        }) {
            return err(format!("code section `{name}` given more than once"));
        }
        if !sections.iter().any(|s| s.eq_ignore_ascii_case(name)) {
            return err(format!(
                "no routine of this unit is in code section `{name}`"
            ));
        }
    }
    for s in sections {
        match opts.fragment.iter().find(|f| {
            f.section
                .as_deref()
                .is_some_and(|n| n.eq_ignore_ascii_case(s))
        }) {
            Some(f) => out.push((*s, f)),
            None => {
                return err(format!(
                    "code section `{s}` has no origin; a fragment gives one for each of its \
                     code sections"
                ))
            }
        }
    }
    Ok(out)
}

/// For each of [`Options::module_routines`]: the unit's code section it
/// lives in (one of `sections`), or `None` when the unit has no routine
/// in its section.  A routine without a section is in the unit's only
/// code section; with several, its section must be given.
fn module_routine_sections<'a>(
    sections: &[&'a str],
    opts: &Options,
) -> Result<Vec<Option<&'a str>>, Diagnostic> {
    opts.module_routines
        .iter()
        .map(|r| match &r.section {
            Some(n) => Ok(sections.iter().find(|s| s.eq_ignore_ascii_case(n)).copied()),
            None => match sections {
                [] => Ok(None),
                [s] => Ok(Some(*s)),
                _ => Err(Diagnostic::error_nopos(format!(
                    "--module-routine: `{}`: the unit has {} code sections; give the \
                     routine's (--module-routine=SECTION:NAME=0xADDR)",
                    r.symbol,
                    sections.len()
                ))),
            },
        })
        .collect()
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
    next_cell: &mut u32,
) -> Result<(), Diagnostic> {
    use std::collections::HashMap;
    enum Target {
        Routine(String),
        Label(u32),
    }
    // Copies of constant cells made for routines out of the first cell's
    // reach: (routine, original cell) -> the copy's label; and each
    // copy's original.
    let mut copies: HashMap<(usize, u32), u32> = HashMap::new();
    let mut origin: HashMap<u32, u32> = HashMap::new();
    'layout: loop {
        let mut at = HashMap::new();
        // (routine, label) -> position
        let mut labels = HashMap::new();
        let mut sites = Vec::new();
        // `case` table entries: (routine, table label, target label).
        let mut entries = Vec::new();
        // Constant cells: label -> (routine, index, position); and the
        // `(d,PC)` references to them: (routine, index, extension word's
        // position, label).
        let mut cells = HashMap::new();
        let mut cell_refs = Vec::new();
        let mut pos = base;
        for (ri, (f, insns)) in routines.iter().enumerate() {
            at.insert(f.symbol.clone(), pos);
            for (ii, i) in insns.iter().enumerate() {
                if let Some(n) = pool_ref(i) {
                    cell_refs.push((ri, ii, pos + 2, n));
                }
                match i {
                    Insn::PoolLabel(n) => {
                        cells.insert(*n, (ri, ii, pos));
                    }
                    Insn::Bsr { target, short } => {
                        sites.push((ri, ii, pos, Target::Routine(target.clone()), *short))
                    }
                    Insn::Branch { label, short, .. } => {
                        sites.push((ri, ii, pos, Target::Label(*label), *short))
                    }
                    // A `dbf`'s displacement is 16 bits, never short.
                    Insn::Dbf { label, .. } => {
                        sites.push((ri, ii, pos, Target::Label(*label), false))
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
                // Out of a 16-bit displacement's reach (bead
                // domain-pascal-pxov).  A call: `jsr abs.l`, as DIR's own
                // calls of its module-local routines more than 32K away
                // (DIR_$CLEANUP 0xE535FE `jsr 0xe4b340`, dir_$map_page;
                // DIR's .TEXT segment is 0x8FD0 bytes).  The distance is
                // the unit's own layout here; the image's may differ, so
                // which calls take `jsr` follows dpc's layout.  A
                // conditional branch (no site: a routine of more than
                // 32K): inverted around `jmp abs.l` to its label, best
                // effort.  A `dbf` (a loop body of more than 32K):
                // `dbf Dn,1f; bra.s 2f; 1: jmp (head).l; 2:`, which keeps
                // the flags as `dbf` does (gas would otherwise lengthen
                // it unseen, and the layout here would drift).
                let fresh = |insns: &[Insn]| {
                    insns
                        .iter()
                        .filter_map(|i| match i {
                            Insn::Label(n) => Some(*n),
                            _ => None,
                        })
                        .max()
                        .map_or(1, |n| n + 1)
                };
                let insns = &mut routines[ri].1;
                match insns[ii].clone() {
                    Insn::Dbf { reg, label, until } => {
                        dpc_syntax::diag::best_effort(Diagnostic::error(
                            routines[ri].0.span,
                            "a `dbf` beyond the reach of a 16-bit displacement (to a `jmp` after \
                             it: no site) is not supported yet",
                        ))?;
                        let insns = &mut routines[ri].1;
                        let (t1, t2) = (fresh(insns), fresh(insns) + 1);
                        let with = vec![
                            Insn::DbfFwd {
                                reg,
                                label: t1,
                                until,
                            },
                            Insn::Branch {
                                cond: crate::insn::Cond::T,
                                label: t2,
                                short: true,
                                back: false,
                            },
                            Insn::Label(t1),
                            Insn::Jmp(crate::insn::Ea::AbsL(format!("{label}b"), 0)),
                            Insn::Label(t2),
                        ];
                        insns.splice(ii..=ii, with);
                    }
                    Insn::Bsr { target, .. } => {
                        insns[ii] = Insn::Jsr(crate::insn::Ea::AbsL(target, 0));
                    }
                    Insn::Branch {
                        cond, label, back, ..
                    } => {
                        dpc_syntax::diag::best_effort(Diagnostic::error(
                            routines[ri].0.span,
                            "a branch beyond the reach of a 16-bit displacement (inverted \
                             around a `jmp`: no site) is not supported yet",
                        ))?;
                        let insns = &mut routines[ri].1;
                        let target = crate::insn::Ea::AbsL(
                            format!("{label}{}", if back { "b" } else { "f" }),
                            0,
                        );
                        let mut with = Vec::new();
                        let skip = fresh(insns);
                        if cond != crate::insn::Cond::T {
                            with.push(Insn::Branch {
                                cond: cond.negate(),
                                label: skip,
                                short: true,
                                back: false,
                            });
                        }
                        with.push(Insn::Jmp(target));
                        if cond != crate::insn::Cond::T {
                            with.push(Insn::Label(skip));
                        }
                        insns.splice(ii..=ii, with);
                    }
                    _ => unreachable!(),
                }
                // The layout is redone: positions and indices moved.
                continue 'layout;
            }
        }
        // A constant cell out of a 16-bit displacement's reach (a unit of
        // more than 32K, DIR's .TEXT, 0x8FD0 bytes): a copy of the cell
        // after the referring routine (best effort, not byte-identical;
        // bead domain-pascal-pxov).  The image keeps its references in
        // reach in its own layout (DIR_$SET_ACL's 0xE52D00 `pea
        // (0xe4b33c,PC)`, displacement -0x79c6); dpc's layout differs.
        // A copy is made once per routine and original cell; a reference
        // its routine's copy does not reach either (a routine of more
        // than 32K) is refused.
        if !changed {
            for &(ri, ii, at_ext, n) in &cell_refs {
                let Some(&(ci, cii, cpos)) = cells.get(&n) else {
                    continue;
                };
                let d = cpos as i64 - at_ext as i64;
                if (-0x8000..0x8000).contains(&d) {
                    continue;
                }
                dpc_syntax::diag::best_effort(Diagnostic::error(
                    routines[ri].0.span,
                    "a constant cell beyond the reach of a 16-bit displacement (copied after \
                     the routine: not seen) is not supported yet",
                ))?;
                let orig = origin.get(&n).copied().unwrap_or(n);
                let m = match copies.get(&(ri, orig)) {
                    Some(&m) if m == n => {
                        return Err(Diagnostic::error(
                            routines[ri].0.span,
                            "a constant cell that no copy after its routine reaches (a routine \
                             of more than 32K) is not supported yet",
                        ))
                    }
                    Some(&m) => m,
                    None => {
                        let Some(Insn::Words(w)) = routines[ci].1.get(cii + 1).cloned() else {
                            unreachable!("a constant cell's words follow its label")
                        };
                        let m = *next_cell;
                        *next_cell += 1;
                        routines[ri].1.push(Insn::PoolLabel(m));
                        routines[ri].1.push(Insn::Words(w));
                        copies.insert((ri, orig), m);
                        origin.insert(m, orig);
                        m
                    }
                };
                set_pool_ref(&mut routines[ri].1[ii], m);
                continue 'layout;
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

/// The constant cell an instruction reads by `(d,PC)` ([`Pool::place`]).
fn pool_ref(i: &Insn) -> Option<u32> {
    match i {
        Insn::Pea(crate::insn::Ea::PoolLabel(n))
        | Insn::Lea {
            src: crate::insn::Ea::PoolLabel(n),
            ..
        }
        | Insn::Move {
            src: crate::insn::Ea::PoolLabel(n),
            ..
        }
        | Insn::Movea {
            src: crate::insn::Ea::PoolLabel(n),
            ..
        } => Some(*n),
        _ => None,
    }
}

/// Point an instruction's `(d,PC)` operand at cell `m` ([`pool_ref`]).
fn set_pool_ref(i: &mut Insn, m: u32) {
    match i {
        Insn::Pea(ea)
        | Insn::Lea { src: ea, .. }
        | Insn::Move { src: ea, .. }
        | Insn::Movea { src: ea, .. } => *ea = crate::insn::Ea::PoolLabel(m),
        _ => unreachable!("a constant cell's reference"),
    }
}

/// The output section of unit `unit`'s static data area in Pascal data
/// section `sect`: `.bss.moddata.<UNIT>` (NOBITS) for the default
/// section, `.bss.moddata.<UNIT>.<SECT>` for another, upper-cased like
/// every external name (codegen-patterns 11); for the kernel's units
/// `<UNIT>` is the map's `D` segment name (`PROC1_`, `PROC2`) and
/// `<SECT>` its section (WIRED_DATA, OS_INIT_DATA).  A block with initial
/// values (`initialised`) is `.moddata.<UNIT>[.<SECT>]`, with contents.
///
/// Only the prefixes are `../source`'s: there a block is the input
/// section `.bss.moddata.<name>` (MODULE_DATA_DEFINE, zero-filled) or
/// `.moddata.<name>` (MODULE_DATA_DEFINE_INIT, "block with the image's
/// initial contents") where `<name>` is the C object of the site
/// (`PROC1_$DATA`, `PROC2_$UNWIRED_DATA`;
/// `../source/domain_os/arch/m68k/arch.h`), and `tools/gen_layout_ld.py`
/// orders a block by the address literal of that C site, never by the
/// map's segment name.  A section of ours would get no ordering key
/// there.  To link a unit into `../source`, the C object owns the bytes
/// and the unit refers to it ([`Options::data_bases`], `dpc
/// --data-base`; `tests/link/README.md`).
pub fn data_section_name(unit: &str, sect: &str, initialised: bool) -> String {
    let prefix = if initialised {
        ".moddata"
    } else {
        ".bss.moddata"
    };
    if sect == DEFAULT_DATA_SECTION {
        format!("{prefix}.{}", unit.to_uppercase())
    } else {
        format!("{prefix}.{}.{}", unit.to_uppercase(), sect.to_uppercase())
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

/// The unit's data area in one data section: one block starting at its
/// [`data_base_label`], `define`d variables as global symbols at their
/// offsets.  Zero-filled and NOBITS when no variable of the section has
/// an initial value; otherwise the whole block has contents, zeros
/// wherever no initial value is given (the image's `D` segments are one
/// block per module and data section, codegen-patterns 11), integers
/// big-endian (m68k).
///
/// The block is 4-aligned and its size a multiple of 4: all 161 `D`
/// segments of the SAU2 map start at a multiple of 4 and have such a
/// size (PROC1_ 0xE254E8, size 0xCC4).  Whether the compiler or the
/// binder rounded is not known; the rounding bytes are zero (TTY's
/// OS_INIT_DATA 0xE351B0, `00 00` at +0x3A after its last datum).
fn data_area(unit: &str, data: &dpc_ir::DataArea) -> String {
    let initialised = !data.init.is_empty();
    let mut out = format!(
        "\n\t.section\t\"{}\",\"aw\",{}\n\t.balign\t4\n{}:\n",
        data_section_name(unit, &data.section, initialised),
        if initialised { "@progbits" } else { "@nobits" },
        data_base_label(&data.section)
    );
    let size = data.size.div_ceil(4) * 4;
    let (bytes, addrs) = data_bytes(data, size);
    // Labels of `define`d variables and addresses, in offset order (a
    // label before the address at its offset).
    let mut marks: Vec<(u32, Option<&str>, Option<&dpc_ir::InitAddr>)> = data
        .globals
        .iter()
        .map(|(sym, off)| (*off, Some(sym.as_str()), None))
        .chain(addrs.iter().map(|(off, a)| (*off, None, Some(*a))))
        .collect();
    marks.sort_by_key(|&(off, _, a)| (off, a.is_some()));
    let mut at = 0;
    for (off, sym, addr) in marks {
        if off > at {
            out.push_str(&data_run(&bytes[at as usize..off as usize]));
            at = off;
        }
        if let Some(sym) = sym {
            out.push_str(&format!("\t.globl\t{sym}\n{sym}:\n"));
        }
        if let Some(a) = addr {
            out.push_str(&format!("\t.long\t{}\n", init_addr_operand(a)));
            at = off + 4;
        }
    }
    if size > at {
        out.push_str(&data_run(&bytes[at as usize..size as usize]));
    }
    out
}

/// An overlay section the unit declares (ref 3.14, p. 3-52; 7.2.4,
/// Method 4, p. 7-7): one object, [`dpc_ir::overlay_symbol`], that every
/// unit declaring the section shares, the units' variables at their
/// offsets in it.  The object file's form of a FORTRAN named common
/// block, which 3.14 likens it to: a common symbol, `.comm SYM,size,4`,
/// which the linker merges with every other unit's of the same name,
/// taking the largest size.  A unit that gives the section initial
/// values defines the symbol instead, in the section
/// `.data.overlay.<SYM>` with its contents (zeros where no initial value
/// is given), and the other units' common symbols resolve to it (with
/// the definition's size, even when a common block is larger: GNU ld
/// says nothing when the definition comes first in link order, "size of
/// symbol ... changed" when a larger common comes first, and "common of
/// ... overridden by definition" either way only with `--warn-common`);
/// two units that both do are a link error ("multiple
/// definition"): which unit's values the Domain binder kept is not
/// known (docs/reference-clarifications.md, question 43).
///
/// 4-aligned and a multiple of 4 bytes, as every overlay section of the
/// SAU2 map (`tools/sectsurvey.py overlays`: PCHIST_UNWIRED_D 0xE85718,
/// size 0x938, the end of its last variable; XPD_$DATA 0xEA5034, 0x4E8;
/// five start at 4 mod 8).  The code never names a section of it
/// (codegen-patterns 11, "Overlay sections"); for a conformance link
/// the symbol is the map section's address (`tools/conform.py`).
///
/// A unit that `define`s a variable in the section (ref 3.14, p. 3-52:
/// a `define` of an `extern` variable of a `var (name)` part "will be
/// stored in section_name") defines the symbol too, with each such
/// variable's symbol a global label at its offset: `.data.overlay.<SYM>`
/// with initial values, else the NOBITS `.bss.overlay.<SYM>` (zeros).
/// An object file cannot define a symbol inside a common block (gas:
/// "can't be equated to common symbol").  Best effort (not
/// byte-identical): no map symbol lies inside the image's overlay
/// sections past their start, and which unit defined NET_PORT_TABLE,
/// whose start is ROUTE_$PORT's map address, is not known
/// (codegen-patterns 11, "Overlay sections"; bead domain-pascal-m15q).
/// No `best_effort` warning: the code's bytes are the same in either
/// form, only the object file's container of the section differs.
fn overlay_area(data: &dpc_ir::DataArea) -> String {
    let sym = dpc_ir::overlay_symbol(&data.section);
    let size = data.size.div_ceil(4) * 4;
    if data.init.is_empty() && data.globals.is_empty() {
        return format!("\n\t.comm\t{sym},{size},4\n");
    }
    let (prefix, kind) = if data.init.is_empty() {
        (".bss", "@nobits")
    } else {
        (".data", "@progbits")
    };
    // `.size`: GNU ld compares it with a larger common block of another
    // unit only when that common comes first in link order ("size of
    // symbol ... changed"); the definition's size wins either way.
    let mut out = format!(
        "\n\t.section\t\"{prefix}.overlay.{sym}\",\"aw\",{kind}\n\t.balign\t4\n\
         \t.globl\t{sym}\n\t.type\t{sym},@object\n\t.size\t{sym},{size}\n{sym}:\n"
    );
    let (bytes, addrs) = data_bytes(data, size);
    // Labels of `define`d variables and addresses, in offset order (a
    // label before the address at its offset), as in [`data_area`].
    let mut marks: Vec<(u32, Option<&str>, Option<&dpc_ir::InitAddr>)> = data
        .globals
        .iter()
        .map(|(s, off)| (*off, Some(s.as_str()), None))
        .chain(addrs.iter().map(|(off, a)| (*off, None, Some(*a))))
        .collect();
    marks.sort_by_key(|&(off, _, a)| (off, a.is_some()));
    let mut at = 0;
    for (off, label, addr) in marks {
        if off > at {
            out.push_str(&data_run(&bytes[at as usize..off as usize]));
            at = off;
        }
        if let Some(label) = label {
            out.push_str(&format!("\t.globl\t{label}\n{label}:\n"));
        }
        if let Some(a) = addr {
            out.push_str(&format!("\t.long\t{}\n", init_addr_operand(a)));
            at = off + 4;
        }
    }
    if size > at {
        out.push_str(&data_run(&bytes[at as usize..size as usize]));
    }
    out
}

/// The assembler operand of an address initial value: the symbol, or the
/// data area's [`data_base_label`], plus the offset.  The assembler
/// leaves a 32-bit absolute relocation against it.
fn init_addr_operand(a: &dpc_ir::InitAddr) -> String {
    let (base, offset) = match a {
        dpc_ir::InitAddr::Symbol { name, offset } => (name.clone(), *offset),
        dpc_ir::InitAddr::Static { section, offset } => (data_base_label(section), *offset),
    };
    match offset {
        0 => base,
        o if o > 0 => format!("{base}+{o}"),
        o => format!("{base}-{}", -i64::from(o)),
    }
}

/// The `size` bytes of a data block: zeros, then each initial value
/// written in order (a later one overwrites an earlier one).  Integers
/// are big-endian.  Addresses are left zero in the bytes and returned
/// by offset (4 bytes each); the IR has refused a later value that
/// overwrites only part of one, and one that overwrites all of it drops
/// it.
fn data_bytes(data: &dpc_ir::DataArea, size: u32) -> (Vec<u8>, Vec<(u32, &dpc_ir::InitAddr)>) {
    let mut bytes = vec![0u8; size as usize];
    let mut addrs: Vec<(u32, &dpc_ir::InitAddr)> = Vec::new();
    for init in &data.init {
        let at = init.offset as usize;
        let n = init.value.size();
        addrs.retain(|&(o, _)| o + 4 <= init.offset || o >= init.offset + n);
        match &init.value {
            dpc_ir::InitValue::Int { size, value } => {
                let n = *size as usize;
                let be = value.to_be_bytes();
                bytes[at..at + n].copy_from_slice(&be[8 - n..]);
            }
            dpc_ir::InitValue::Bytes(b) => bytes[at..at + b.len()].copy_from_slice(b),
            dpc_ir::InitValue::Address(a) => {
                bytes[at..at + 4].fill(0);
                addrs.push((init.offset, a));
            }
            dpc_ir::InitValue::Bits { bit, width, value } => {
                // Bit `p` of the field's string is bit `7 - p % 8` of
                // byte `p / 8`; the value's most significant bit first.
                for i in 0..*width {
                    let p = (bit + i) as usize;
                    let m = 0x80u8 >> (p % 8);
                    if (value >> (width - 1 - i)) & 1 != 0 {
                        bytes[at + p / 8] |= m;
                    } else {
                        bytes[at + p / 8] &= !m;
                    }
                }
            }
        }
    }
    (bytes, addrs)
}

/// Directives for a run of data bytes: `.space n` for zeros (also what
/// a NOBITS block is made of), `.byte` lines of up to 16 otherwise; a
/// zero run inside the contents stays in the `.byte` lines unless it is
/// 16 bytes or more.
fn data_run(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let zeros = bytes[i..].iter().take_while(|&&b| b == 0).count();
        if zeros >= 16 || i + zeros == bytes.len() {
            out.push_str(&format!("\t.space\t{zeros}\n"));
            i += zeros;
            continue;
        }
        // up to 16 bytes, stopping before a long zero run or the
        // trailing zeros
        let mut j = i;
        while j < bytes.len() && j - i < 16 {
            let z = bytes[j..].iter().take_while(|&&b| b == 0).count();
            if z > 0 && (z >= 16 || j + z == bytes.len()) {
                break;
            }
            j += 1;
        }
        let row: Vec<String> = bytes[i..j].iter().map(|b| format!("0x{b:02x}")).collect();
        out.push_str(&format!("\t.byte\t{}\n", row.join(",")));
        i = j;
    }
    out
}

/// The absolute local symbol that records, under `--data-base`, how many
/// bytes the unit declares in Pascal data section `sect` (raw, not
/// rounded): `__dpc_data_size` for the default section,
/// `__dpc_data_size.<SECT>` for another.
pub fn data_size_symbol(sect: &str) -> String {
    if sect == DEFAULT_DATA_SECTION {
        "__dpc_data_size".into()
    } else {
        format!("__dpc_data_size.{}", sect.to_uppercase())
    }
}

/// A data area that another object owns ([`Options::data_bases`]): the
/// unit's label for it is an alias of `sym` (gas relocates references to
/// an alias of an undefined symbol against that symbol), and nothing is
/// allocated.
fn external_data_area(sect: &str, sym: &str, size: u32) -> String {
    format!(
        "\n| data section {sect}: owned by {sym} (dpc --data-base)\n\
         \t.set\t{},{sym}\n\t.set\t{},{size}\n",
        data_base_label(sect),
        data_size_symbol(sect)
    )
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
            with_temps: Vec::new(),
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
            fragment: vec![FragmentSection::only(0xE148CA)],
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
            fragment: vec![FragmentSection::only(0xE00EB0)],
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
            fragment: vec![FragmentSection {
                ends_module: true,
                ..FragmentSection::only(0xE148CA)
            }],
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
            fragment: vec![FragmentSection::only(0xE148CA)],
            ..Default::default()
        };
        let e = emit(&module(&[".text", "wired_proc"]), "m.pas", &opts).unwrap_err();
        assert!(e[0].message.contains("give each one's origin"), "{e:?}");
    }

    fn sect(section: &str, origin: u32, ends_module: bool) -> FragmentSection {
        FragmentSection {
            section: Some(section.into()),
            origin,
            ends_module,
        }
    }

    /// A fragment of each of two code sections: each has its own origin
    /// symbol (named after its output section), its own pad, and its own
    /// module-end padding or none.
    #[test]
    fn fragment_of_two_code_sections() {
        let mut m = module(&[".text", "wired_proc"]);
        m.name = "tty".into();
        let opts = Options {
            fragment: vec![
                sect("WIRED_PROC", 0xE1AF0A, true),
                sect(".TEXT", 0xE67618, false),
            ],
            ..Default::default()
        };
        let s = emit(&m, "m.pas", &opts).unwrap();
        assert!(
            s.contains("\t.set\t__dpc_fragment_origin.text,0xe67618\n"),
            "{s}"
        );
        assert!(
            s.contains("\t.set\t__dpc_fragment_origin.text.TTY.WIRED_PROC,0xe1af0a\n"),
            "{s}"
        );
        assert!(!s.contains("\t.set\t__dpc_fragment_origin,"), "{s}");
        let (text, wired) = s
            .split_once("\t.section\t\".text.TTY.WIRED_PROC\",\"ax\",@progbits\n")
            .unwrap();
        // .text at 0 mod 4: no pad, no end padding
        assert!(
            text.ends_with("\t.text\n\t.globl\tP0\nP0:\n\trts\n\n"),
            "{s}"
        );
        // WIRED_PROC at 2 mod 4 and the end of its segment
        assert!(wired.starts_with("\t.space\t2\t|"), "{s}");
        assert!(wired.ends_with(FILLER_END), "{s}");
        // one section only, given by name: the plain symbol
        let one = Options {
            fragment: vec![sect("wired_proc", 0xE1AF0A, false)],
            ..Default::default()
        };
        let s = emit(&module(&["wired_proc"]), "m.pas", &one).unwrap();
        assert!(
            s.contains("\t.set\t__dpc_fragment_origin,0xe1af0a\n"),
            "{s}"
        );
        // every code section needs an origin; every origin a section
        for (fragment, msg) in [
            (
                vec![sect(".text", 0x1000, false)],
                "code section `wired_proc` has no origin",
            ),
            (
                vec![
                    sect(".text", 0x1000, false),
                    sect("wired_proc", 0x2000, false),
                    sect("os_init_proc", 0x3000, false),
                ],
                "no routine of this unit is in code section `os_init_proc`",
            ),
            (
                vec![
                    sect(".text", 0x1000, false),
                    sect("wired_proc", 0x2000, false),
                    sect("WIRED_PROC", 0x2000, false),
                ],
                "given more than once",
            ),
        ] {
            let opts = Options {
                fragment,
                ..Default::default()
            };
            let e = emit(&m, "m.pas", &opts).unwrap_err();
            assert!(e[0].message.contains(msg), "{e:?}");
        }
    }

    #[test]
    fn data_area_is_a_zero_block_after_the_code() {
        let mut m = module(&[".text"]);
        m.name = "proc1_".into();
        m.data = vec![dpc_ir::DataArea {
            section: ".data".into(),
            size: 14,
            globals: vec![("PROC1_$X".into(), 2), ("PROC1_$Y".into(), 8)],
            init: vec![],
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

    /// An overlay section (ref 3.14): a common symbol of the section's
    /// name, its size rounded to 4; with initial values, the symbol
    /// defined in `.data.overlay.<SYM>` with its contents.
    #[test]
    fn overlay_sections_are_common_blocks() {
        let mut m = module(&[".text"]);
        m.overlays = vec![
            dpc_ir::DataArea {
                section: "pchist_unwired_d".into(),
                size: 0x937,
                globals: vec![],
                init: vec![],
            },
            dpc_ir::DataArea {
                section: "sec".into(),
                size: 6,
                globals: vec![],
                init: vec![
                    dpc_ir::DataInit {
                        offset: 4,
                        value: dpc_ir::InitValue::Address(dpc_ir::InitAddr::Symbol {
                            name: "X".into(),
                            offset: 2,
                        }),
                    },
                    dpc_ir::DataInit {
                        offset: 0,
                        value: dpc_ir::InitValue::Int { size: 2, value: 5 },
                    },
                ],
            },
        ];
        let s = emit(&m, "m.pas", &Options::default()).unwrap();
        let tail = s.split("filler\n").nth(1).unwrap();
        assert_eq!(
            tail,
            "\n\t.comm\tPCHIST_UNWIRED_D,2360,4\n\
             \n\t.section\t\".data.overlay.SEC\",\"aw\",@progbits\n\t.balign\t4\n\
             \t.globl\tSEC\n\t.type\tSEC,@object\n\t.size\tSEC,8\nSEC:\n\
             \t.byte\t0x00,0x05\n\t.space\t2\n\t.long\tX+2\n"
        );
    }

    /// An overlay section with `define`d variables (ref 3.14; bead
    /// domain-pascal-m15q): the unit defines the section's symbol, each
    /// variable's a global label at its offset; NOBITS without initial
    /// values, in `.data.overlay.<SYM>` with them.
    #[test]
    fn overlay_section_with_defined_variables() {
        let mut m = module(&[".text"]);
        m.overlays = vec![
            dpc_ir::DataArea {
                section: "net_port_table".into(),
                size: 0x2E0,
                globals: vec![("ROUTE_$PORT".into(), 0), ("B".into(), 0x10)],
                init: vec![],
            },
            dpc_ir::DataArea {
                section: "sec".into(),
                size: 10,
                globals: vec![("A".into(), 4), ("C".into(), 8)],
                init: vec![
                    dpc_ir::DataInit {
                        offset: 4,
                        value: dpc_ir::InitValue::Address(dpc_ir::InitAddr::Symbol {
                            name: "X".into(),
                            offset: 0,
                        }),
                    },
                    dpc_ir::DataInit {
                        offset: 0,
                        value: dpc_ir::InitValue::Int { size: 2, value: 5 },
                    },
                ],
            },
        ];
        let s = emit(&m, "m.pas", &Options::default()).unwrap();
        let tail = s.split("filler\n").nth(1).unwrap();
        assert_eq!(
            tail,
            "\n\t.section\t\".bss.overlay.NET_PORT_TABLE\",\"aw\",@nobits\n\t.balign\t4\n\
             \t.globl\tNET_PORT_TABLE\n\t.type\tNET_PORT_TABLE,@object\n\
             \t.size\tNET_PORT_TABLE,736\nNET_PORT_TABLE:\n\
             \t.globl\tROUTE_$PORT\nROUTE_$PORT:\n\t.space\t16\n\t.globl\tB\nB:\n\t.space\t720\n\
             \n\t.section\t\".data.overlay.SEC\",\"aw\",@progbits\n\t.balign\t4\n\
             \t.globl\tSEC\n\t.type\tSEC,@object\n\t.size\tSEC,12\nSEC:\n\
             \t.byte\t0x00,0x05\n\t.space\t2\n\t.globl\tA\nA:\n\t.long\tX\n\
             \t.globl\tC\nC:\n\t.space\t4\n"
        );
    }

    #[test]
    fn initialised_data_area_has_contents() {
        use dpc_ir::{DataInit, InitValue};
        let mut m = module(&[".text"]);
        m.name = "tty".into();
        let int = |offset, size, value| DataInit {
            offset,
            value: InitValue::Int { size, value },
        };
        m.data = vec![dpc_ir::DataArea {
            section: "wired_data".into(),
            size: 0x2A,
            globals: vec![("TTY_$X".into(), 0x24)],
            init: vec![
                int(0, 4, 0x0012_0010),
                int(4, 2, -2),
                int(6, 1, 0xFF),
                DataInit {
                    offset: 7,
                    value: InitValue::Bytes(vec![1, 0, 2]),
                },
                // overwritten by the later value
                int(0x24, 4, 0x1111_1111),
                int(0x24, 4, 0x3C),
                int(0x28, 2, 0x1A),
            ],
        }];
        let s = emit(&m, "m.pas", &Options::default()).unwrap();
        let tail = s.split("filler\n").nth(1).unwrap();
        assert_eq!(
            tail,
            "\n\t.section\t\".moddata.TTY.WIRED_DATA\",\"aw\",@progbits\n\t.balign\t4\n\
             .Ldata.WIRED_DATA:\n\
             \t.byte\t0x00,0x12,0x00,0x10,0xff,0xfe,0xff,0x01,0x00,0x02\n\
             \t.space\t26\n\
             \t.globl\tTTY_$X\nTTY_$X:\n\
             \t.byte\t0x00,0x00,0x00,0x3c,0x00,0x1a\n\
             \t.space\t2\n"
        );
        // a long zero run inside the contents
        assert_eq!(
            data_run(&[1, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3]),
            "\t.byte\t0x01,0x00,0x00,0x02\n\t.space\t16\n\t.byte\t0x03\n"
        );
    }

    #[test]
    fn bit_field_initial_values_merge_into_bytes() {
        // ref 3.10.7 p. 3-35: bit 0 the most significant of the first
        // byte; a field across a byte boundary; a later value replaces
        // only its own bits.
        use dpc_ir::{DataInit, InitValue};
        let bits = |offset, bit, width, value| DataInit {
            offset,
            value: InitValue::Bits { bit, width, value },
        };
        let area = dpc_ir::DataArea {
            section: "data".into(),
            size: 4,
            globals: vec![],
            init: vec![
                DataInit {
                    offset: 0,
                    value: InitValue::Int {
                        size: 1,
                        value: 0xFB,
                    },
                },
                bits(1, 0, 4, 2),
                bits(1, 4, 4, 0),
                bits(2, 4, 8, 0x9C),
                bits(2, 0, 16, 0xFFFF),
                bits(2, 6, 2, 0),
            ],
        };
        assert_eq!(data_bytes(&area, 4).0, vec![0xFB, 0x20, 0xFC, 0xFF]);
    }

    #[test]
    fn data_base_aliases_an_external_block() {
        let mut m = module(&[".text"]);
        m.name = "proc1_".into();
        m.data = vec![dpc_ir::DataArea {
            section: "wired_data".into(),
            size: 14,
            globals: vec![("PROC1_$X".into(), 2)],
            init: vec![],
        }];
        let with = |bases: &[(&str, &str)]| {
            let opts = Options {
                data_bases: bases
                    .iter()
                    .map(|(a, b)| (a.to_string(), b.to_string()))
                    .collect(),
                ..Default::default()
            };
            emit(&m, "m.pas", &opts)
        };
        let s = with(&[("WIRED_DATA", "PROC1_$DATA")]).unwrap();
        let plain = with(&[]).unwrap();
        // the code is the same; no block, no define'd global
        assert_eq!(s.split("filler\n").next(), plain.split("filler\n").next());
        let tail = s.split("filler\n").nth(1).unwrap();
        assert_eq!(
            tail,
            "\n| data section WIRED_DATA: owned by PROC1_$DATA (dpc --data-base)\n\
             \t.set\t.Ldata.WIRED_DATA,PROC1_$DATA\n\
             \t.set\t__dpc_data_size.WIRED_DATA,14\n"
        );
        assert!(!s.contains("moddata") && !s.contains("PROC1_$X"), "{s}");
        // a section of nothing in the unit, or one given twice
        assert!(with(&[("other_data", "X")]).is_err());
        assert!(with(&[("wired_data", "X"), ("WIRED_DATA", "Y")]).is_err());
        assert_eq!(data_size_symbol(".data"), "__dpc_data_size");
    }

    #[test]
    fn data_base_for_a_routine_section_without_variables() {
        let mut m = module(&[".text"]);
        m.functions[0].data_section = "os_init_data".into();
        let opts = Options {
            data_bases: vec![("os_init_data".into(), "B".into())],
            ..Default::default()
        };
        let s = emit(&m, "m.pas", &opts).unwrap();
        assert!(
            s.ends_with("\t.set\t.Ldata.OS_INIT_DATA,B\n\t.set\t__dpc_data_size.OS_INIT_DATA,0\n"),
            "{s}"
        );
    }

    #[test]
    fn section_directives() {
        assert_eq!(section_directive("proc1_", ".text"), "\t.text");
        assert_eq!(
            section_directive("proc1_", "wired_proc"),
            "\t.section\t\".text.PROC1_.WIRED_PROC\",\"ax\",@progbits"
        );
        assert_eq!(
            data_section_name("proc1_", ".data", false),
            ".bss.moddata.PROC1_"
        );
        assert_eq!(
            data_section_name("proc1_", "wired_data", false),
            ".bss.moddata.PROC1_.WIRED_DATA"
        );
        assert_eq!(data_section_name("tty", ".data", true), ".moddata.TTY");
        assert_eq!(
            data_section_name("tty", "wired_data", true),
            ".moddata.TTY.WIRED_DATA"
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
                    until: Some(test.clone()),
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
                until: Some(Cond::Test(Expr::Load {
                    addr: Addr::Symbol {
                        name: "B".into(),
                        offset: 0,
                    },
                    ty: Scalar::BOOLEAN,
                })),
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
            fragment: vec![FragmentSection {
                ends_module: true,
                ..FragmentSection::only(0x1000)
            }],
            loop_align: LoopAlign::OffFrom(vec!["P_$C".into()]),
            ..Default::default()
        };
        let s = emit(&m, "m.pas", &tail).unwrap();
        assert!(routine_text(&s, "P_$B").contains("0x2048"), "{s}");
        assert!(s.ends_with(ZERO_END), "{s}");
        let mid = Options {
            fragment: vec![FragmentSection::only(0x1000)],
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
                fragment: vec![FragmentSection::only(0x1002)],
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
            fragment: vec![FragmentSection::only(0x1000)],
            module_routines: vec![ModuleRoutine::new("Q", at)],
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
        lower.module_routines[0].symbol = "q".into();
        let s = emit(&m, "m.pas", &lower).unwrap();
        assert!(s.contains(shorts), "{s}");
        // without the option: `jsr`
        let s = emit(&m, "m.pas", &Options::default()).unwrap();
        assert!(s.contains("\tjsr\t(Q).l\n"), "{s}");
        let mut opts = frag(0x1100);
        opts.module_routines[0].symbol = "p".into();
        let e = emit(&m, "m.pas", &opts).unwrap_err();
        assert!(e[0].message.contains("has its body in this unit"), "{e:?}");
    }
    /// Module routines in a unit of several code sections (TTY): a call
    /// from the routine's own code section is a `bsr`, sized from its
    /// address and that section's origin; one from another section, or
    /// to a routine of a section the unit has no routine in, stays `jsr
    /// abs.l` (codegen-patterns 1.6, 11: TTY_$K_RESET 0xE6730E `jsr` to
    /// the WIRED_PROC module-local 0xE1AED0).
    #[test]
    fn module_routines_per_code_section() {
        use dpc_ir::{Call, Callee, Stmt, StmtKind};
        let calling = |section: &str, symbol: &str, callees: &[&str]| {
            let mut f = one_routine(section, symbol);
            f.body = callees
                .iter()
                .map(|c| Stmt {
                    kind: StmtKind::Call(Call {
                        callee: Callee::External((*c).into()),
                        args: Vec::new(),
                        pad: 0,
                        link: None,
                    }),
                    span: f.span,
                })
                .collect();
            f
        };
        let m = Module {
            name: "tty".into(),
            functions: vec![
                calling("wired_proc", "W", &["LOCK", "GET_DESC", "INIT"]),
                calling(".text", "T", &["LOCK", "GET_DESC", "INIT"]),
            ],
            ..Default::default()
        };
        let routine = |section: &str, symbol: &str, addr| ModuleRoutine {
            section: Some(section.into()),
            symbol: symbol.into(),
            addr,
        };
        let opts = Options {
            fragment: vec![
                sect("WIRED_PROC", 0xE1AF0A, false),
                sect(".text", 0xE67618, false),
            ],
            module_routines: vec![
                routine("WIRED_PROC", "LOCK", 0xE1AED0),
                routine(".TEXT", "GET_DESC", 0xE66738),
                routine("OS_INIT_PROC", "INIT", 0xE3324C),
            ],
            ..Default::default()
        };
        let s = emit(&m, "m.pas", &opts).unwrap();
        // W at 0xE1AF0A: `bsr.s LOCK` at 0xE1AF0E, 0x40 bytes back,
        // written as its encoding; GET_DESC (.text) and INIT (a section
        // the unit does not have) by `jsr`.
        let w = routine_text(&s, "W");
        assert!(w.contains("\t.short\t0x61c0\t| bsr.s LOCK\n"), "{w}");
        assert!(w.contains("\tjsr\t(GET_DESC).l\n"), "{w}");
        assert!(w.contains("\tjsr\t(INIT).l\n"), "{w}");
        // T at 0xE67618: GET_DESC is 0xEE0 back, `bsr.w`; LOCK by `jsr`.
        let t = routine_text(&s, "T");
        assert!(t.contains("\tbsr.w\tGET_DESC\n"), "{t}");
        assert!(t.contains("\tjsr\t(LOCK).l\n"), "{t}");
        assert!(t.contains("\tjsr\t(INIT).l\n"), "{t}");
        // without a section, a module routine of a unit of several code
        // sections is an error
        let mut plain = opts.clone();
        plain.module_routines[0].section = None;
        let e = emit(&m, "m.pas", &plain).unwrap_err();
        assert!(e[0].message.contains("give the routine's"), "{e:?}");
        // with one code section it is that section's
        let one = Module {
            name: "tty".into(),
            functions: vec![calling("wired_proc", "W", &["LOCK", "GET_DESC"])],
            ..Default::default()
        };
        let opts = Options {
            fragment: vec![FragmentSection::only(0xE1AF0A)],
            module_routines: vec![
                ModuleRoutine::new("LOCK", 0xE1AED0),
                routine(".text", "GET_DESC", 0xE66738),
            ],
            ..Default::default()
        };
        let s = emit(&one, "m.pas", &opts).unwrap();
        assert!(s.contains("\t.short\t0x61c0\t| bsr.s LOCK\n"), "{s}");
        assert!(s.contains("\tjsr\t(GET_DESC).l\n"), "{s}");
    }
}
