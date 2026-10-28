//! `dpc`: the Domain Pascal compiler driver.
//!
//! usage: dpc --print-runtime[=TARGET]
//!        dpc [-I dir]... [--root DIR] [-o out.s] [--fragment-origin 0xADDR [--fragment-ends-module]]
//!            [--fragment-origin=SECTION=0xADDR [--fragment-ends-module=SECTION]]...
//!            [--loop-align=on|off | --loop-align-off-from=ROUTINE,...]
//!            [--a5-always-from=ROUTINE,...] [--a5-always=ROUTINE,...]
//!            [--a5-absolute=ROUTINE,...] [--module-routine=[SECTION:]ROUTINE=0xADDR]...
//!            [--schedule=off|r11|r11:N] [--data-base=SECTION=SYMBOL]...
//!            [--merge-constant-stores] [--strict]
//!            [--parse-only | --dump-ast | --check-only | --dump-locals] file.pas
//!
//! The reference's compiler options (ref 6.4, Table 6-2, pp. 6-6..6-8:
//! `-config var...`, `-cond`, `-idir dir...`, `-nwarn`, `-msgs`, `-b
//! [pathname]`, `-nb`, `-natural`, `-comchk`, `-version`, ...) are
//! accepted too; `refopts` and `docs/compiler-options.md` say what dpc
//! does with each.  Precedence: an argument spelt exactly as a reference
//! option (one dash, lower case) is that option; otherwise dpc's own
//! options are tried (`-I`, `-Idir`, `-o`, `-h`, `--...`); any other
//! argument beginning with `-` is an error.  No reference option is spelt
//! like one of dpc's (`-idir` is not `-I` + `dir`: `-I` is upper case).
//! The reference's list options take the arguments up to the next one
//! beginning with `-`, so the source file goes first (`dpc m.pas -config
//! a b`) or after `--` (`dpc -config a b -- m.pas`).
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
//! A pointer local assigned once from a call and only dereferenced stays in
//! the address register allocation gives it (A0 where no call follows,
//! TTY_$I_INQ_RAW 0xE67414); a 4-byte integer local assigned an
//! `a0_return` function's result and read only as a type-transferred
//! pointer it dereferences is held in D0 (`move.l A0,D0`) and moved to an
//! address register where first dereferenced (codegen-patterns 3, "Pointer
//! locals held in D0"; bead domain-pascal-yzxr).  The options that held
//! pointer locals in D0 (`--park-d0`, and the default that
//! `--no-park-pointer-locals` turned off) were retired with that reading
//! (bead domain-pascal-78n2).
//!
//! `dpc --print-runtime` or `dpc --print-runtime=TARGET` (alone, like
//! `gcc -print-libgcc-file-name`) prints the path of the target's prebuilt
//! run-time library, the archive compiled code is linked with (m68k:
//! `libm_arith.a`, M$ARITH; runtime/README.md), and exits 0; 1 when the
//! file is not there (not built: `make runtime`).  TARGET defaults to, and
//! may only be, `m68k`.  The library is looked for in
//! `$DPC_RUNTIME_DIR/TARGET/` when that variable is set, else in
//! `runtime/TARGET/` beside the directory holding dpc (for
//! `target/debug/dpc`, `target/runtime/m68k/`, where `make build` puts
//! it).  The file name comes from the target description
//! (`dpc_target::Target::runtime_library`).
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
//!
//! `--merge-constant-stores` (a per-unit build setting, as the A5 and
//! loop-alignment options are; bead domain-pascal-qb9q): the unit merges
//! two consecutive statements that store word constants to adjacent words
//! in ascending order into one longword store, as the image's merging
//! units do (TTY_$K_RESET 0xE67322 `move.l #0x10001,(0x2ca,A2)`;
//! codegen-patterns 13, "Store merging"; `dpc_m68k::emit::Options::
//! merge_constant_stores`).  Without it such pairs are two stores, as in
//! the units that do not merge (TTY_$I_INIT 0xE3332E).  Which units merge
//! is not something the source says: `tools/storesurvey.py units` derives
//! it from the image, per map segment, and `tools/srcsurvey.py` gives the
//! option to the ../source modules whose segments merge.  Shapes neither
//! kind of unit settles are refused either way (`dpc-m68k`, `isel/merge.rs`).
//!
//! `--strict`: every refusal is an error.  Without it (the default, owner
//! 2026-10-17, bead domain-pascal-5kgg) a refusal that only guards the
//! image's exact form (a register choice, an instruction order, a hold or
//! a reload, which of two seen forms), at a site where dpc has a correct
//! code path, is reported as a warning `best effort (not byte-identical):
//! ...` and that path is taken (`dpc_syntax::diag::best_effort`): the
//! output computes the right values but is not checked against the image.
//! Refusals of constructs dpc does not implement stay errors either way.
//! The conformance harness, the golden tests and `tools/srcsurvey.py` use
//! `--strict`; `tools/linktest.py` and ../source's build do not.

