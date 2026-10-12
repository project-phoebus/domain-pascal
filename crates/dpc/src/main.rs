//! `dpc`: the Domain Pascal compiler driver.
//!
//! usage: dpc [-I dir]... [-o out.s] [--parse-only | --dump-ast | --check-only] file.pas
//!
//! `--parse-only` stops after parsing (syntax errors are reported with
//! their positions); `--dump-ast` also prints the AST to stdout;
//! `--check-only` stops after semantic analysis.

use std::path::PathBuf;
use std::process::ExitCode;

use dpc_syntax::{preprocess, Diagnostic, SourceMap};

const PROG: &str = "dpc";
const USAGE: &str =
    "usage: dpc [-I dir]... [-o out.s] [--parse-only | --dump-ast | --check-only] file.pas";

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
}

fn parse_args(mut argv: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut include_dirs = Vec::new();
    let mut output = None;
    let mut input = None;
    let mut stop = Stop::Never;
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "-I" => include_dirs.push(argv.next().ok_or("-I needs a directory")?.into()),
            "-o" => {
                if output.is_some() {
                    return Err("-o given more than once".into());
                }
                output = Some(argv.next().ok_or("-o needs a file name")?.into());
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
    Ok(Args {
        include_dirs,
        output,
        input,
        stop,
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
    let module = dpc_ir::lower(&program);
    let asm = dpc_m68k::emit::emit(&module, &args.input.display().to_string());

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
    }
}
