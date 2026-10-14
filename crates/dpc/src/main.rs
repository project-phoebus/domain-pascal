//! `dpc`: the Domain Pascal compiler driver.
//!
//! usage: dpc [-I dir]... [--root DIR] [-o out.s] [--fragment-origin 0xADDR [--fragment-ends-module]]
//!            [--fragment-origin=SECTION=0xADDR [--fragment-ends-module=SECTION]]...
//!            [--loop-align=on|off | --loop-align-off-from=ROUTINE,...]
//!            [--a5-always-from=ROUTINE,...] [--a5-always=ROUTINE,...]
//!            [--a5-absolute=ROUTINE,...] [--module-routine=[SECTION:]ROUTINE=0xADDR]...
//!            [--schedule=off|r11|r11:N] [--data-base=SECTION=SYMBOL]...
//!            [--park-d0=ROUTINE,...]
//!            [--parse-only | --dump-ast | --check-only | --dump-locals] file.pas
//!
//! `--parse-only` stops after parsing (syntax errors are reported with
//! their positions); `--dump-ast` also prints the AST to stdout;
//! `--check-only` stops after semantic analysis.  `--dump-locals` lowers
//! every routine that lowers and prints, one JSON object per line, how
//! each body uses its scalar locals (`dpc_m68k::locals`; the input of
//! `tools/localsurvey.py`, codegen-patterns 12.5); routines that do not
//! lower are named on stderr, and the exit status is 0.
//!
//! `--root DIR`: include names beginning with one `/` (Apollo's
//! `%include '/sys/ins/base.ins.pas'`) are looked up under DIR, e.g. a
//! copy of an Apollo installation (`preprocess::Options::root`; used by
//! `tools/corpus.py`).
//!
//! `--fragment-origin 0xADDR` (for image-conformance tests) says that the
//! output is a fragment of a module whose first byte belongs at image
//! address ADDR (even): 4-byte alignment is made absolute and no
//! module-end padding is emitted (`dpc_m68k::emit::Options`).  With
//! `--fragment-ends-module` the fragment is the tail of its module and
//! does get the module-end padding.  A unit with several code sections
//! gives each its own origin, `--fragment-origin=SECTION=0xADDR`
//! (SECTION as the source names it, `.text` for the default; any case),
//! and `--fragment-ends-module=SECTION` for each that ends its module's
//! segment (`dpc_m68k::emit::FragmentSection`).  The plain forms are for
//! a unit with one code section, whatever its name.
//!
//! `--a5-*` choose which routines load the unit's data base, A5
//! (codegen-patterns 4; `dpc_ir::DataBaseOptions`).  By default ("used
//! mode") an exported routine loads it iff it addresses the unit's data,
//! directly or through `internal` callees that inherit it.
//! `--a5-always-from=R,...`: from routine R on, within R's code section
//! (one switch per section, as per map segment), every exported routine
//! with a frame loads it.  `--a5-always=R,...`: these
//! routines (`internal` ones too) load it.  `--a5-absolute=R,...`: these
//! exported routines do not load it and address the data with `lea
//! <area>.l,An`.  Names are matched case-insensitively.
//!
//! `--module-routine=[SECTION:]R=0xADDR` (with `--fragment-origin`;
//! repeatable): R, declared `extern` in the fragment, is a routine of the
//! same module outside the fragment, at image address ADDR, in Pascal code
//! section SECTION (default: the unit's only code section).  The original
//! compiled the whole module and reached it with `bsr` from its own code
//! section (codegen-patterns 1.6: DISK_$UNASSIGN_ALL 0xE6BE34 `bsr.w
//! DISK_$UNASSIGN`) and with `jsr abs.l` from another (codegen-patterns
//! 11); the fragment does too, a `bsr` sized from ADDR
//! (`dpc_m68k::emit::Options::module_routines`).  SECTION need not be one
//! the unit has routines in.
//! A module-local routine the map does not name is declared `internal;
//! extern` and named by its symbol (MST_$MAPS 0xE439C4 `bsr.w` to
//! 0xE43182, the fragment's `ALLOC_SEGS`): the `bsr` goes to that local
//! symbol, which the conformance harness defines at link time (bead
//! domain-pascal-3h9; test scaffolding, `tests/conformance/README.md`).
//! Without the option such a call is an error: internal routines can be
//! called only within the unit that defines them (ref 5.2, p. 5-4).
//!
//! `--schedule=off|r11|r11:N` chooses how instructions are ordered inside
//! basic blocks after register allocation (`dpc_m68k::sched`;
//! codegen-patterns 12.4 R9-R11): `off` keeps the selector's order, `r11`
//! applies the candidate list rule R11 over the whole block, `r11:N` with
//! a window of the next N instructions.  The default is `r11:3`; the
//! policy is not settled (12.4 R11).
//!
//! `--park-d0=R,...`: in these routines a pointer local assigned once from
//! a call and otherwise only dereferenced is held in D0 after the call
//! (`move.l A0,D0`) and moved to an address register by the first
//! statement that refers to it or calls a routine, also where a call
//! follows the assignment (`dpc_m68k::emit::Options::park_d0`;
//! codegen-patterns 3, "Pointer locals held in D0").  Without it only a
//! local that no call follows is (the image does both by a rule not known:
//! 14 routines park where a call follows, TTY_$K_FLUSH_INPUT 0xE1C0A8,
//! against about 105 sites of the same shape that do not; the option is
//! taken from the image, `tools/srcsurvey.py`).  Names are matched
//! case-insensitively.
//!
//! `--data-base=SECTION=SYMBOL` (repeatable; link-test scaffolding, bead
//! domain-pascal-3bp, `tests/link/README.md`): the data area of Pascal
//! data section SECTION (`.data` for the default) belongs to another
//! object, whose global SYMBOL is its first byte.  The unit allocates no
//! block there and does not define its `define`d variables; its routines
//! address the area through SYMBOL (`lea SYMBOL.l,A5`), with the same
//! bytes as without the option (`dpc_m68k::emit::Options::data_bases`).
//! Used to link a unit into `../source`, where a C `MODULE_DATA_DEFINE`
//! block owns the module's data.

