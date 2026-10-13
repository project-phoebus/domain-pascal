//! `dpc`: the Domain Pascal compiler driver.
//!
//! usage: dpc [-I dir]... [-o out.s] [--fragment-origin 0xADDR [--fragment-ends-module]]
//!            [--loop-align=on|off | --loop-align-off-from=ROUTINE,...]
//!            [--a5-always-from=ROUTINE,...] [--a5-always=ROUTINE,...]
//!            [--a5-absolute=ROUTINE,...] [--module-routine=ROUTINE=0xADDR]...
//!            [--schedule=off|r11|r11:N] [--data-base=SECTION=SYMBOL]...
//!            [--parse-only | --dump-ast | --check-only] file.pas
//!
//! `--parse-only` stops after parsing (syntax errors are reported with
//! their positions); `--dump-ast` also prints the AST to stdout;
//! `--check-only` stops after semantic analysis.
//!
//! `--fragment-origin 0xADDR` (for image-conformance tests) says that the
//! output is a fragment of a module whose first byte belongs at image
//! address ADDR (even): 4-byte alignment is made absolute and no
//! module-end padding is emitted (`dpc_m68k::emit::Options`).  With
//! `--fragment-ends-module` the fragment is the tail of its module and
//! does get the module-end padding.
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
//! `--module-routine=R=0xADDR` (with `--fragment-origin`; repeatable): R,
//! declared `extern` in the fragment, is a routine of the same module
//! outside the fragment, at image address ADDR.  The original compiled
//! the whole module and reached it with `bsr` (codegen-patterns 1.6:
//! DISK_$UNASSIGN_ALL 0xE6BE34 `bsr.w DISK_$UNASSIGN`); the fragment does
//! too, sized from ADDR (`dpc_m68k::emit::Options::module_routines`).
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
use dpc_m68k::emit::LoopAlign;
use dpc_m68k::sched::Schedule;
use dpc_syntax::{preprocess, Diagnostic, SourceMap};

const PROG: &str = "dpc";
const USAGE: &str = "usage: dpc [-I dir]... [-o out.s] \
                     [--fragment-origin 0xADDR [--fragment-ends-module]] \
                     [--loop-align=on|off | --loop-align-off-from=ROUTINE,...] \
                     [--a5-always-from=ROUTINE,...] [--a5-always=ROUTINE,...] \
                     [--a5-absolute=ROUTINE,...] [--module-routine=ROUTINE=0xADDR]... \
                     [--schedule=off|r11|r11:N] [--data-base=SECTION=SYMBOL]... \
                     [--parse-only | --dump-ast | --check-only] file.pas";

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
}

