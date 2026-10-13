//! Assembly emitter: gas syntax for `m68k-elf-as -m68010` (CLAUDE.md 4.4).
//!
//! One output section per Pascal code section, in order of first use.
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
    /// The routines before the named one, in emission order, align; it
    /// and every later one do not.  The name is matched case-insensitively
    /// against each routine's symbol, which is its Pascal name upper-cased
    /// (codegen-patterns 11), so either spelling works.
    OffFrom(String),
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
    /// fragment is linked.
    pub module_routines: Vec<(String, u32)>,
}

/// For each routine of `module` (indexed as `module.functions`): does it
/// align its loop heads?  "Before" in [`LoopAlign::OffFrom`] is emission
/// order: code section by code section, in `sections` order, and the
/// unit's order within each.  Fails if [`LoopAlign::OffFrom`] names no
/// routine of the module.
fn aligning(module: &Module, sections: &[&str], la: &LoopAlign) -> Result<Vec<bool>, Diagnostic> {
    let n = module.functions.len();
    match la {
        LoopAlign::On => Ok(vec![true; n]),
        LoopAlign::Off => Ok(vec![false; n]),
        LoopAlign::OffFrom(name) => {
            let order = sections
                .iter()
                .flat_map(|&sec| (0..n).filter(move |&i| module.functions[i].code_section == sec));
            let mut aligns = vec![false; n];
            for i in order {
                if module.functions[i].symbol.eq_ignore_ascii_case(name) {
                    return Ok(aligns);
                }
                aligns[i] = true;
            }
            Err(Diagnostic::error_nopos(format!(
                "--loop-align-off-from: no routine `{name}` in this unit"
            )))
        }
    }
}