use std::path::PathBuf;
use std::process::ExitCode;

use dpc_ir::DataBaseOptions;
use dpc_m68k::emit::{FragmentSection, LoopAlign, ModuleRoutine};
use dpc_m68k::sched::Schedule;
use dpc_syntax::{preprocess, Diagnostic, SourceMap};
use dpc_target::Target;

const PROG: &str = "dpc";
const USAGE: &str = "usage: dpc [-I dir]... [--root DIR] [-o out.s] \
                     [--fragment-origin 0xADDR [--fragment-ends-module]] \
                     [--fragment-origin=SECTION=0xADDR [--fragment-ends-module=SECTION]]... \
                     [--loop-align=on|off | --loop-align-off-from=ROUTINE,...] \
                     [--a5-always-from=ROUTINE,...] [--a5-always=ROUTINE,...] \
                     [--a5-absolute=ROUTINE,...] [--module-routine=[SECTION:]ROUTINE=0xADDR]... \
                     [--schedule=off|r11|r11:N] [--data-base=SECTION=SYMBOL]... \
                     [--park-d0=ROUTINE,...] \
                     [--parse-only | --dump-ast | --check-only | --dump-locals] file.pas";

/// Where to stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stop {
    /// Run the whole pipeline.
    Never,
    /// Stop after parsing.
    AfterParse,
    /// Stop after parsing and print the AST.
    DumpAst,
    /// Stop after semantic analysis.
    AfterCheck,
    /// Lower what lowers and print the uses of scalar locals.
    DumpLocals,
}

#[derive(Debug)]
struct Args {
    include_dirs: Vec<PathBuf>,
    root: Option<PathBuf>,
    output: Option<PathBuf>,
    input: PathBuf,
    stop: Stop,
    fragment: Vec<FragmentSection>,
    loop_align: LoopAlign,
    data_base: DataBaseOptions,
    module_routines: Vec<ModuleRoutine>,
    schedule: Schedule,
    data_bases: Vec<(String, String)>,
    park_d0: Vec<String>,
}

/// The routine names of `--a5-always=`, `--a5-absolute=`,
/// `--a5-always-from=`, `--loop-align-off-from=` and `--park-d0=`: a comma list without
/// empty items.
fn routine_list(opt: &str, v: &str) -> Result<Vec<String>, String> {
    let names: Vec<String> = v.split(',').map(str::to_string).collect();
    if names.iter().any(String::is_empty) {
        return Err(format!(
            "{opt} needs a comma-separated list of routine names"
        ));
    }
    Ok(names)
}

/// A number in C notation: `0x` hex or decimal.
fn parse_number(s: &str) -> Option<u32> {
    match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(h) => u32::from_str_radix(h, 16).ok(),
        None => s.parse().ok(),
    }
}

/// A fragment origin: an even address (code is at even addresses).
fn fragment_address(v: &str) -> Result<u32, String> {
    let a = parse_number(v).ok_or_else(|| format!("--fragment-origin: bad address `{v}`"))?;
    if a % 2 != 0 {
        return Err(format!("--fragment-origin: odd address `{v}`"));
    }
    Ok(a)
}