#[derive(Debug)]
struct Args {
    include_dirs: Vec<PathBuf>,
    output: Option<PathBuf>,
    input: PathBuf,
    stop: Stop,
    fragment_origin: Option<u32>,
    fragment_ends_module: bool,
    loop_align: LoopAlign,
    data_base: DataBaseOptions,
    module_routines: Vec<(String, u32)>,
    schedule: Schedule,
    data_bases: Vec<(String, String)>,
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

fn parse_args(mut argv: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut include_dirs = Vec::new();
    let mut output = None;
    let mut input = None;
    let mut stop = Stop::Never;
    let mut fragment_origin = None;
    let mut fragment_ends_module = false;
    let mut loop_align = None;
    let mut always_from = None;
    let mut always = None;
    let mut absolute = None;
    let mut module_routines: Vec<(String, u32)> = Vec::new();
    let mut schedule = None;
    let mut data_bases: Vec<(String, String)> = Vec::new();
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "-I" => include_dirs.push(argv.next().ok_or("-I needs a directory")?.into()),
            "-o" => {
                if output.is_some() {
                    return Err("-o given more than once".into());
                }
                output = Some(argv.next().ok_or("-o needs a file name")?.into());
            }
            "--fragment-origin" => {
                let v = argv.next().ok_or("--fragment-origin needs an address")?;
                let a = parse_number(&v)
                    .ok_or_else(|| format!("--fragment-origin: bad address `{v}`"))?;
                // code is at even addresses
                if a % 2 != 0 {
                    return Err(format!("--fragment-origin: odd address `{v}`"));
                }
                fragment_origin = Some(a);
            }
            "--fragment-ends-module" => fragment_ends_module = true,
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
                let bad = || format!("--module-routine: want ROUTINE=0xADDR, got `{v}`");
                let (name, a) = v.split_once('=').ok_or_else(bad)?;
                let a = parse_number(a).ok_or_else(bad)?;
                if name.is_empty() || a % 2 != 0 {
                    return Err(bad());
                }
                if module_routines
                    .iter()
                    .any(|(n, _)| n.eq_ignore_ascii_case(name))
                {
                    return Err(format!("--module-routine: `{name}` given more than once"));
                }
                module_routines.push((name.to_string(), a));
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
    if fragment_ends_module && fragment_origin.is_none() {
        return Err("--fragment-ends-module needs --fragment-origin".into());
    }
    if !module_routines.is_empty() && fragment_origin.is_none() {
        return Err("--module-routine needs --fragment-origin".into());
    }
    Ok(Args {
        include_dirs,
        output,
        input,
        stop,
        fragment_origin,
        fragment_ends_module,
        loop_align: loop_align.unwrap_or_default(),
        data_base: DataBaseOptions {
            always_from: always_from.unwrap_or_default(),
            always: always.unwrap_or_default(),
            absolute: absolute.unwrap_or_default(),
        },
        module_routines,
        schedule: schedule.unwrap_or_default(),
        data_bases,
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
    let opts = preprocess::Options {
        include_dirs: args.include_dirs.clone(),
        ..Default::default()
    };
    let pp = preprocess::preprocess(sm, file, &opts);
    if pp.has_errors() {
        return Err(pp.diagnostics);
    }
    warnings.extend(pp.diagnostics);
    let unit = dpc_syntax::parser::parse(&pp.tokens)?;
    match args.stop {
        Stop::Never | Stop::AfterCheck => {}
        Stop::AfterParse => return Ok(()),
        Stop::DumpAst => {
            println!("{unit:#?}");
            return Ok(());
        }
    }
    let program = dpc_sema::analyze(&unit, &dpc_m68k::M68k)?;
    warnings.extend(program.warnings.iter().cloned());
    if args.stop == Stop::AfterCheck {
        return Ok(());
    }
    let module = dpc_ir::lower_with(&program, &args.data_base)?;
    let opts = dpc_m68k::emit::Options {
        fragment_origin: args.fragment_origin,
        fragment_ends_module: args.fragment_ends_module,
        loop_align: args.loop_align.clone(),
        module_routines: args.module_routines.clone(),
        schedule: args.schedule,
        data_bases: args.data_bases.clone(),
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
    let mut sm = SourceMap::new();
    let mut warnings = Vec::new();
    let result = compile(&args, &mut sm, &mut warnings);
    for d in &warnings {
        eprintln!("{}", d.render_with_source(&sm, PROG));
    }
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(diags) => {
            for d in &diags {
                eprintln!("{}", d.render_with_source(&sm, PROG));
            }
            ExitCode::FAILURE
        }
    }
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
        assert_eq!(a.fragment_origin, None);
        let a = args(&["--fragment-origin", "0xE148CA", "m.pas"]).unwrap();
        assert_eq!(a.fragment_origin, Some(0xE148CA));
        let a = args(&["--fragment-origin", "16", "m.pas"]).unwrap();
        assert_eq!(a.fragment_origin, Some(16));
        assert!(!a.fragment_ends_module);
        let a = args(&[
            "--fragment-origin",
            "0xE128F6",
            "--fragment-ends-module",
            "m.pas",
        ]);
        assert!(a.unwrap().fragment_ends_module);
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
            vec![("DISK_$UNASSIGN".into(), 0xE6BDA8), ("X".into(), 16)]
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
        assert!(args(&["--a5-absolute=P", "--a5-absolute=Q", "m.pas"]).is_err());
        assert!(args(&["--a5-always-from", "P", "m.pas"]).is_err());
        assert!(args(&["--data-base=", "m.pas"]).is_err());
        assert!(args(&["--data-base=WIRED_DATA", "m.pas"]).is_err());
        assert!(args(&["--data-base==PROC1_$DATA", "m.pas"]).is_err());
        assert!(args(&["--data-base=W=", "m.pas"]).is_err());
        assert!(args(&["--data-base=W=A=B", "m.pas"]).is_err());
        assert!(args(&["--data-base=w=A", "--data-base=W=B", "m.pas"]).is_err());
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
}
