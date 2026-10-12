//! `dpc`: the Domain Pascal compiler driver.
//!
//! usage: dpc [-I dir]... [-o out.s] [--fragment-origin 0xADDR [--fragment-ends-module]]
//!            [--loop-align=on|off | --loop-align-off-from=ROUTINE]
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

use std::path::PathBuf;
use std::process::ExitCode;

use dpc_m68k::emit::LoopAlign;
use dpc_syntax::{preprocess, Diagnostic, SourceMap};

const PROG: &str = "dpc";
const USAGE: &str = "usage: dpc [-I dir]... [-o out.s] \
                     [--fragment-origin 0xADDR [--fragment-ends-module]] \
                     [--loop-align=on|off | --loop-align-off-from=ROUTINE] \
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
                    (_, "") => return Err("--loop-align-off-from needs a routine name".into()),
                    _ => LoopAlign::OffFrom(v.to_string()),
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
    Ok(Args {
        include_dirs,
        output,
        input,
        stop,
        fragment_origin,
        fragment_ends_module,
        loop_align: loop_align.unwrap_or_default(),
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
    let module = dpc_ir::lower(&program)?;
    let opts = dpc_m68k::emit::Options {
        fragment_origin: args.fragment_origin,
        fragment_ends_module: args.fragment_ends_module,
        loop_align: args.loop_align.clone(),
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
        eprintln!("{}", d.render(&sm, PROG));
    }
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(diags) => {
            for d in &diags {
                eprintln!("{}", d.render(&sm, PROG));
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
            LoopAlign::OffFrom("vtoc_$dismount".into())
        );
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
    }
}