/// The fragment's code sections from the `--fragment-origin` options
/// `origins` (section, or None for the plain form) and the
/// `--fragment-ends-module` ones `ends`.  The plain forms go together and
/// at most once; each section at most once, every ending section with an
/// origin.  Section names are matched case-insensitively.
fn fragment_sections(
    origins: &[(Option<String>, u32)],
    ends: &[Option<String>],
) -> Result<Vec<FragmentSection>, String> {
    let plain = origins.iter().filter(|(s, _)| s.is_none()).count();
    if plain > 1 {
        return Err("--fragment-origin given more than once".into());
    }
    if plain == 1 && origins.len() > 1 {
        return Err(
            "--fragment-origin 0xADDR and --fragment-origin=SECTION=0xADDR do not mix".into(),
        );
    }
    let mut out: Vec<FragmentSection> = Vec::new();
    for (section, origin) in origins {
        if let Some(n) = section {
            if out.iter().any(|f| {
                f.section
                    .as_deref()
                    .is_some_and(|m| m.eq_ignore_ascii_case(n))
            }) {
                return Err(format!(
                    "--fragment-origin: code section `{n}` given more than once"
                ));
            }
        }
        out.push(FragmentSection {
            section: section.clone(),
            origin: *origin,
            ends_module: false,
        });
    }
    for (i, e) in ends.iter().enumerate() {
        if ends[..i].iter().any(|d| match (d, e) {
            (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
            (None, None) => true,
            _ => false,
        }) {
            return Err("--fragment-ends-module given more than once for one section".into());
        }
        let f = match e {
            None => out.iter_mut().find(|f| f.section.is_none()).ok_or(
                "--fragment-ends-module needs --fragment-origin 0xADDR (with \
                 --fragment-origin=SECTION=0xADDR: --fragment-ends-module=SECTION)",
            )?,
            Some(n) => out
                .iter_mut()
                .find(|f| {
                    f.section
                        .as_deref()
                        .is_some_and(|m| m.eq_ignore_ascii_case(n))
                })
                .ok_or_else(|| {
                    format!("--fragment-ends-module={n} needs --fragment-origin={n}=0xADDR")
                })?,
        };
        f.ends_module = true;
    }
    Ok(out)
}

fn parse_args(mut argv: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut include_dirs = Vec::new();
    let mut root = None;
    let mut output = None;
    let mut input = None;
    let mut stop = Stop::Never;
    // (section or None for the plain form, origin)
    let mut origins: Vec<(Option<String>, u32)> = Vec::new();
    // `--fragment-ends-module`: None for the plain form
    let mut ends: Vec<Option<String>> = Vec::new();
    let mut loop_align = None;
    let mut always_from = None;
    let mut always = None;
    let mut absolute = None;
    let mut module_routines: Vec<ModuleRoutine> = Vec::new();
    let mut schedule = None;
    let mut data_bases: Vec<(String, String)> = Vec::new();
    let mut park_d0 = None;
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "-I" => include_dirs.push(argv.next().ok_or("-I needs a directory")?.into()),
            "--root" => {
                if root.is_some() {
                    return Err("--root given more than once".into());
                }
                root = Some(argv.next().ok_or("--root needs a directory")?.into());
            }
            "-o" => {
                if output.is_some() {
                    return Err("-o given more than once".into());
                }
                output = Some(argv.next().ok_or("-o needs a file name")?.into());
            }
            "--fragment-origin" => {
                let v = argv.next().ok_or("--fragment-origin needs an address")?;
                origins.push((None, fragment_address(&v)?));
            }
            s if s.starts_with("--fragment-origin=") => {
                let v = &s["--fragment-origin=".len()..];
                origins.push(match v.split_once('=') {
                    Some((sect, a)) if !sect.is_empty() => {
                        (Some(sect.to_string()), fragment_address(a)?)
                    }
                    Some(_) => {
                        return Err(format!("--fragment-origin: want SECTION=0xADDR, got `{v}`"))
                    }
                    None => (None, fragment_address(v)?),
                });
            }
            "--fragment-ends-module" => ends.push(None),
            s if s.starts_with("--fragment-ends-module=") => {
                let v = &s["--fragment-ends-module=".len()..];
                if v.is_empty() || v.contains('=') {
                    return Err(format!(
                        "--fragment-ends-module: want a code section, got `{v}`"
                    ));
                }
                ends.push(Some(v.to_string()));
            }
            s if s.starts_with("--loop-align=") || s.starts_with("--loop-align-off-from=") => {
                if loop_align.is_some() {
                    return Err("--loop-align / --loop-align-off-from given more than once".into());
                }
                let (opt, v) = s.split_once('=').unwrap();
                loop_align = Some(match (opt, v) {
                    ("--loop-align", "on") => LoopAlign::On,
                    ("--loop-align", "off") => LoopAlign::Off,
                    ("--loop-align", _) => {
                        return Err(format!("--loop-align: want `on` or `off`, got `{v}`"))
                    }
                    _ => LoopAlign::OffFrom(routine_list("--loop-align-off-from", v)?),
                });
            }
            s if s.starts_with("--a5-always-from=") => {
                if always_from.is_some() {
                    return Err("--a5-always-from given more than once".into());
                }
                always_from = Some(routine_list(
                    "--a5-always-from",
                    &s["--a5-always-from=".len()..],
                )?);
            }
            s if s.starts_with("--a5-always=") => {
                if always.is_some() {
                    return Err("--a5-always given more than once".into());
                }
                always = Some(routine_list("--a5-always", &s["--a5-always=".len()..])?);
            }
            s if s.starts_with("--a5-absolute=") => {
                if absolute.is_some() {
                    return Err("--a5-absolute given more than once".into());
                }
                absolute = Some(routine_list("--a5-absolute", &s["--a5-absolute=".len()..])?);
            }
            s if s.starts_with("--park-d0=") => {
                if park_d0.is_some() {
                    return Err("--park-d0 given more than once".into());
                }
                park_d0 = Some(routine_list("--park-d0", &s["--park-d0=".len()..])?);
            }
            s if s.starts_with("--module-routine=") => {
                let v = &s["--module-routine=".len()..];
                let bad = || format!("--module-routine: want [SECTION:]ROUTINE=0xADDR, got `{v}`");
                let (name, a) = v.split_once('=').ok_or_else(bad)?;
                let (section, name) = match name.split_once(':') {
                    Some((s, n)) if !s.is_empty() => (Some(s.to_string()), n),
                    Some(_) => return Err(bad()),
                    None => (None, name),
                };
                let a = parse_number(a).ok_or_else(bad)?;
                if name.is_empty() || a % 2 != 0 {
                    return Err(bad());
                }
                if module_routines
                    .iter()
                    .any(|r| r.symbol.eq_ignore_ascii_case(name))
                {
                    return Err(format!("--module-routine: `{name}` given more than once"));
                }
                module_routines.push(ModuleRoutine {
                    section,
                    symbol: name.to_string(),
                    addr: a,
                });
            }
            s if s.starts_with("--data-base=") => {
                let v = &s["--data-base=".len()..];
                let (sect, sym) = v
                    .split_once('=')
                    .filter(|(a, b)| !a.is_empty() && !b.is_empty() && !b.contains('='))
                    .ok_or_else(|| format!("--data-base: want SECTION=SYMBOL, got `{v}`"))?;
                if data_bases.iter().any(|(s, _)| s.eq_ignore_ascii_case(sect)) {
                    return Err(format!(
                        "--data-base: section `{sect}` given more than once"
                    ));
                }
                data_bases.push((sect.to_string(), sym.to_string()));
            }
            s if s.starts_with("--schedule=") => {
                if schedule.is_some() {
                    return Err("--schedule given more than once".into());
                }
                let v = &s["--schedule=".len()..];
                let bad = || format!("--schedule: want `off`, `r11` or `r11:N`, got `{v}`");
                schedule = Some(match v {
                    "off" => Schedule::Off,
                    "r11" => Schedule::R11 { window: 0 },
                    _ => match v.strip_prefix("r11:").map(str::parse::<u8>) {
                        Some(Ok(w)) if w > 0 => Schedule::R11 { window: w },
                        _ => return Err(bad()),
                    },
                });
            }
            "-h" | "--help" => return Err(String::new()),
            "--parse-only" => stop = Stop::AfterParse,
            "--dump-ast" => stop = Stop::DumpAst,
            "--check-only" => stop = Stop::AfterCheck,
            "--dump-locals" => stop = Stop::DumpLocals,
            s if s.starts_with("-I") && s.len() > 2 => include_dirs.push(s[2..].into()),
            s if s.starts_with('-') && s != "-" => return Err(format!("unknown option `{s}`")),
            _ => {
                if input.is_some() {
                    return Err("more than one input file".into());
                }
                input = Some(PathBuf::from(arg));
            }
        }
    }
    let input = input.ok_or("no input file")?;
    let fragment = fragment_sections(&origins, &ends)?;
    if !module_routines.is_empty() && fragment.is_empty() {
        return Err("--module-routine needs --fragment-origin".into());
    }
    Ok(Args {
        include_dirs,
        root,
        output,
        input,
        stop,
        fragment,
        loop_align: loop_align.unwrap_or_default(),
        data_base: DataBaseOptions {
            always_from: always_from.unwrap_or_default(),
            always: always.unwrap_or_default(),
            absolute: absolute.unwrap_or_default(),
        },
        module_routines,
        schedule: schedule.unwrap_or_default(),
        data_bases,
        park_d0: park_d0.unwrap_or_default(),
    })
}

