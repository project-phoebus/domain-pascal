//! Integration tests of the reference's compiler options (ref 6.4,
//! Table 6-2, pp. 6-6..6-8; `crates/dpc/src/refopts.rs`,
//! `docs/compiler-options.md`): run the `dpc` binary.

use std::path::PathBuf;
use std::process::Command;

fn dpc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dpc"))
}

/// A fresh scratch directory under the cargo target dir.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("options")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct Run {
    code: Option<i32>,
    stderr: String,
    /// The output file `m.s`, if written.
    asm: Option<String>,
}

/// Compile `src` as `m.pas` in a scratch directory: `dpc m.pas ARGS`, the
/// source first as in the reference's `pas source [options]` (6.3,
/// p. 6-4); the output is `m.s` there unless ARGS say otherwise.
fn run(name: &str, src: &str, args: &[&str]) -> Run {
    let dir = scratch(name);
    std::fs::write(dir.join("m.pas"), src).unwrap();
    let out = dpc()
        .current_dir(&dir)
        .arg("m.pas")
        .args(args)
        .output()
        .unwrap();
    Run {
        code: out.status.code(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        asm: std::fs::read_to_string(dir.join("m.s")).ok(),
    }
}

/// The reference's %error example (4.7 "%error", p. 4-51), in a module:
/// an error with `-config mono`.
const ERROR_EXAMPLE: &str = "module m;
%VAR color mono
%IF color %THEN
  {Code for color node.}
  procedure p; begin end;
%ELSEIF mono %THEN
   %ERROR 'I have not finished the code for a monochromatic node.'
%ENDIF
";

/// The reference's %warning example (4.7 "%warning", p. 4-52), in a
/// module: a warning without `-config color`.  The page prints the string
/// without its closing quote (an erratum); it is closed here.
const WARNING_EXAMPLE: &str = "module m;
%VAR color mono
%IF color %THEN
  {Code for color node.}
  procedure p; begin end;
%ELSE
   %WARNING 'You forgot to use the -CONFIG color option.'
%ENDIF
";

#[test]
fn config_reference_error_example() {
    let r = run("config_error_none", ERROR_EXAMPLE, &[]);
    assert_eq!(r.code, Some(0), "{}", r.stderr);
    assert_eq!(r.stderr, "");
    let asm = r.asm.unwrap();
    assert!(!asm.contains("P:"), "{asm}");
    let r = run("config_error_color", ERROR_EXAMPLE, &["-config", "color"]);
    assert_eq!(r.code, Some(0), "{}", r.stderr);
    assert!(r.asm.unwrap().contains("P:"));
    // "If you do compile with the -config mono option, Domain Pascal prints
    // out the following error message ... does not create an executable
    // object" (p. 4-51).
    let r = run("config_error_mono", ERROR_EXAMPLE, &["-config", "mono"]);
    assert_eq!(r.code, Some(1));
    assert!(
        r.stderr.starts_with(
            "m.pas:7:4: error: I have not finished the code for a monochromatic node."
        ),
        "{}",
        r.stderr
    );
    assert!(r.asm.is_none());
}

#[test]
fn config_reference_warning_example() {
    // "if you don't compile with the -config color option, Domain Pascal
    // prints out [the warning] ... A warning does not prevent the compiler
    // from creating an executable object" (p. 4-52).
    let r = run("config_warning_none", WARNING_EXAMPLE, &[]);
    assert_eq!(r.code, Some(0), "{}", r.stderr);
    assert!(
        r.stderr
            .starts_with("m.pas:7:4: warning: You forgot to use the -CONFIG color option."),
        "{}",
        r.stderr
    );
    assert!(r.asm.is_some());
    let r = run("config_warning_mono", WARNING_EXAMPLE, &["-config", "mono"]);
    assert!(r.stderr.contains("warning: You forgot"), "{}", r.stderr);
    let r = run(
        "config_warning_color",
        WARNING_EXAMPLE,
        &["-config", "COLOR"],
    );
    assert_eq!(r.code, Some(0), "{}", r.stderr);
    assert_eq!(r.stderr, "");
    assert!(r.asm.unwrap().contains("P:"));
    // With -nwarn the warning is not shown (ref 6.4.30, p. 6-35), but
    // -msgs still counts it.
    let r = run(
        "config_warning_nwarn",
        WARNING_EXAMPLE,
        &["-nwarn", "-msgs"],
    );
    assert_eq!(r.code, Some(0), "{}", r.stderr);
    assert_eq!(
        r.stderr,
        format!(
            "No errors, 1 warning, no info msgs, dpc {}\n",
            env!("CARGO_PKG_VERSION")
        )
    );
}

#[test]
fn config_several_names_and_percent_config() {
    // Figure 6-2 (p. 6-13) run as "-config first third" (p. 6-14), its
    // statements replaced by routines (no I/O in dpc); its %VAR list with
    // commas (owner, 2026-10-20).
    let src = "module m;
%VAR first, second, third
%IF first %THEN procedure x5; begin end; %ENDIF
%IF second %THEN procedure y10; begin end; %ENDIF
%IF third %THEN procedure z15; begin end; %ENDIF
%IF %config %THEN procedure configured; begin end; %ENDIF
";
    let asm = run("config_none", src, &[]).asm.unwrap();
    assert!(!asm.contains("X5:") && !asm.contains("Z15:") && !asm.contains("CONFIGURED:"));
    let r = run("config_first_third", src, &["-config", "first", "third"]);
    assert_eq!(r.code, Some(0), "{}", r.stderr);
    let asm = r.asm.unwrap();
    assert!(
        asm.contains("X5:") && !asm.contains("Y10:") && asm.contains("Z15:"),
        "{asm}"
    );
    assert!(asm.contains("CONFIGURED:"));
    // The list ends at the next option; `--` ends the options.
    let dir = scratch("config_dashdash");
    std::fs::write(dir.join("m.pas"), src).unwrap();
    let out = dpc()
        .current_dir(&dir)
        .args(["-config", "second", "--", "m.pas"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let asm = std::fs::read_to_string(dir.join("m.s")).unwrap();
    assert!(asm.contains("Y10:") && !asm.contains("X5:"));
    // Without `--` the file name is taken as a name of the list.
    let out = dpc()
        .current_dir(&dir)
        .args(["-config", "second", "m.pas"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("no input file (-config took the arguments after it"),
        "{err}"
    );
}

#[test]
fn config_errors() {
    let src = "module m;\n%var a\n";
    // p. 4-50: set to true more than once.
    let r = run("config_twice", src, &["-config", "a", "A"]);
    assert_eq!(r.code, Some(1));
    assert!(
        r.stderr
            .contains("`A` is set to true more than once by -config"),
        "{}",
        r.stderr
    );
    let r = run(
        "config_enable",
        "module m;\n%var a\n%enable a\n",
        &["-config", "a"],
    );
    assert_eq!(r.code, Some(1));
    assert!(r.stderr.contains("already set to true"), "{}", r.stderr);
    // p. 4-48: not a predeclared variable.
    let r = run("config_predeclared", src, &["-config", "_ISP__M68K"]);
    assert_eq!(r.code, Some(1));
    assert!(
        r.stderr.contains("cannot be set with -config"),
        "{}",
        r.stderr
    );
    // A usage error: no names, or not a name.
    let r = run("config_empty", src, &["-config"]);
    assert_eq!(r.code, Some(2));
    assert!(
        r.stderr.contains("-config needs at least one"),
        "{}",
        r.stderr
    );
    let r = run("config_bad", src, &["-config", "a-b"]);
    assert_eq!(r.code, Some(2));
}

#[test]
fn cond_compiles_debug_lines() {
    // ref 6.4.7, p. 6-12; %debug, pp. 4-54..4-55.
    let src = "module m;\n%debug; procedure dbg; begin end;\nprocedure p; begin end;\n";
    let asm = run("ncond", src, &[]).asm.unwrap();
    assert!(!asm.contains("DBG:") && asm.contains("P:"));
    let asm = run("cond", src, &["-cond"]).asm.unwrap();
    assert!(asm.contains("DBG:") && asm.contains("P:"));
    assert_eq!(
        run("ncond2", src, &["-ncond"]).asm,
        run("ncond3", src, &[]).asm
    );
}

#[test]
fn idir_and_dash_i_search_in_order() {
    let dir = scratch("idir");
    for (d, name) in [("a", "pa"), ("b", "pb")] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
        std::fs::write(
            dir.join(d).join("x.ins.pas"),
            format!("procedure {name}; begin end;\n"),
        )
        .unwrap();
    }
    std::fs::write(dir.join("m.pas"), "module m;\n%include 'x.ins.pas';\n").unwrap();
    let asm = |args: &[&str]| {
        let out = dpc()
            .current_dir(&dir)
            .arg("m.pas")
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        std::fs::read_to_string(dir.join("m.s")).unwrap()
    };
    assert!(asm(&["-idir", "a", "b"]).contains("PA:"));
    assert!(asm(&["-idir", "b", "a"]).contains("PB:"));
    assert!(asm(&["-I", "b", "-idir", "a"]).contains("PB:"));
    assert!(asm(&["-idir", "a", "-Ib"]).contains("PA:"));
}

#[test]
fn natural_is_the_directive_at_the_start() {
    // ref 6.4.22, p. 6-26: "-natural has the same effect as the
    // %natural_alignment compiler directive."
    let src = "module m;
type r = record c: char; i: integer32; end;
var v: r;
procedure p(var x: r); begin x.i := 1; end;
";
    let plain = run("nnatural", src, &[]).asm.unwrap();
    let natural = run("natural", src, &["-natural"]).asm.unwrap();
    let directive = run("directive", &format!("%natural_alignment;\n{src}"), &[])
        .asm
        .unwrap();
    assert_ne!(plain, natural);
    assert_eq!(natural, directive);
    assert_eq!(run("nnatural2", src, &["-nnatural"]).asm.unwrap(), plain);
}

#[test]
fn binary_options() {
    let src = "module m;\nprocedure p; begin end;\n";
    // -nb: compiled, nothing written (ref 6.4.3, p. 6-9).
    let r = run("nb", src, &["-nb"]);
    assert_eq!(r.code, Some(0), "{}", r.stderr);
    assert!(r.asm.is_none());
    // ... but errors are still found.
    let r = run(
        "nb_error",
        "module m;\nprocedure p; begin x := 1; end;\n",
        &["-nb"],
    );
    assert_eq!(r.code, Some(1));
    // -b pathname: pathname.s (p. 6-10's `pathname.bin`).
    let dir = scratch("b_path");
    std::fs::write(dir.join("m.pas"), src).unwrap();
    for (arg, file) in [("jest", "jest.s"), ("other.s", "other.s")] {
        let out = dpc()
            .current_dir(&dir)
            .args(["m.pas", "-b", arg])
            .output()
            .unwrap();
        assert!(out.status.success());
        assert!(dir.join(file).is_file(), "{file}");
    }
    let r = run("b_o", src, &["-b", "x", "-o", "y.s"]);
    assert_eq!(r.code, Some(2));
    let r = run("b_default", src, &["-b"]);
    assert!(r.asm.is_some());
}

#[test]
fn comchk_warns_of_unbalanced_comments() {
    // ref 6.4.5, p. 6-11.
    let src = "module m;\n{This comment should be closed, but I forgot to do it!\n\
               procedure p; {We need this statement. }\nprocedure q; begin end;\n";
    let r = run("ncomchk", src, &[]);
    assert_eq!(r.stderr, "");
    let r = run("comchk", src, &["-comchk"]);
    assert_eq!(r.code, Some(0));
    assert!(
        r.stderr.starts_with(
            "m.pas:3:14: warning: Unbalanced comment; another comment start found before end."
        ),
        "{}",
        r.stderr
    );
}

#[test]
fn msgs_report() {
    let ok = "module m;\n";
    let r = run("msgs", ok, &["-msgs"]);
    let v = env!("CARGO_PKG_VERSION");
    assert_eq!(
        r.stderr,
        format!("No errors, no warnings, no info msgs, dpc {v}\n")
    );
    let r = run(
        "msgs_err",
        "module m;\nprocedure p; begin x := 1; y := 2; end;\n",
        &["-msgs"],
    );
    assert!(
        r.stderr
            .ends_with(&format!("2 errors, no warnings, no info msgs, dpc {v}\n")),
        "{}",
        r.stderr
    );
    assert_eq!(run("nmsgs", ok, &["-nmsgs"]).stderr, "");
}

#[test]
fn version() {
    // The version and the commit the binary was built from (build.rs);
    // `--version` is the same.
    for flag in ["-version", "--version"] {
        let out = dpc().arg(flag).output().unwrap();
        assert!(out.status.success());
        let s = String::from_utf8_lossy(&out.stdout).into_owned();
        let head = format!("dpc {} (commit ", env!("CARGO_PKG_VERSION"));
        assert!(s.starts_with(&head) && s.ends_with(")\n"), "{flag}: {s}");
        let commit = &s[head.len()..s.len() - 2];
        assert!(!commit.is_empty() && !commit.contains(' '), "{flag}: {s}");
    }
    // "do not include the filename" (ref 6.4.29, p. 6-35).
    let r = run("version_file", "module m;\n", &["-version"]);
    assert_eq!(r.code, Some(2));
    assert!(r.stderr.contains("-version is used alone"), "{}", r.stderr);
}

#[test]
fn accepted_noted_and_refused() {
    let src = "module m;\nprocedure p; begin end;\n";
    let base = run("base", src, &[]).asm.unwrap();
    // Defaults and options without effect on dpc: same output, silent.
    let r = run(
        "silent",
        src,
        &[
            "-ac",
            "-bounds_violation",
            "-ncomchk",
            "-compress",
            "-ncond",
            "-ndb",
            "-nexp",
            "-nfrnd",
            "-nimap",
            "-nindexl",
            "-ninfo",
            "-niso",
            "-nl",
            "-nmap",
            "-nmsgs",
            "-nnatural",
            "-nclines",
            "-opt",
            "3",
            "-nprasm",
            "-nstd",
            "-nsubchk",
            "-warn",
            "-xrs",
            "-cpu",
            "mathlib_sr10",
        ],
    );
    assert_eq!(r.code, Some(0), "{}", r.stderr);
    assert_eq!(r.stderr, "");
    assert_eq!(r.asm.unwrap(), base);
    // Accepted with a note each: same output.
    let r = run(
        "noted",
        src,
        &[
            "-map", "-exp", "-l", "-dbs", "-alnchk", "-std", "-info", "4", "-cpu", "mathchip",
        ],
    );
    assert_eq!(r.code, Some(0), "{}", r.stderr);
    assert_eq!(r.stderr.lines().count(), 8, "{}", r.stderr);
    assert!(
        r.stderr.lines().all(|l| l.starts_with("dpc: note: -")),
        "{}",
        r.stderr
    );
    assert_eq!(r.asm.unwrap(), base);
    // Refused: a usage error, nothing written.
    for args in [
        &["-subchk"][..],
        &["-opt", "0"],
        &["-pic"],
        &["-iso"],
        &["-cpu", "a88k"],
        &["-dba"],
        &["-nxrs"],
    ] {
        let r = run("refused", src, args);
        assert_eq!(r.code, Some(2), "{args:?}");
        assert!(r.stderr.contains("is not supported"), "{}", r.stderr);
        assert!(r.asm.is_none());
    }
    // A pair given twice.
    let r = run("pair", src, &["-warn", "-nwarn"]);
    assert_eq!(r.code, Some(2));
    assert!(
        r.stderr.contains("-nwarn conflicts with -warn"),
        "{}",
        r.stderr
    );
    // dpc's own long options still work beside them.
    let r = run(
        "mixed",
        src,
        &["--strict", "-config", "x", "--loop-align=off"],
    );
    assert_eq!(r.code, Some(0), "{}", r.stderr);
    // `x` is declared by no %var: a warning naming it (owner, 2026-10-20).
    assert_eq!(
        r.stderr,
        "dpc: warning: -config variable `x` is not declared with %var\n"
    );
    assert!(r.asm.unwrap().contains("0x0000"), "--loop-align=off taken");
}