mod refopts;

use std::path::PathBuf;
use std::process::ExitCode;

use refopts::{Binary, RefOptions};

use dpc_ir::DataBaseOptions;
use dpc_m68k::emit::{FragmentSection, LoopAlign, ModuleRoutine};
use dpc_m68k::sched::Schedule;
use dpc_syntax::{preprocess, Diagnostic, SourceMap};
use dpc_target::Target;

const PROG: &str = "dpc";
const USAGE: &str = "usage: dpc --print-runtime[=TARGET]\n       \
                     dpc [-I dir]... [--root DIR] [-o out.s] \
                     [--fragment-origin 0xADDR [--fragment-ends-module]] \
                     [--fragment-origin=SECTION=0xADDR [--fragment-ends-module=SECTION]]... \
                     [--loop-align=on|off | --loop-align-off-from=ROUTINE,...] \
                     [--a5-always-from=ROUTINE,...] [--a5-always=ROUTINE,...] \
                     [--a5-absolute=ROUTINE,...] [--module-routine=[SECTION:]ROUTINE=0xADDR]... \
                     [--schedule=off|r11|r11:N] [--data-base=SECTION=SYMBOL]... \
                     [--merge-constant-stores] [--strict] \
                     [--parse-only | --dump-ast | --check-only | --dump-locals] \
                     [reference options: -config var..., -cond, -idir dir..., -nwarn, -msgs, \
                     -b [pathname], -nb, -natural, -comchk, ... (docs/compiler-options.md)] \
                     [--] file.pas\n       \
                     dpc -version | --version";

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
    /// `-I` and `-idir` directories, in command-line order.
    include_dirs: Vec<PathBuf>,
    /// The reference's options (`refopts`); its `include_dirs` are
    /// merged into `include_dirs`, its `-b pathname` into `output`.
    refs: RefOptions,
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
    merge_constant_stores: bool,
    /// `--strict`: every refusal is an error, also those with a
    /// best-effort path (`dpc_syntax::diag::best_effort`).
    strict: bool,
}

/// The routine names of `--a5-always=`, `--a5-absolute=`,
/// `--a5-always-from=` and `--loop-align-off-from=`: a comma list without
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