/// Run the pipeline.  Diagnostics are returned for the caller to render
/// against `sm`; warnings from a successful run go into `warnings`.
fn compile(
    args: &Args,
    sm: &mut SourceMap,
    warnings: &mut Vec<Diagnostic>,
) -> Result<(), Vec<Diagnostic>> {
    let file = sm.load(&args.input).map_err(|e| {
        vec![Diagnostic::error_nopos(format!(
            "cannot read {}: {e}",
            args.input.display()
        ))]
    })?;
    let target = dpc_m68k::M68k;
    let opts = preprocess::Options {
        include_dirs: args.include_dirs.clone(),
        root: args.root.clone(),
        predeclared: target
            .predeclared_cond_vars()
            .iter()
            .map(|&(n, v)| (n.to_string(), v))
            .collect(),
        ..Default::default()
    };
    let pp = preprocess::preprocess(sm, file, &opts);
    if pp.has_errors() {
        return Err(pp.diagnostics);
    }
    warnings.extend(pp.diagnostics);
    let unit = dpc_syntax::parser::parse(&pp.tokens)?;
    match args.stop {
        Stop::Never | Stop::AfterCheck | Stop::DumpLocals => {}
        Stop::AfterParse => return Ok(()),
        Stop::DumpAst => {
            println!("{unit:#?}");
            return Ok(());
        }
    }
    let program = dpc_sema::analyze(&unit, &target)?;
    warnings.extend(program.warnings.iter().cloned());
    if args.stop == Stop::AfterCheck {
        return Ok(());
    }
    if args.stop == Stop::DumpLocals {
        let (functions, errors) = dpc_ir::lower_each(&program);
        for f in &functions {
            println!("{}", local_facts_json(&program, f));
        }
        for d in errors {
            warnings.push(Diagnostic {
                severity: dpc_syntax::diag::Severity::Warning,
                ..d
            });
        }
        return Ok(());
    }
    let module = dpc_ir::lower_with(&program, &args.data_base)?;
    let opts = dpc_m68k::emit::Options {
        fragment: args.fragment.clone(),
        loop_align: args.loop_align.clone(),
        module_routines: args.module_routines.clone(),
        schedule: args.schedule,
        data_bases: args.data_bases.clone(),
        park_d0: args.park_d0.clone(),
    };
    let asm = dpc_m68k::emit::emit(&module, &args.input.display().to_string(), &opts)?;

    let out = args
        .output
        .clone()
        .unwrap_or_else(|| args.input.with_extension("s"));
    if out == args.input {
        return Err(vec![Diagnostic::error_nopos(format!(
            "output {} would overwrite the input; use -o",
            out.display()
        ))]);
    }
    std::fs::write(&out, asm).map_err(|e| {
        vec![Diagnostic::error_nopos(format!(
            "cannot write {}: {e}",
            out.display()
        ))]
    })
}