/// The section directive for Pascal code section `name`.
fn section_directive(name: &str) -> String {
    if name == ".text" {
        // ref 5.7.1 p. 5-24: the default code section.
        "\t.text".into()
    } else {
        // Named sections are rejected by IR lowering for now: how they map
        // to `../source`'s `.text.<name>` link sections is not settled
        // (codegen-patterns 11).
        format!("\t.section\t\"{name}\",\"ax\",@progbits")
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
    let aligns = aligning(module, &sections, &opts.loop_align).map_err(|d| vec![d])?;
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
    for sec in sections {
        out.push('\n');
        out.push_str(&section_directive(sec));
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
        for (f, &al) in module
            .functions
            .iter()
            .zip(&aligns)
            .filter(|(f, _)| f.code_section == sec)
        {
            last_aligns = al;
            match isel::select(f) {
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
        if !failed {
            if let Err(d) = size_branches(&mut routines, base, &outside) {
                errors.push(d);
            }
        }
        for (f, insns) in routines {
            if f.global {
                out.push_str(&format!("\t.globl\t{}\n", f.symbol));
            }
            out.push_str(&format!("{}:\n", f.symbol));
            for i in insns {
                if !matches!(i, Insn::Label(_)) {
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
    if let Some(data) = &module.data {
        out.push_str(&data_area(&module.name, data));
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
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

/// The output section of unit `unit`'s static data area: the NOBITS
/// section `.bss.moddata.<UNIT>`, the unit name upper-cased like every
/// external name (codegen-patterns 11); for the kernel's units that is
/// the map's `D` segment name (`PROC1_`, `PROC2`).
///
/// Only the prefix is `../source`'s: there a block is the input section
/// `.bss.moddata.<name>` / `.moddata.<name>` where `<name>` is the C
/// object of a `MODULE_DATA_DEFINE` site (`PROC1_$DATA`,
/// `PROC2_$UNWIRED_DATA`; `../source/domain_os/arch/m68k/arch.h`), and
/// `tools/gen_layout_ld.py` orders a block by the address literal of that
/// C site, never by the map's segment name.  A section of ours would get
/// no ordering key there; linking a dpc unit into `../source` needs a
/// mapping that does not exist yet (codegen-patterns 11).
pub fn data_section_name(unit: &str) -> String {
    format!(".bss.moddata.{}", unit.to_uppercase())
}

/// The unit's data area: one zero-filled block starting at
/// [`DATA_BASE_LABEL`], `define`d variables as global symbols at their
/// offsets.
///
/// The block is 4-aligned and its size a multiple of 4: all 161 `D`
/// segments of the SAU2 map start at a multiple of 4 and have such a
/// size (PROC1_ 0xE254E8, size 0xCC4).  Whether the compiler or the
/// binder rounded is not known.
fn data_area(unit: &str, data: &dpc_ir::DataArea) -> String {
    let mut out = format!(
        "\n\t.section\t\"{}\",\"aw\",@nobits\n\t.balign\t4\n{DATA_BASE_LABEL}:\n",
        data_section_name(unit)
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
            frame_size: 0,
            result: None,
            loads_data_base: false,
            data_absolute: false,
            value_params: Vec::new(),
            address_params: Vec::new(),
            scalar_locals: Vec::new(),
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
        m.data = Some(dpc_ir::DataArea {
            size: 14,
            globals: vec![("PROC1_$X".into(), 2), ("PROC1_$Y".into(), 8)],
        });
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
        assert_eq!(section_directive(".text"), "\t.text");
        assert_eq!(
            section_directive("wired_proc"),
            "\t.section\t\"wired_proc\",\"ax\",@progbits"
        );
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
                })
            })
            .collect();
        if let Some(l) = local {
            body.push(StmtKind::Call(Call {
                callee: Callee::Local(l.into()),
                args: vec![],
                pad: 0,
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
        let s = emit(&m, "m.pas", &loop_align(LoopAlign::OffFrom("p_$b".into()))).unwrap();
        assert!(routine_text(&s, "P_$A").contains("0x2048"), "{s}");
        assert!(!routine_text(&s, "P_$B").contains("0x2048"), "{s}");
        assert!(!routine_text(&s, "P_$C").contains("0x2048"), "{s}");
        assert!(s.ends_with(ZERO_END), "{s}");
        // from the first routine: the same as off
        let s = emit(&m, "m.pas", &loop_align(LoopAlign::OffFrom("P_$A".into()))).unwrap();
        assert_eq!(s, emit(&m, "m.pas", &loop_align(LoopAlign::Off)).unwrap());
        // a fragment that ends its module: the padding follows the last
        // routine too; one that does not end it has none
        let tail = Options {
            fragment_origin: Some(0x1000),
            fragment_ends_module: true,
            loop_align: LoopAlign::OffFrom("P_$C".into()),
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
        let e = emit(&m, "m.pas", &loop_align(LoopAlign::OffFrom("P_$D".into()))).unwrap_err();
        assert!(e[0].message.contains("no routine `P_$D`"), "{e:?}");
    }

    /// `--loop-align-off-from` counts in emission order, where a whole
    /// module's routines are grouped by code section: with `A` (.text),
    /// `B` (S2), `C` (.text), off from `B`, `C` is emitted before `B` and
    /// aligns.  Each section's end padding follows its own last routine.
    #[test]
    fn loop_align_off_from_follows_emission_order() {
        let mut b = one_loop("B");
        b.code_section = "S2".into();
        let m = Module {
            functions: vec![one_loop("A"), b, one_loop("C")],
            ..Default::default()
        };
        let s = emit(&m, "m.pas", &loop_align(LoopAlign::OffFrom("B".into()))).unwrap();
        assert!(routine_text(&s, "A").contains("0x2048"), "{s}");
        assert!(routine_text(&s, "C").contains("0x2048"), "{s}");
        assert!(!routine_text(&s, "B").contains("0x2048"), "{s}");
        let (text, s2) = s.split_once("\n\t.section\t\"S2\"").unwrap();
        assert!(text.ends_with(FILLER_END), "{s}");
        assert!(s2.ends_with(ZERO_END), "{s}");
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
    #[test]
    fn calls_of_module_routines_outside_a_fragment() {
        use dpc_ir::{Call, Callee, Stmt, StmtKind};
        let mut f = one_routine(".text", "P");
        let call = Stmt {
            kind: StmtKind::Call(Call {
                callee: Callee::External("Q".into()),
                args: Vec::new(),
                pad: 0,
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
        // P: link at 0x1000, the calls at 0x1004 and 0x1006 (short).
        let s = emit(&m, "m.pas", &frag(0x1080)).unwrap();
        assert!(s.contains("\tbsr.s\tQ\n\tbsr.s\tQ\n"), "{s}");
        // 0x1100 is out of `.s` reach from 0x1004.
        let s = emit(&m, "m.pas", &frag(0x1100)).unwrap();
        assert!(s.contains("\tbsr.w\tQ\n\tbsr.w\tQ\n"), "{s}");
        // The option's spelling does not matter: the symbol is the IR's.
        let mut lower = frag(0x1080);
        lower.module_routines[0].0 = "q".into();
        let s = emit(&m, "m.pas", &lower).unwrap();
        assert!(s.contains("\tbsr.s\tQ\n\tbsr.s\tQ\n"), "{s}");
        // without the option: `jsr`
        let s = emit(&m, "m.pas", &Options::default()).unwrap();
        assert!(s.contains("\tjsr\t(Q).l\n"), "{s}");
        let mut opts = frag(0x1100);
        opts.module_routines[0].0 = "p".into();
        let e = emit(&m, "m.pas", &opts).unwrap_err();
        assert!(e[0].message.contains("has its body in this unit"), "{e:?}");
    }
}
