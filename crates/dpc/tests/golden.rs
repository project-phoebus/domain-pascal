//! Golden tests (CLAUDE.md 6.2; tests/golden/README.md): every
//! `tests/golden/<area>/<name>.pas` is compiled with the `dpc` binary and
//! its output, normalised, must equal `<name>.s`.
//!
//! Normalisation: `|` comments are removed, each line is trimmed, runs of
//! blanks become one space, empty lines are dropped.
//!
//! `DPC_BLESS=1 cargo test --test golden` rewrites the expected files from
//! the compiler's output; a golden change needs a reason in the commit
//! message.

use std::path::{Path, PathBuf};
use std::process::Command;

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden")
}

/// The normalised form of an assembly text.
fn normalise(asm: &str) -> String {
    let mut out = String::new();
    for line in asm.lines() {
        // `|` starts a comment anywhere on an m68k gas line.  No string
        // directives are emitted yet; revisit when they are.
        let code = line.split('|').next().unwrap_or("");
        let words: Vec<&str> = code.split_whitespace().collect();
        if words.is_empty() {
            continue;
        }
        out.push_str(&words.join(" "));
        out.push('\n');
    }
    out
}

fn cases(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            cases(&p, out);
        } else if p.extension().is_some_and(|e| e == "pas") {
            out.push(p);
        }
    }
}

#[test]
fn normalise_strips_comments_and_spacing() {
    assert_eq!(
        normalise("| hdr\n\n\t.text\n\tmove.w\t(8,%fp),%d0  | c\n"),
        ".text\nmove.w (8,%fp),%d0\n"
    );
}

#[test]
fn golden() {
    let mut pas = Vec::new();
    cases(&golden_dir(), &mut pas);
    assert!(!pas.is_empty(), "no golden tests found");
    let bless = std::env::var_os("DPC_BLESS").is_some();
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("golden");
    std::fs::create_dir_all(&scratch).unwrap();
    let mut failures = Vec::new();
    for src in &pas {
        let expected_path = src.with_extension("s");
        let name = src
            .strip_prefix(golden_dir())
            .unwrap()
            .display()
            .to_string();
        let out_path = scratch
            .join(name.replace(['/', '\\'], "_"))
            .with_extension("s");
        let run = Command::new(env!("CARGO_BIN_EXE_dpc"))
            .arg("-o")
            .arg(&out_path)
            .arg(src)
            .output()
            .unwrap();
        if !run.status.success() {
            failures.push(format!(
                "{name}: dpc failed:\n{}",
                String::from_utf8_lossy(&run.stderr)
            ));
            continue;
        }
        let got = normalise(&std::fs::read_to_string(&out_path).unwrap());
        if bless {
            std::fs::write(&expected_path, &got).unwrap();
            continue;
        }
        let want = std::fs::read_to_string(&expected_path)
            .map(|s| normalise(&s))
            .unwrap_or_else(|e| format!("<cannot read {}: {e}>", expected_path.display()));
        if got != want {
            failures.push(format!("{name}:\n--- expected\n{want}--- got\n{got}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