/// One line of `--dump-locals`: the routine and the uses of each of its
/// scalar locals (`dpc_m68k::locals`), with the locals' names and type
/// kinds from the checked program.
fn local_facts_json(program: &dpc_sema::Program, f: &dpc_ir::Function) -> String {
    use dpc_m68k::locals::{body_uses, Site};
    use dpc_sema::types::TypeKind;
    use std::fmt::Write;
    let routine = program.routines.iter().find(|r| r.symbol == f.symbol);
    let var_at = |off: i32| {
        routine.and_then(|r| {
            r.frame_vars
                .iter()
                .map(|&v| program.var(v))
                .find(|v| matches!(v.storage, dpc_sema::Storage::Frame { offset } if offset == off))
        })
    };
    let kind = |v: &dpc_sema::Var| {
        let mut t = v.ty;
        loop {
            match program.types.kind(t) {
                TypeKind::Subrange { base, .. } => t = *base,
                TypeKind::Integer16 | TypeKind::Integer32 => return "integer",
                TypeKind::Boolean => return "boolean",
                TypeKind::Char => return "char",
                TypeKind::Enum { .. } => return "enum",
                TypeKind::Set { .. } => return "set",
                TypeKind::Pointer { .. } | TypeKind::UnivPtr | TypeKind::RoutinePointer(_) => {
                    return "pointer"
                }
                _ => return "other",
            }
        }
    };
    let site = |s: &Site| {
        let lp = match s.lp {
            Some((lo, hi)) => format!("[{lo},{hi}]"),
            None => "null".into(),
        };
        format!(
            "\"top\":{},\"seq\":{},\"nested\":{},\"loop\":{lp}",
            s.top, s.seq, s.nested
        )
    };
    let uses = body_uses(f);
    let mut o = String::new();
    write!(
        o,
        "{{\"routine\":\"{}\",\"global\":{},\"frame\":{},\"nested_routine\":{},\"stmts\":{},\"calls\":[",
        f.symbol,
        f.global,
        f.frame_size,
        f.nesting.is_some(),
        f.body.len()
    )
    .unwrap();
    for (k, c) in uses.calls.iter().enumerate() {
        write!(o, "{}{{{}}}", if k > 0 { "," } else { "" }, site(c)).unwrap();
    }
    o.push_str("],\"locals\":[");
    for (k, l) in uses.locals.iter().enumerate() {
        let v = var_at(l.offset);
        write!(
            o,
            "{}{{\"name\":\"{}\",\"type\":\"{}\",\"offset\":{},\"bytes\":{},\"deref\":{},\"other\":{},\"for_index\":{},\"across_call\":{},\"stores\":[",
            if k > 0 { "," } else { "" },
            v.map_or("?", |v| v.name.as_str()),
            v.map_or("?", kind),
            l.offset,
            l.width.bytes(),
            l.deref,
            l.other,
            l.for_index,
            uses.across_call(l)
        )
        .unwrap();
        for (j, s) in l.stores.iter().enumerate() {
            write!(
                o,
                "{}{{{},\"const\":{},\"call\":{}}}",
                if j > 0 { "," } else { "" },
                site(&s.at),
                s.konst,
                s.call
            )
            .unwrap();
        }
        o.push_str("],\"loads\":[");
        for (j, r) in l.loads.iter().enumerate() {
            write!(
                o,
                "{}{{{},\"arg\":{},\"whole\":{},\"cond\":{}}}",
                if j > 0 { "," } else { "" },
                site(&r.at),
                r.arg,
                r.whole,
                r.cond
            )
            .unwrap();
        }
        o.push_str("]}");
    }
    o.push_str("]}");
    o
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(msg) => {
            if msg.is_empty() {
                // -h / --help: not an error.
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            eprintln!("{PROG}: error: {msg}");
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    // The passes recurse over the program's nesting; they run on a thread
    // whose stack `MAX_DEPTH` (dpc_syntax::parser) is sized for (bead
    // domain-pascal-9gb).  The backstop runs on that thread too: the
    // routine context it reports is that thread's.
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .name("dpc".into())
            .stack_size(dpc_syntax::parser::STACK_SIZE)
            .spawn_scoped(s, || run(&args))
            .map(|h| h.join())
    })
    .unwrap_or_else(|e| Err(Box::new(e)))
    .unwrap_or_else(|_| {
        // The backstop catches every panic of the compiler; this is the
        // thread itself failing.
        eprintln!("{PROG}: internal error: the compiler thread failed (please report)");
        ExitCode::from(3)
    })
}

