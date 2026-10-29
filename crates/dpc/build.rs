//! Records the commit dpc is built from, for `dpc --version` / `-version`.
//! `make stable` builds from a `git archive` export (no .git) and passes the
//! commit in `DPC_COMMIT`; otherwise it is asked of git, or "unknown".

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=DPC_COMMIT");
    let dir = env!("CARGO_MANIFEST_DIR");
    let git = |args: &[&str]| -> Option<String> {
        let out = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
        (!s.is_empty()).then_some(s)
    };
    // Rebuild when HEAD moves (a commit or a checkout).  `--git-path`
    // resolves each file where git keeps it, which in a worktree is the
    // common directory for refs (a path under the worktree's own git
    // directory would not exist and make every build dirty).
    let watch = |rel: &str| {
        // Only files that exist: cargo counts a missing one as changed.
        if let Some(p) = git(&["rev-parse", "--path-format=absolute", "--git-path", rel]) {
            if std::path::Path::new(&p).exists() {
                println!("cargo:rerun-if-changed={p}");
            }
        }
    };
    watch("HEAD");
    watch("packed-refs");
    if let Some(r) = git(&["symbolic-ref", "-q", "HEAD"]) {
        watch(&r);
    }
    let commit = std::env::var("DPC_COMMIT")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| git(&["rev-parse", "HEAD"]))
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=DPC_COMMIT={commit}");
}