fn parse_args(argv: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut argv = argv.peekable();
    let mut refs = RefOptions::default();
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
    let mut merge_constant_stores = false;
    let mut strict = false;
    while let Some(arg) = argv.next() {
        // The reference's options first, by their exact spellings
        // (`refopts`; none is spelt like one of dpc's own).
        if let Some(r) = refopts::parse(&arg, &mut argv, &mut refs) {
            r?;
            // `-idir` and `-I` directories are searched in the order given.
            include_dirs.append(&mut refs.include_dirs);
            continue;
        }
        match arg.as_str() {
            // The end of the options: what follows is the input file.
            "--" => {
                for a in argv.by_ref() {
                    if input.is_some() {
                        return Err("more than one input file".into());
                    }
                    input = Some(PathBuf::from(a));
                }
            }
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
            "--merge-constant-stores" => {
                if merge_constant_stores {
                    return Err("--merge-constant-stores given more than once".into());
                }
                merge_constant_stores = true;
            }
            "--strict" => {
                if strict {
                    return Err("--strict given more than once".into());
                }
                strict = true;
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
    let input = input.ok_or_else(|| {
        if refs.took_operands.is_empty() {
            "no input file".to_string()
        } else {
            format!(
                "no input file ({} took the arguments after it; give the source file \
                 first, as in `pas source [options]` (ref 6.3, p. 6-4), or after `--`)",
                refs.took_operands.join(", ")
            )
        }
    })?;
    // -b pathname / -nb (ref 6.4.3, p. 6-9) and -o name the same file.
    match (&refs.binary, &output) {
        (Binary::Path(p), None) => output = Some(p.clone()),
        (Binary::Path(_), Some(_)) => return Err("-b pathname and -o do not mix".into()),
        (Binary::Suppress, Some(_)) => return Err("-nb and -o do not mix".into()),
        _ => {}
    }
    let fragment = fragment_sections(&origins, &ends)?;
    if !module_routines.is_empty() && fragment.is_empty() {
        return Err("--module-routine needs --fragment-origin".into());
    }
    Ok(Args {
        include_dirs,
        refs,
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
        merge_constant_stores,
        strict,
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
        config: args.refs.config.clone(),
        cond: args.refs.cond,
        comchk: args.refs.comchk,
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
    let sema = dpc_sema::Options {
        natural: args.refs.natural,
    };
    let program = dpc_sema::analyze_with(&unit, &target, &sema)?;
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
        merge_constant_stores: args.merge_constant_stores,
        cpu: args.refs.cpu,
    };
    let asm = dpc_m68k::emit::emit(&module, &args.input.display().to_string(), &opts)?;
    // -nb (ref 6.4.3, p. 6-9): compiled, but no output file.
    if args.refs.binary == Binary::Suppress {
        return Ok(());
    }

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
    let write_uses = |o: &mut String,
                      uses: &dpc_m68k::locals::BodyUses,
                      vars: &[Option<&dpc_sema::Var>]| {
        for (k, l) in uses.locals.iter().enumerate() {
            let v = vars[k];
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
    };
    let vars: Vec<_> = uses.locals.iter().map(|l| var_at(l.offset)).collect();
    write_uses(&mut o, &uses, &vars);
    // The value parameters' copies of 1, 2 or 4 bytes (codegen-patterns
    // 1.2, "Held in a register instead"; bead domain-pascal-gfnj).
    let (pidx, puses) = dpc_m68k::locals::param_copy_uses(f);
    let vars: Vec<_> = puses
        .locals
        .iter()
        .map(|l| {
            routine.and_then(|r| {
                r.params
                    .iter()
                    .find(|p| p.copy_offset == Some(l.offset))
                    .map(|p| program.var(p.var))
            })
        })
        .collect();
    o.push_str("],\"params\":[");
    write_uses(&mut o, &puses, &vars);
    // Each copy's argument slot, in the order of "params".
    o.push_str("],\"param_slots\":[");
    for (k, &i) in pidx.iter().enumerate() {
        let slot = f.param_copies[i].slot;
        write!(o, "{}{slot}", if k > 0 { "," } else { "" }).unwrap();
    }
    o.push_str("]}");
    o
}

/// The target named by `--print-runtime[=TARGET]` (`opt`), or None when
/// `opt` is not that option.
fn print_runtime_target(opt: &str) -> Option<Result<&'static dyn Target, String>> {
    let name = match opt.strip_prefix("--print-runtime") {
        Some("") => "m68k",
        Some(v) => v.strip_prefix('=')?,
        None => return None,
    };
    Some(match name {
        "m68k" => Ok(&dpc_m68k::M68k),
        _ => Err(format!("--print-runtime: unknown target `{name}`")),
    })
}

/// The directory holding the targets' run-time libraries:
/// `$DPC_RUNTIME_DIR`, else `runtime` beside the directory of the dpc
/// executable `exe`.
fn runtime_dir(env: Option<std::ffi::OsString>, exe: &std::path::Path) -> PathBuf {
    match env {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => {
            let bin = exe.parent().unwrap_or(std::path::Path::new("."));
            bin.parent().unwrap_or(bin).join("runtime")
        }
    }
}

/// `dpc --print-runtime[=TARGET]`: print the run-time library's path.
fn print_runtime(target: &dyn Target) -> ExitCode {
    let Some(lib) = target.runtime_library() else {
        eprintln!(
            "{PROG}: error: target {} has no run-time library",
            target.name()
        );
        return ExitCode::from(1);
    };
    // the executable itself, not a symlink to it (on macOS current_exe
    // is the path dpc was started by: ~/bin/dpc -> target/debug/dpc must
    // find target/runtime, not ~/runtime)
    let exe = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .unwrap_or_else(|_| PathBuf::from(PROG));
    let path = runtime_dir(std::env::var_os("DPC_RUNTIME_DIR"), &exe)
        .join(target.name())
        .join(lib);
    if !path.is_file() {
        eprintln!(
            "{PROG}: error: {} not found (not built? `make runtime'; or set DPC_RUNTIME_DIR)",
            path.display()
        );
        return ExitCode::from(1);
    }
    println!("{}", path.display());
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if let Some(t) = argv.first().and_then(|a| print_runtime_target(a)) {
        let t = match (t, argv.len()) {
            (Ok(t), 1) => t,
            (Err(msg), _) => {
                eprintln!("{PROG}: error: {msg}");
                return ExitCode::from(2);
            }
            (Ok(_), _) => {
                eprintln!("{PROG}: error: --print-runtime takes no other arguments");
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        };
        return print_runtime(t);
    }
    // `pas -version` (ref 6.4.29, p. 6-35): alone, without a file name.
    // `--version` is the same: the build's commit lets a measurement
    // record which compiler it was made with (build.rs).
    if argv.len() == 1 && (argv[0] == "-version" || argv[0] == "--version") {
        println!("{PROG} {} (commit {})", env!("CARGO_PKG_VERSION"), env!("DPC_COMMIT"));
        return ExitCode::SUCCESS;
    }
    let args = match parse_args(argv.into_iter()) {
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
    // Options accepted although dpc does not give what the reference
    // describes (`refopts`): one note each.
    for n in &args.refs.notes {
        eprintln!("{PROG}: note: {n}");
    }
    let mut sm = SourceMap::new();
    let mut warnings = Vec::new();
    let panic_now = test_panic_setup();
    dpc_syntax::diag::set_strict(args.strict);
    let result = backstop(|| {
        if panic_now {
            panic!("DPC_INTERNAL_PANIC_FOR_TEST");
        }
        compile(args, &mut sm, &mut warnings)
    });
    // The refusals taken on a best-effort path (not `--strict`), as
    // warnings: the output is correct but not checked against the image.
    warnings.extend(dpc_syntax::diag::take_best_effort());
    let mut counts = Counts::default();
    // -nwarn (ref 6.4.30, p. 6-35) suppresses warning messages, "though
    // it does report on the total number of warnings that would have been
    // issued" (in the -msgs report).
    let mut report = |d: &Diagnostic| {
        if d.is_error() {
            counts.errors += 1;
        } else if d.severity == dpc_syntax::diag::Severity::Warning {
            counts.warnings += 1;
            if !args.refs.warn {
                return;
            }
        }
        eprintln!("{}", d.render_with_source(&sm, PROG));
    };
    warnings.iter().for_each(&mut report);
    let code = match result {
        Ok(Ok(())) => ExitCode::SUCCESS,
        Ok(Err(diags)) => {
            diags.iter().for_each(&mut report);
            ExitCode::FAILURE
        }
        Err(panic) => {
            eprintln!("{}", internal_error(args, &sm, &panic));
            counts.errors += 1;
            ExitCode::from(3)
        }
    };
    if args.refs.msgs {
        eprintln!("{}", counts.report());
    }
    code
}

/// The messages of one compilation, for the `-msgs` report.
#[derive(Debug, Default)]
struct Counts {
    errors: usize,
    warnings: usize,
}

impl Counts {
    /// The final compilation report of `-msgs` (ref 6.4.21, p. 6-25):
    /// "xx errors, yy warnings, zz info msgs, Pascal compiler variant Rev
    /// n.n", the counts "either `no` or a number" (singular for one, as in
    /// the examples "1 error, no warnings" and "No errors, 1 warning",
    /// p. 4-51, p. 4-52), with dpc's name and version for the compiler's.
    /// dpc issues no information messages.
    fn report(&self) -> String {
        let n = |k: usize, one: &str, many: &str| match k {
            0 => format!("no {many}"),
            1 => format!("1 {one}"),
            k => format!("{k} {many}"),
        };
        let line = format!(
            "{}, {}, no info msgs, {PROG} {}",
            n(self.errors, "error", "errors"),
            n(self.warnings, "warning", "warnings"),
            env!("CARGO_PKG_VERSION")
        );
        let mut c = line.chars();
        match c.next() {
            Some(f) => f.to_uppercase().chain(c).collect(),
            None => line,
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
    fn print_runtime_option() {
        let t = print_runtime_target("--print-runtime").unwrap().unwrap();
        assert_eq!(t.name(), "m68k");
        assert_eq!(t.runtime_library(), Some("libm_arith.a"));
        let t = print_runtime_target("--print-runtime=m68k")
            .unwrap()
            .unwrap();
        assert_eq!(t.name(), "m68k");
        assert!(print_runtime_target("--print-runtime=aarch64")
            .unwrap()
            .is_err());
        assert!(print_runtime_target("--print-runtimes").is_none());
        assert!(print_runtime_target("m.pas").is_none());
    }

    #[test]
    fn runtime_dir_beside_the_executables_directory() {
        let exe = std::path::Path::new("/x/target/debug/dpc");
        assert_eq!(runtime_dir(None, exe), PathBuf::from("/x/target/runtime"));
        assert_eq!(
            runtime_dir(Some("".into()), exe),
            PathBuf::from("/x/target/runtime")
        );
        assert_eq!(runtime_dir(Some("/rt".into()), exe), PathBuf::from("/rt"));
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
        // Retired with the integer reading of D0 parking (bead
        // domain-pascal-78n2): unknown options now.
        assert!(args(&["--park-d0=TTY_$K_FLUSH_INPUT", "m.pas"]).is_err());
        assert!(args(&["--no-park-pointer-locals", "m.pas"]).is_err());
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

    /// The reference's options beside dpc's own (`refopts`).
    #[test]
    fn parses_reference_options() {
        let a = args(&[
            "m.pas", "-I", "a", "-idir", "b", "c", "-Id", "-config", "x", "y", "--strict",
        ])
        .unwrap();
        assert_eq!(a.input, PathBuf::from("m.pas"));
        assert_eq!(a.include_dirs, ["a", "b", "c", "d"].map(PathBuf::from));
        assert_eq!(a.refs.config, Some(vec!["x".into(), "y".into()]));
        assert!(a.strict);
        // `--` ends the options.
        let a = args(&["-config", "x", "--", "m.pas"]).unwrap();
        assert_eq!(a.input, PathBuf::from("m.pas"));
        assert!(args(&["--", "a.pas", "b.pas"]).is_err());
        let a = args(&["--", "-odd.pas"]).unwrap();
        assert_eq!(a.input, PathBuf::from("-odd.pas"));
        // A list takes the file name: a usage error with a hint.
        let e = args(&["-config", "x", "m.pas"]).unwrap_err();
        assert!(e.starts_with("no input file (-config took"), "{e}");
        // -b pathname gives the output; it and -nb do not mix with -o.
        let a = args(&["m.pas", "-b", "out"]).unwrap();
        assert_eq!(a.output, Some(PathBuf::from("out.s")));
        assert!(args(&["m.pas", "-b", "out", "-o", "x.s"]).is_err());
        assert!(args(&["m.pas", "-nb", "-o", "x.s"]).is_err());
        assert_eq!(args(&["m.pas", "-nb"]).unwrap().output, None);
        // Upper case is not a reference option: `-IDIR` is `-I` + `DIR`.
        let a = args(&["m.pas", "-IDIR"]).unwrap();
        assert_eq!(a.include_dirs, [PathBuf::from("DIR")]);
        assert!(args(&["m.pas", "-CONFIG", "x"]).is_err());
    }

    #[test]
    fn msgs_report_counts() {
        let v = env!("CARGO_PKG_VERSION");
        let c = |errors, warnings| Counts { errors, warnings }.report();
        assert_eq!(
            c(0, 0),
            format!("No errors, no warnings, no info msgs, dpc {v}")
        );
        assert_eq!(
            c(1, 0),
            format!("1 error, no warnings, no info msgs, dpc {v}")
        );
        assert_eq!(
            c(0, 1),
            format!("No errors, 1 warning, no info msgs, dpc {v}")
        );
        assert_eq!(
            c(3, 2),
            format!("3 errors, 2 warnings, no info msgs, dpc {v}")
        );
    }

    #[test]
    fn parses_merge_constant_stores() {
        assert!(!args(&["m.pas"]).unwrap().merge_constant_stores);
        assert!(
            args(&["--merge-constant-stores", "m.pas"])
                .unwrap()
                .merge_constant_stores
        );
        assert!(args(&[
            "--merge-constant-stores",
            "--merge-constant-stores",
            "m.pas"
        ])
        .is_err());
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