/// Compile `args.input`, report, and give the exit status: 0, 1 with
/// diagnostics, 3 on an internal error ([`backstop`]).
fn run(args: &Args) -> ExitCode {
    let mut sm = SourceMap::new();
    let mut warnings = Vec::new();
    let panic_now = test_panic_setup();
    let result = backstop(|| {
        if panic_now {
            panic!("DPC_INTERNAL_PANIC_FOR_TEST");
        }
        compile(args, &mut sm, &mut warnings)
    });
    for d in &warnings {
        eprintln!("{}", d.render_with_source(&sm, PROG));
    }
    match result {
        Ok(Ok(())) => ExitCode::SUCCESS,
        Ok(Err(diags)) => {
            for d in &diags {
                eprintln!("{}", d.render_with_source(&sm, PROG));
            }
            ExitCode::FAILURE
        }
        Err(panic) => {
            eprintln!("{}", internal_error(args, &sm, &panic));
            ExitCode::from(3)
        }
    }
}

/// A panic caught by [`backstop`]: its message and where in the compiler
/// it was raised.
struct Panic {
    message: String,
    location: String,
}

std::thread_local! {
    static PANIC: std::cell::RefCell<Option<Panic>> = const { std::cell::RefCell::new(None) };
}

/// Runs `f`, turning a panic (a compiler bug: every user-reachable failure
/// is a diagnostic) into `Err` instead of a crash.  The default panic
/// message is suppressed unless `RUST_BACKTRACE` is set (not to `0`), so the report is
/// the one line of [`internal_error`].
fn backstop<T>(f: impl FnOnce() -> T) -> Result<T, Panic> {
    let verbose = std::env::var_os("RUST_BACKTRACE").is_some_and(|v| v != "0");
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "panic".to_string()
        };
        let location = info
            .location()
            .map_or_else(String::new, |l| format!("{}:{}", l.file(), l.line()));
        PANIC.with(|p| *p.borrow_mut() = Some(Panic { message, location }));
        if verbose {
            default(info);
        }
    }));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    let _ = std::panic::take_hook();
    r.map_err(|_| {
        PANIC.with(|p| p.borrow_mut().take()).unwrap_or(Panic {
            message: "panic".into(),
            location: String::new(),
        })
    })
}

/// The report of a caught panic, one line: `FILE:LINE:COL: internal
/// error: MSG (at SRC:LINE; please report)`, at the routine being
/// compiled (`dpc_syntax::diag::context`) when there is one, else
/// `FILE: internal error: ...` at the input file.  A message of several
/// lines (`assert_eq!`'s) is joined into one.
fn internal_error(args: &Args, sm: &SourceMap, panic: &Panic) -> String {
    let at = if panic.location.is_empty() {
        String::new()
    } else {
        format!("at {}; ", panic.location)
    };
    let text: Vec<&str> = panic
        .message
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let message = format!("internal error: {} ({at}please report)", text.join(" "));
    match dpc_syntax::diag::context() {
        Some(span) => {
            let file = sm.file(span.file);
            let lc = file.line_col(span.lo);
            format!("{}:{}:{}: {message}", file.path.display(), lc.line, lc.col)
        }
        None => format!("{}: {message}", args.input.display()),
    }
}

