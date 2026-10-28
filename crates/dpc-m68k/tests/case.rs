//! `case` statements end to end from Pascal source (ref "Case", pp.
//! 4-36..4-37; codegen-patterns 7.3): the jump table and the
//! compare chain, the choice between them, the arms, and what is reported
//! as not supported yet.

use dpc_m68k::emit::{emit, Options};
use dpc_syntax::preprocess::{preprocess, Options as PpOptions};
use dpc_syntax::SourceMap;

/// Compile `src` to assembly lines without comments and blank lines, or
/// the rendered diagnostics.
fn compile(src: &str) -> Result<Vec<String>, Vec<String>> {
    let mut sm = SourceMap::new();
    let f = sm.add("t.pas", src.to_string());
    let pp = preprocess(&mut sm, f, &PpOptions::default());
    assert!(!pp.has_errors(), "preprocessor errors");
    let unit = dpc_syntax::parser::parse(&pp.tokens).expect("parses");
    let render = |ds: Vec<dpc_syntax::Diagnostic>| -> Vec<String> {
        ds.iter().map(|d| d.render(&sm, "dpc")).collect()
    };
    let program = dpc_sema::analyze(&unit, &dpc_m68k::M68k).map_err(render)?;
    let module = dpc_ir::lower(&program).map_err(render)?;
    let asm = emit(&module, "t.pas", &Options::default()).map_err(render)?;
    Ok(asm
        .lines()
        .map(|l| l.split('|').next().unwrap().trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

const DECLS: &str = "module m;
var
  w, x, y: extern integer;
  l: extern integer32;
  c: extern char;
  b: extern boolean;
function f: integer; options(val_param, extern);
procedure ext_$p(x: integer); options(val_param, extern);
procedure ext_$q(x: integer32); options(val_param, extern);
";

/// The body lines of routine `t`, after its `link`, up to its `unlk`.
fn body(routine: &str) -> Vec<String> {
    let lines = compile(&format!("{DECLS}{routine}")).unwrap();
    let start = lines.iter().position(|l| l == "T:").unwrap() + 2;
    let end = start
        + lines[start..]
            .iter()
            .position(|l| l.starts_with("unlk"))
            .unwrap();
    lines[start..end].to_vec()
}

/// The body of `procedure t` whose statement part is `stmts`.
fn proc_body(stmts: &str) -> Vec<String> {
    body(&format!("procedure t; internal;\nbegin\n{stmts}\nend;\n"))
}

fn error(stmts: &str) -> String {
    let e = compile(&format!(
        "{DECLS}procedure t; internal;\nbegin\n{stmts}\nend;\n"
    ))
    .unwrap_err();
    assert_eq!(e.len(), 1, "{e:?}");
    e[0].clone()
}

/// OSINFO_$GET_MMAP 0xE5C6C0..0xE5C798: labels 0..4, no `otherwise`:
/// `cmpi.w #5; bcc` to the join, the table right after the `jmp`, arms in
/// source order (label 4's before label 3's), each ending in `bra` to the
/// join but the last, which falls through.
#[test]
fn jump_table_without_otherwise() {
    let b = proc_body(
        "case w of
  0: x := 1;
  1: x := 2;
  2: x := 3;
  4: y := 0;
  3: y := 1;
end;",
    );
    assert_eq!(
        b,
        [
            "move.w\t(W).l,%d0",
            "cmpi.w\t#5,%d0",
            "bcc.s\t1f",
            "add.w\t%d0,%d0",
            "move.w\t(7f,%pc,%d0.w),%d0",
            "jmp\t(7f,%pc,%d0.w)",
            "7:",
            ".short\t2f-7b",
            ".short\t3f-7b",
            ".short\t4f-7b",
            ".short\t6f-7b",
            ".short\t5f-7b",
            "2:",
            "move.w\t#1,(X).l",
            "bra.s\t1f",
            "3:",
            "move.w\t#2,(X).l",
            "bra.s\t1f",
            "4:",
            "move.w\t#3,(X).l",
            "bra.s\t1f",
            "5:",
            "clr.w\t(Y).l",
            "bra.s\t1f",
            "6:",
            "move.w\t#1,(Y).l",
            "1:",
        ]
    );
}

/// PROC1_$CREATE_P 0xE15176..0xE151AE: labels from 3, `subq.w #3`; a
/// missing label (6, 7) and the `bcc` go to the `otherwise` part, which
/// follows the arms and falls through to the join.  An empty arm's label
/// goes to the join (KBD_$RCV 0xE1CCFE).
#[test]
fn jump_table_with_otherwise_and_lower_bound() {
    let b = proc_body(
        "case w of
  3, 4, 5, 10: x := 5;
  8: x := 6;
  9: ;
  otherwise x := 0;
end;",
    );
    assert_eq!(
        b,
        [
            "move.w\t(W).l,%d0",
            "subq.w\t#3,%d0",
            "cmpi.w\t#8,%d0",
            "bcc.s\t4f",
            "add.w\t%d0,%d0",
            "move.w\t(5f,%pc,%d0.w),%d0",
            "jmp\t(5f,%pc,%d0.w)",
            "5:",
            ".short\t2f-5b",
            ".short\t2f-5b",
            ".short\t2f-5b",
            ".short\t4f-5b",
            ".short\t4f-5b",
            ".short\t3f-5b",
            ".short\t1f-5b",
            ".short\t2f-5b",
            "2:",
            "move.w\t#5,(X).l",
            "bra.s\t1f",
            "3:",
            "move.w\t#6,(X).l",
            "bra.s\t1f",
            "4:",
            "clr.w\t(X).l",
            "1:",
        ]
    );
}

/// The other lower-bound forms: `subi.w #100` (ACL_$SERVER 0xE495CE, a
/// `char` selector widened with `clr.w; move.b`), `addq.w #4` for -4
/// (MST_$PRIV_SET_TOUCH_AHEAD_CNT 0xE44534); a longword selector with
/// `cmpi.l` and a word `add.w` (ASKNODE_$INTERNET_INFO 0xE64F92).
#[test]
fn jump_table_bounds() {
    let b = proc_body(
        "case c of
  'd', 'f', 'h', 'j': x := 1;
  'l': x := 2;
end;",
    );
    assert_eq!(
        b[..6],
        [
            "clr.w\t%d0",
            "move.b\t(C).l,%d0",
            "subi.w\t#0x64,%d0",
            "cmpi.w\t#9,%d0",
            "bcc.s\t1f",
            "add.w\t%d0,%d0",
        ]
    );
    let b = proc_body(
        "case w of
  -4, -3: x := 1;
  -2, -1: x := 2;
end;",
    );
    assert_eq!(
        b[..3],
        ["move.w\t(W).l,%d0", "addq.w\t#4,%d0", "cmpi.w\t#4,%d0"]
    );
    let b = proc_body(
        "case l of
  0, 1: x := 1;
  2, 3: x := 2;
end;",
    );
    assert_eq!(
        b[..4],
        [
            "move.l\t(L).l,%d0",
            "cmpi.l\t#4,%d0",
            "bcc.s\t1f",
            "add.w\t%d0,%d0",
        ]
    );
}

/// SYSBUS_$INIT 0xE0AB64..0xE0ABAA: three labels, dense, as a compare
/// chain; label 0 first is the `beq` on the flags of the load; `bra` to
/// the join after the chain; the last arm falls through.
#[test]
fn compare_chain_of_three_labels() {
    let b = proc_body(
        "case w of
  0: x := 1;
  1: x := 2;
  2: x := 3;
end;",
    );
    assert_eq!(
        b,
        [
            "move.w\t(W).l,%d0",
            "beq.s\t2f",
            "cmpi.w\t#1,%d0",
            "beq.s\t3f",
            "cmpi.w\t#2,%d0",
            "beq.s\t4f",
            "bra.s\t1f",
            "2:",
            "move.w\t#1,(X).l",
            "bra.s\t1f",
            "3:",
            "move.w\t#2,(X).l",
            "bra.s\t1f",
            "4:",
            "move.w\t#3,(X).l",
            "1:",
        ]
    );
}

/// Module-local 0xE1CC64: a sparse `char` case (8 labels over 48 values),
/// `clr.w D0; move.b ...,D0`, the labels tested in source order, an arm
/// with several labels, `bra` to the `otherwise` part, which follows the
/// arms.  Label 0 after a compare: `tst.w` (TIME_$GET_EC 0xE16762); first
/// after the zero-extension: `tst.w` too (0xE556E6).
#[test]
fn sparse_compare_chain() {
    let b = proc_body(
        "case c of
  chr(16#c0): x := 22;
  chr(16#91): x := 24;
  chr(16#99), chr(16#98), chr(16#97): x := 9;
  chr(0): x := 1;
  otherwise x := 0;
end;",
    );
    assert_eq!(
        b,
        [
            "clr.w\t%d0",
            "move.b\t(C).l,%d0",
            "cmpi.w\t#0xc0,%d0",
            "beq.s\t2f",
            "cmpi.w\t#0x91,%d0",
            "beq.s\t3f",
            "cmpi.w\t#0x99,%d0",
            "beq.s\t4f",
            "cmpi.w\t#0x98,%d0",
            "beq.s\t4f",
            "cmpi.w\t#0x97,%d0",
            "beq.s\t4f",
            "tst.w\t%d0",
            "beq.s\t5f",
            "bra.s\t6f",
            "2:",
            "move.w\t#0x16,(X).l",
            "bra.s\t1f",
            "3:",
            "move.w\t#0x18,(X).l",
            "bra.s\t1f",
            "4:",
            "move.w\t#9,(X).l",
            "bra.s\t1f",
            "5:",
            "move.w\t#1,(X).l",
            "bra.s\t1f",
            "6:",
            "clr.w\t(X).l",
            "1:",
        ]
    );
    let b = proc_body("case c of chr(0): x := 1; 'a': x := 2; end;");
    assert_eq!(
        b[..4],
        ["clr.w\t%d0", "move.b\t(C).l,%d0", "tst.w\t%d0", "beq.s\t2f"]
    );
    // A longword chain (status codes: 0xE5D2FE `cmpi.l`).
    let b = proc_body("case l of 16#f0010: x := 1; 16#f0003: x := 2; end;");
    assert_eq!(
        b[..3],
        ["move.l\t(L).l,%d0", "cmpi.l\t#0xf0010,%d0", "beq.s\t2f"]
    );
}

/// The choice (codegen-patterns 7.3): up to 3 labels a chain
/// however dense; from 4, a table when `7r <= 19n` (r/n <= 2.71), a chain
/// when `3r >= 13n` (r/n >= 4.33), refused between.
#[test]
fn table_or_chain() {
    let is_table = |labels: &str| {
        let b = proc_body(&format!("case w of {labels}: x := 1; 100: x := 2; end;"));
        b.iter().any(|l| l.starts_with("jmp"))
    };
    // 2 labels over 2; 3 over 3.
    assert!(!is_table("101"));
    assert!(!is_table("98, 99"));
    // 4 labels over 4 (dense) and over 10 (2.5).
    assert!(is_table("97, 98, 99"));
    assert!(is_table("91, 92, 93"));
    // 4 labels over 18 (4.5): a chain.
    assert!(!is_table("83, 84, 85"));
    // The boundaries: 7 labels over 19 (module-local 0xE63376, the
    // sparsest table) and 6 over 25 (SAU9 SCSI_$C90_INT 0x3C4A59CE, the
    // densest chain; TTY_$I_SIGNAL 0xE1B832 has 6 over 26); one more or
    // one less in range is in the refused band.
    let case_of = |labels: &str| format!("case w of {labels}: x := 2; end;");
    assert!(proc_body(&case_of("1, 2, 3, 4, 5, 6: x := 1; 19"))
        .iter()
        .any(|l| l.starts_with("jmp")));
    assert!(error(&case_of("1, 2, 3, 4, 5, 6: x := 1; 20")).contains("7 labels over a range of 20"));
    assert!(!proc_body(&case_of("2, 1, 3, 4, 5: x := 1; 25"))
        .iter()
        .any(|l| l.starts_with("jmp")));
    assert!(error(&case_of("2, 1, 3, 4, 5: x := 1; 24")).contains("6 labels over a range of 24"));
    let e = error("case w of 89, 90, 91: x := 1; 100: x := 2; end;");
    assert!(
        e.contains("a `case` of 4 labels over a range of 12 (between the sparsest jump table"),
        "{e}"
    );
}

/// codegen-patterns 1.1: a call at the end of an arm, whose `bra` goes to
/// the join and then the epilogue, is not popped; one followed by more
/// code with a branch or push is.
#[test]
fn pops_in_arms() {
    let b = proc_body(
        "case w of
  0: ext_$p(1);
  1: ext_$q(2);
  2: x := 1;
end;",
    );
    assert_eq!(
        b[7..],
        [
            "2:",
            "subq.l\t#2,%sp",
            "move.w\t#1,-(%sp)",
            "jsr\t(EXT_$P).l",
            "bra.s\t1f",
            "3:",
            "pea\t(2).w",
            "jsr\t(EXT_$Q).l",
            "bra.s\t1f",
            "4:",
            "move.w\t#1,(X).l",
            "1:",
        ]
    );
    let b = proc_body(
        "case w of
  0: ext_$p(1);
  1: x := 2;
  2: x := 1;
end;
ext_$p(3);",
    );
    assert!(b.contains(&"addq.w\t#4,%sp".to_string()), "{b:?}");
}

#[test]
fn unsettled_cases_are_rejected() {
    let has = |stmts: &str, what: &str| {
        let e = error(stmts);
        assert!(e.contains(what), "{e}");
    };
    has("case w of 1: x := 1; end;", "a `case` with one label");
    has(
        "case w of 1, 2: x := 1; end;",
        "a compare-chain `case` whose last label belongs to its first arm",
    );
    has(
        "case w & 7 of 0: x := 1; 1: x := 2; end;",
        "label 0 tested first on a computed `case` selector",
    );
    has(
        "case l of 1, 2: x := 1; 3, 4: x := 2; end;",
        "a jump table on a longword selector whose labels do not start at 0",
    );
    has(
        "case w of -12, -11: x := 1; -10, -9: x := 2; end;",
        "a jump table whose lowest label is below -8",
    );
    has(
        "case w of 1: ext_$p(1); 2: ext_$p(2); end; ext_$p(0);",
        "a `case` whose parts end in calls with the same argument bytes",
    );
    has(
        "case w of 1: ; 2: ; otherwise x := 1; end;",
        "a compare-chain `case` whose arms are all empty",
    );
    has(
        "case w of 1: x := 1; 2: x := 2; otherwise ; end;",
        "an empty `otherwise` part",
    );
    has(
        "case b of true: x := 1; false: x := 2; end;",
        "a signed byte",
    );
    has(
        "case f of 1: x := 1; 2: x := 2; end;",
        "a function call in a `case` selector",
    );
    // An arm of only `return` in a jump table: its entry would name a
    // `bra` (no entry in the image does; only branches are seen threaded,
    // codegen-patterns 7.1).
    has(
        "case w of 0: return; 1: x := 1; 2: x := 2; 3: x := 3; end; x := 3;",
        "a `case` table entry whose target a `bra` follows",
    );
}

/// A by-value parameter loaded at entry, used in an arm (codegen-patterns
/// 12.2): the selector takes the next register, a copy when it is the
/// parameter itself (NETWORK_$SET_SERVICE 0xE0F48E `move.w D2,D0`).
#[test]
fn selector_from_a_held_value() {
    let b = body(
        "procedure t(k: integer); internal;
begin
  x := k;
  case k of
    0: y := 1;
    1: y := k;
    2: y := 3;
  end;
end;
",
    );
    assert_eq!(
        b[..5],
        [
            "move.w\t(8,%fp),%d0",
            "move.w\t%d0,(X).l",
            "move.w\t%d0,%d1",
            "beq.s\t2f",
            "cmpi.w\t#1,%d1",
        ]
    );
}