/// Test only: `DPC_INTERNAL_PANIC_FOR_TEST=N` makes the compiler panic,
/// so the tests can drive the internal-error backstop end to end (no
/// input is known to panic).  `0` panics before anything is compiled;
/// `N` > 0 when the phases (checker, lowering, selector) enter a
/// routine for the Nth time, counting every entry of every phase
/// (`dpc_syntax::diag::set_context_hook`).  Not for anything but tests.
/// True for `0`: the caller panics.
fn test_panic_setup() -> bool {
    std::thread_local! {
        static LEFT: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    }
    let Some(n) = std::env::var("DPC_INTERNAL_PANIC_FOR_TEST")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
    else {
        return false;
    };
    if n == 0 {
        return true;
    }
    LEFT.with(|l| l.set(n));
    dpc_syntax::diag::set_context_hook(Some(|_| {
        let left = LEFT.with(|l| {
            l.set(l.get().saturating_sub(1));
            l.get()
        });
        if left == 0 {
            panic!("DPC_INTERNAL_PANIC_FOR_TEST");
        }
    }));
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Result<Args, String> {
        parse_args(v.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_options() {
        let a = args(&["-I", "a", "-Ib", "-o", "x.s", "m.pas"]).unwrap();
        assert_eq!(a.include_dirs, vec![PathBuf::from("a"), PathBuf::from("b")]);
        assert_eq!(a.output, Some(PathBuf::from("x.s")));
        assert_eq!(a.input, PathBuf::from("m.pas"));
        assert_eq!(a.stop, Stop::Never);
        assert!(a.fragment.is_empty());
        assert_eq!(a.root, None);
        let a = args(&["--root", "/x/install", "--check-only", "m.pas"]).unwrap();
        assert_eq!(a.root, Some(PathBuf::from("/x/install")));
        assert_eq!(a.stop, Stop::AfterCheck);
        assert!(args(&["--root", "a", "--root", "b", "m.pas"]).is_err());
        assert!(args(&["m.pas", "--root"]).is_err());
        let a = args(&["--fragment-origin", "0xE148CA", "m.pas"]).unwrap();
        assert_eq!(a.fragment, vec![FragmentSection::only(0xE148CA)]);
        let a = args(&["--fragment-origin=0xE148CA", "m.pas"]).unwrap();
        assert_eq!(a.fragment, vec![FragmentSection::only(0xE148CA)]);
        let a = args(&["--fragment-origin", "16", "m.pas"]).unwrap();
        assert_eq!(a.fragment, vec![FragmentSection::only(16)]);
        let a = args(&[
            "--fragment-origin",
            "0xE128F6",
            "--fragment-ends-module",
            "m.pas",
        ]);
        assert!(a.unwrap().fragment[0].ends_module);
        assert_eq!(args(&["m.pas"]).unwrap().loop_align, LoopAlign::On);
        assert_eq!(
            args(&["--loop-align=on", "m.pas"]).unwrap().loop_align,
            LoopAlign::On
        );
        assert_eq!(
            args(&["--loop-align=off", "m.pas"]).unwrap().loop_align,
            LoopAlign::Off
        );
        assert_eq!(
            args(&["--loop-align-off-from=vtoc_$dismount", "m.pas"])
                .unwrap()
                .loop_align,
            LoopAlign::OffFrom(vec!["vtoc_$dismount".into()])
        );
        // one routine per code section (codegen-patterns 7, 11)
        assert_eq!(
            args(&["--loop-align-off-from=a,b", "m.pas"])
                .unwrap()
                .loop_align,
            LoopAlign::OffFrom(vec!["a".into(), "b".into()])
        );
        assert_eq!(
            args(&["--a5-always-from=a,b", "m.pas"])
                .unwrap()
                .data_base
                .always_from,
            ["a", "b"]
        );
        assert!(args(&["--a5-always-from=a,", "m.pas"]).is_err());
        assert_eq!(
            args(&["m.pas"]).unwrap().data_base,
            DataBaseOptions::default()
        );
        let a = args(&[
            "--a5-always-from=smd_$busy_wait",
            "--a5-always=P,q_$x",
            "--a5-absolute=AUDIT_$SUSPEND",
            "m.pas",
        ])
        .unwrap();
        assert_eq!(
            a.data_base,
            DataBaseOptions {
                always_from: vec!["smd_$busy_wait".into()],
                always: vec!["P".into(), "q_$x".into()],
                absolute: vec!["AUDIT_$SUSPEND".into()],
            }
        );
        let a = args(&[
            "--fragment-origin",
            "0xE6BE1C",
            "--module-routine=DISK_$UNASSIGN=0xE6BDA8",
            "--module-routine=X=16",
            "m.pas",
        ])
        .unwrap();
        assert_eq!(
            a.module_routines,
            vec![
                ModuleRoutine::new("DISK_$UNASSIGN", 0xE6BDA8),
                ModuleRoutine::new("X", 16)
            ]
        );
        for bad in [
            &["--module-routine=X=0x10", "m.pas"][..],
            &["--fragment-origin", "16", "--module-routine=X", "m.pas"],
            &["--fragment-origin", "16", "--module-routine==0x10", "m.pas"],
            &[
                "--fragment-origin",
                "16",
                "--module-routine=X=0x11",
                "m.pas",
            ],
            &[
                "--fragment-origin",
                "16",
                "--module-routine=X=16",
                "--module-routine=x=18",
                "m.pas",
            ],
        ] {
            assert!(args(bad).is_err(), "{bad:?}");
        }
        assert_eq!(
            args(&["--parse-only", "m.pas"]).unwrap().stop,
            Stop::AfterParse
        );
        assert_eq!(args(&["--dump-ast", "m.pas"]).unwrap().stop, Stop::DumpAst);
        assert_eq!(
            args(&["--check-only", "m.pas"]).unwrap().stop,
            Stop::AfterCheck
        );
    }

    #[test]
    fn rejects_bad_usage() {
        assert!(args(&[]).is_err());
        assert!(args(&["-o"]).is_err());
        assert!(args(&["-x", "m.pas"]).is_err());
        assert!(args(&["a.pas", "b.pas"]).is_err());
        assert!(args(&["--fragment-origin", "m.pas"]).is_err());
        assert!(args(&["--fragment-origin", "0xZZ", "m.pas"]).is_err());
        assert!(args(&["m.pas", "--fragment-origin"]).is_err());
        assert!(args(&["--fragment-origin", "0xE148CB", "m.pas"]).is_err());
        assert!(args(&["--fragment-ends-module", "m.pas"]).is_err());
        assert!(args(&["--loop-align=maybe", "m.pas"]).is_err());
        assert!(args(&["--loop-align", "off", "m.pas"]).is_err());
        assert!(args(&["--loop-align-off-from=", "m.pas"]).is_err());
        assert!(args(&["--loop-align=off", "--loop-align-off-from=P", "m.pas"]).is_err());
        assert!(args(&["--a5-always-from=", "m.pas"]).is_err());
        assert!(args(&["--a5-always-from=P", "--a5-always-from=Q", "m.pas"]).is_err());
        assert!(args(&["--a5-always=", "m.pas"]).is_err());
        assert!(args(&["--a5-always=P,", "m.pas"]).is_err());
        assert!(args(&["--a5-always=P", "--a5-always=Q", "m.pas"]).is_err());
        assert!(args(&["--a5-absolute=,P", "m.pas"]).is_err());
        assert_eq!(
            args(&["--park-d0=TTY_$K_FLUSH_INPUT,b", "m.pas"])
                .unwrap()
                .park_d0,
            vec!["TTY_$K_FLUSH_INPUT".to_string(), "b".to_string()]
        );
        assert!(args(&["--park-d0=", "m.pas"]).is_err());
        assert!(args(&["--park-d0=a", "--park-d0=b", "m.pas"]).is_err());
        assert!(args(&["--a5-absolute=P", "--a5-absolute=Q", "m.pas"]).is_err());
        assert!(args(&["--a5-always-from", "P", "m.pas"]).is_err());
        assert!(args(&["--data-base=", "m.pas"]).is_err());
        assert!(args(&["--data-base=WIRED_DATA", "m.pas"]).is_err());
        assert!(args(&["--data-base==PROC1_$DATA", "m.pas"]).is_err());
        assert!(args(&["--data-base=W=", "m.pas"]).is_err());
        assert!(args(&["--data-base=W=A=B", "m.pas"]).is_err());
        assert!(args(&["--data-base=w=A", "--data-base=W=B", "m.pas"]).is_err());
    }

    /// Per-section fragment origins and module routines with a section
    /// (a unit of a module with several code sections, TTY).
    #[test]
    fn parses_section_fragments() {
        let a = args(&[
            "--fragment-origin=wired_proc=0xE1AF0A",
            "--fragment-origin=.text=0xE67618",
            "--fragment-ends-module=.TEXT",
            "--module-routine=WIRED_PROC:TTY_$I_LOCK=0xE1AED0",
            "--module-routine=TTY_$I_GET_DESC=0xE66738",
            "m.pas",
        ])
        .unwrap();
        assert_eq!(
            a.fragment,
            vec![
                FragmentSection {
                    section: Some("wired_proc".into()),
                    origin: 0xE1AF0A,
                    ends_module: false
                },
                FragmentSection {
                    section: Some(".text".into()),
                    origin: 0xE67618,
                    ends_module: true
                },
            ]
        );
        assert_eq!(
            a.module_routines,
            vec![
                ModuleRoutine {
                    section: Some("WIRED_PROC".into()),
                    symbol: "TTY_$I_LOCK".into(),
                    addr: 0xE1AED0
                },
                ModuleRoutine::new("TTY_$I_GET_DESC", 0xE66738),
            ]
        );
        for bad in [
            // plain and per-section forms do not mix
            &[
                "--fragment-origin",
                "16",
                "--fragment-origin=S=0x20",
                "m.pas",
            ][..],
            &["--fragment-origin", "16", "--fragment-origin=0x20", "m.pas"],
            &[
                "--fragment-origin=S=16",
                "--fragment-origin=s=0x20",
                "m.pas",
            ],
            &["--fragment-origin==16", "m.pas"],
            &["--fragment-origin=S=17", "m.pas"],
            &["--fragment-origin=S=", "m.pas"],
            // an ending section needs its origin
            &[
                "--fragment-origin=S=16",
                "--fragment-ends-module=T",
                "m.pas",
            ],
            &["--fragment-origin=S=16", "--fragment-ends-module", "m.pas"],
            &[
                "--fragment-origin",
                "16",
                "--fragment-ends-module=S",
                "m.pas",
            ],
            &[
                "--fragment-origin=S=16",
                "--fragment-ends-module=S",
                "--fragment-ends-module=s",
                "m.pas",
            ],
            &["--fragment-ends-module=", "m.pas"],
            &[
                "--fragment-origin=S=16",
                "--module-routine=:X=0x10",
                "m.pas",
            ],
            &[
                "--fragment-origin=S=16",
                "--module-routine=S:X=0x10",
                "--module-routine=T:x=0x20",
                "m.pas",
            ],
            &["--module-routine=S:X=0x10", "m.pas"],
        ] {
            assert!(args(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn parses_data_base() {
        assert!(args(&["m.pas"]).unwrap().data_bases.is_empty());
        let a = args(&[
            "--data-base=wired_data=PROC1_$DATA",
            "--data-base=.data=X",
            "m.pas",
        ])
        .unwrap();
        assert_eq!(
            a.data_bases,
            vec![
                ("wired_data".to_string(), "PROC1_$DATA".to_string()),
                (".data".to_string(), "X".to_string())
            ]
        );
    }

    /// Bead domain-pascal-1dn: a panic is caught and reported as one
    /// `internal error` line, at the routine being compiled when one is
    /// recorded, else at the input file.
    #[test]
    fn backstop_reports_a_panic_as_an_internal_error() {
        assert_eq!(backstop(|| 7).ok(), Some(7));
        let p = backstop(|| panic!("boom {}", 1)).err().unwrap();
        assert_eq!(p.message, "boom 1");
        assert!(p.location.contains("main.rs"), "{}", p.location);
        let a = args(&["m.pas"]).unwrap();
        let mut sm = SourceMap::new();
        dpc_syntax::diag::set_context(None);
        let line = internal_error(&a, &sm, &p);
        assert!(
            line.starts_with("m.pas: internal error: boom 1 (at "),
            "{line}"
        );
        assert!(line.ends_with("; please report)"), "{line}");
        let f = sm.add("m.pas", "module m;\nprocedure p; begin end;\n".into());
        let span = dpc_syntax::Span::new(f, 10, 22);
        dpc_syntax::diag::set_context(Some(span));
        let line = internal_error(&a, &sm, &p);
        dpc_syntax::diag::set_context(None);
        assert!(
            line.starts_with("m.pas:2:1: internal error: boom 1 (at "),
            "{line}"
        );
        // One line, whatever the message.
        let p = backstop(|| assert_eq!(1, 2)).err().unwrap();
        let line = internal_error(&a, &sm, &p);
        assert_eq!(line.lines().count(), 1, "{line}");
    }
}
