//! Sets (ref 3.9.3; "Set Operations", pp. 4-173..4-176; codegen-patterns
//! 5.1), end to end from Pascal source: the instruction forms, the
//! refusals, and the meaning of the code.  The last is checked by running
//! the selected instructions on a small model of the 68010 against the
//! bit model of ref 3.9.3 (Figure 3-6, p. 3-16): a set is a big-endian
//! bit string whose member k is bit k of the whole, so member k is bit
//! k mod 8 of the byte `size-1 - k div 8`.

use std::collections::HashMap;

use dpc_m68k::emit::{emit, Options};
use dpc_m68k::insn::{AluOp, BitOp, Cond, Ea, Insn, Reg, ShiftCount, ShiftOp, Size, UnaryOp};
use dpc_syntax::preprocess::{preprocess, Options as PpOptions};
use dpc_syntax::SourceMap;

fn front(src: &str) -> Result<dpc_ir::Module, Vec<String>> {
    let mut sm = SourceMap::new();
    let f = sm.add("t.pas", src.to_string());
    let pp = preprocess(&mut sm, f, &PpOptions::default());
    assert!(!pp.has_errors(), "preprocessor errors");
    let unit = dpc_syntax::parser::parse(&pp.tokens).expect("parses");
    let render = |ds: Vec<dpc_syntax::Diagnostic>| -> Vec<String> {
        ds.iter().map(|d| d.message.clone()).collect()
    };
    let program = dpc_sema::analyze(&unit, &dpc_m68k::M68k::default()).map_err(render)?;
    dpc_ir::lower(&program).map_err(render)
}

/// Compile `src` to assembly lines without comments and blank lines, or
/// the diagnostics' messages.
fn compile(src: &str) -> Result<Vec<String>, Vec<String>> {
    let module = front(src)?;
    let asm = emit(&module, "t.pas", &Options::default()).map_err(|ds| {
        ds.iter()
            .map(|d| d.message.clone())
            .collect::<Vec<String>>()
    })?;
    Ok(asm
        .lines()
        .map(|l| l.split('|').next().unwrap().trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

/// A record with sets of each kind: a word set at 0, a longword set of
/// an enumeration at 2, a 32-byte `set of char` at 6, an 8-byte set at 38.
const DECLS: &str = "module m;
type
  e32 = (e00, e01, e02, e03, e04, e05, e06, e07, e08, e09, e10, e11, e12, e13,
         e14, e15, e16, e17, e18, e19, e20, e21, e22, e23, e24, e25, e26, e27,
         e28, e29, e30, e31);
  r = record
    w: set of 0..15;      { 0 }
    l: set of e32;        { 2 }
    big: set of char;     { 6 }
    b64: set of 0..63;    { 38 }
    w2: set of 0..15;     { 46 }
    l2: set of e32;       { 48 }
  end;
  rp = ^r;
  k64p = ^k64;
  k64 = 0..63;
  w16 = set of 0..15;
var
  gw: extern integer;
";

/// The body of routine `T` (between its entry and `unlk`).
fn body(routine: &str) -> Vec<String> {
    let lines = compile(&format!("{DECLS}{routine}")).unwrap_or_else(|e| panic!("{e:?}"));
    let start = lines.iter().position(|l| l == "T:").unwrap() + 2;
    let end = start
        + lines[start..]
            .iter()
            .position(|l| l.starts_with("unlk"))
            .unwrap();
    lines[start..end].to_vec()
}

fn error(routine: &str) -> String {
    let e = compile(&format!("{DECLS}{routine}")).unwrap_err();
    assert_eq!(e.len(), 1, "{e:?}");
    e[0].clone()
}

#[test]
fn constant_members_in_place() {
    // One member: `bset`/`bclr` on its byte (TTY_$K_SET_FLAG 0xE6745C
    // `bset #4,(9,A2)`; TTY_$I_OUTPUT_BUFFER_DRAINED 0xE1B39E).
    assert_eq!(
        body("procedure t(p: rp); options(val_param); begin p^.w := p^.w + [4] end;"),
        ["movea.l\t(8,%fp),%a0", "bset\t#4,(1,%a0)"]
    );
    assert_eq!(
        body("procedure t(p: rp); options(val_param); begin p^.w := p^.w - [9] end;"),
        ["movea.l\t(8,%fp),%a0", "bclr\t#1,(%a0)"]
    );
    assert_eq!(
        body("procedure t(p: rp); options(val_param); begin p^.l := p^.l + [e17] end;"),
        ["movea.l\t(8,%fp),%a0", "bset\t#1,(3,%a0)"]
    );
    // Several: `andi`/`ori` of the mask (TTY_$I_RCV 0xE1B98E `andi.w
    // #-0xe1,(8,A2)`).
    assert_eq!(
        body("procedure t(p: rp); options(val_param); begin p^.w := p^.w - [5, 6, 7] end;"),
        ["movea.l\t(8,%fp),%a0", "andi.w\t#-0xe1,(%a0)"]
    );
    assert_eq!(
        body("procedure t(p: rp); options(val_param); begin p^.l := p^.l + [e00..e03, e31] end;"),
        ["movea.l\t(8,%fp),%a0", "ori.l\t#-0x7ffffff1,(2,%a0)"]
    );
}

#[test]
fn run_time_members_in_place() {
    // TTY_$K_SET_INPUT_FLAG 0xE6768E..0xE676A2.
    assert_eq!(
        body(
            "procedure t(p: rp; k: e32); options(val_param);
             begin p^.l := p^.l + [k] end;"
        ),
        [
            "clr.l\t%d0",
            "move.w\t(0xc,%fp),%d1",
            "movea.l\t(8,%fp),%a0",
            "bset\t%d1,%d0",
            "or.l\t%d0,(2,%a0)"
        ]
    );
    assert_eq!(
        body(
            "procedure t(p: rp; k: e32); options(val_param);
             begin p^.l := p^.l - [k] end;"
        ),
        [
            "clr.l\t%d0",
            "move.w\t(0xc,%fp),%d1",
            "movea.l\t(8,%fp),%a0",
            "bset\t%d1,%d0",
            "not.l\t%d0",
            "and.l\t%d0,(2,%a0)"
        ]
    );
    // A word set (AST_$DISMOUNT 0xE069F4; DBUF_$INVALIDATE 0xE3AA8E).
    assert_eq!(
        body(
            "procedure t(p: rp; k: integer); options(val_param);
             begin p^.w := p^.w - [k] end;"
        ),
        [
            "clr.w\t%d0",
            "move.w\t(0xc,%fp),%d1",
            "movea.l\t(8,%fp),%a0",
            "bset\t%d1,%d0",
            "not.w\t%d0",
            "and.w\t%d0,(%a0)"
        ]
    );
}

#[test]
fn set_variables_and_values() {
    // In place from another set (TTY 0xE1C034, 0xE1BF9E); `[]` cleared
    // (0xE1C044); an intersection stored, the right operand first
    // (0xE1BF92).
    assert_eq!(
        body(
            "procedure t(p: rp); options(val_param);
             begin
               p^.l := p^.l + p^.l2;
               p^.w := [];
               p^.l := p^.l - p^.l2;
               p^.w2 := [];
               p^.l2 := p^.l * p^.l2;
             end;"
        ),
        [
            "movea.l\t(8,%fp),%a0",
            "move.l\t(0x30,%a0),%d0",
            "or.l\t%d0,(2,%a0)",
            "clr.w\t(%a0)",
            // Still in D0 (the register-contents cache, `crate::cache`).
            "not.l\t%d0",
            "and.l\t%d0,(2,%a0)",
            "clr.w\t(0x2e,%a0)",
            "move.l\t(0x30,%a0),%d0",
            "and.l\t(2,%a0),%d0",
            "move.l\t%d0,(0x30,%a0)"
        ]
    );
    // A constant set stored (TTY_$I_INIT 0xE33290 `move.l #0x1ffef,...`).
    assert_eq!(
        body("procedure t(p: rp); options(val_param); begin p^.l := [e00..e03, e05..e16] end;"),
        ["movea.l\t(8,%fp),%a0", "move.l\t#0x1ffef,(2,%a0)"]
    );
}

#[test]
fn empty_set_tests() {
    // `s <> []`: `tst` (TTY_$K_GET 0xE1C5D4); `s * c <> []`: the flags of
    // the `and` (TTY 0xE1BFFE); `x - y = []`: those of the difference's
    // `and` (PROC2_$SIGPAUSE 0xE3FACC).
    assert_eq!(
        body(
            "procedure t(p: rp); options(val_param);
             begin
               if p^.w <> [] then gw := 1;
               if p^.w * [0, 1, 2] <> [] then gw := 2;
               if p^.l - p^.l2 = [] then gw := 3;
             end;"
        ),
        [
            "movea.l\t(8,%fp),%a0",
            "tst.w\t(%a0)",
            "beq.s\t1f",
            "move.w\t#1,(GW).l",
            "1:",
            "moveq\t#7,%d0",
            "and.w\t(%a0),%d0",
            "beq.s\t2f",
            "move.w\t#2,(GW).l",
            "2:",
            "move.l\t(0x30,%a0),%d0",
            "not.l\t%d0",
            "and.l\t(2,%a0),%d0",
            "bne.s\t3f",
            "move.w\t#3,(GW).l",
            "3:"
        ]
    );
}

#[test]
fn set_comparisons() {
    // `=`/`<>` of register-sized sets: the integer comparison of their
    // values (bead domain-pascal-2i6f).  `s * c1 = c2`: PROC1_$SET_PRIORITY
    // 0xE152AC `moveq #11,D0; and.w (84,A4),D0; cmpi.w #8,D0; bne.s`; a
    // set against a constant, `cmpi`; two intersections, SIO_$K_SET_PARAM
    // 0xE680F6 `moveq; moveq; and.l; and.l; cmp.l D0,D1; beq.s` (the image
    // has the left operand in D1, dpc in D0 as the integer 0xE49C2E: the
    // register depends on the context and is not reproduced there).
    assert_eq!(
        body(
            "procedure t(p: rp); options(val_param);
             begin
               if p^.w * [0, 1, 3] = [3] then gw := 1;
               if p^.w2 <> [2, 4] then gw := 2;
               if p^.l * [e00] <> p^.l2 * [e00] then gw := 3;
             end;"
        ),
        [
            "movea.l\t(8,%fp),%a0",
            "moveq\t#0xb,%d0",
            "and.w\t(%a0),%d0",
            "cmpi.w\t#8,%d0",
            "bne.s\t1f",
            "move.w\t#1,(GW).l",
            "1:",
            "cmpi.w\t#0x14,(0x2e,%a0)",
            "beq.s\t2f",
            "move.w\t#2,(GW).l",
            "2:",
            "moveq\t#1,%d0",
            "moveq\t#1,%d1",
            "and.l\t(2,%a0),%d0",
            "and.l\t(0x30,%a0),%d1",
            "cmp.l\t%d1,%d0",
            "beq.s\t3f",
            "move.w\t#3,(GW).l",
            "3:"
        ]
    );
}

#[test]
fn membership() {
    // A constant set in a register (TTY_$K_SET_DELAY 0xE67A18..0xE67A1E).
    assert_eq!(
        body(
            "procedure t(k: e32); options(val_param);
             begin gw := 0; if not (k in [e00..e04]) then gw := 1 end;"
        ),
        [
            "move.w\t(8,%fp),%d0",
            "clr.w\t(GW).l",
            "moveq\t#0x1f,%d1",
            "btst\t%d0,%d1",
            "bne.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
    // A `set of char` in memory (the TTY module-local 0xE1B740..0xE1B754):
    // here the char is a parameter loaded at entry (codegen-patterns 12.2)
    // and copied from its register.
    assert_eq!(
        body(
            "procedure t(p: rp; c: char); options(val_param);
             begin gw := 0; if c in p^.big then gw := 1 end;"
        ),
        [
            "move.l\t%d2,-(%sp)",
            "movea.l\t(8,%fp),%a0",
            "move.b\t(0xc,%fp),%d0",
            "clr.w\t(GW).l",
            "clr.w\t%d1",
            "move.w\t#0xff,%d2",
            "move.b\t%d0,%d1",
            "lea\t(6,%a0),%a1",
            "sub.w\t%d1,%d2",
            "lsr.w\t#3,%d2",
            "btst\t%d1,(0,%a1,%d2.w)",
            "beq.s\t1f",
            "move.w\t#1,(GW).l",
            "1:",
            "move.l\t(-4,%fp),%d2"
        ]
    );
    // A 64-bit set and a word element in memory (MSG_$OPENI
    // 0xE59250..0xE5925E: `moveq #63`, the element, the `lea`).
    assert_eq!(
        body(
            "procedure t(p: rp; q: kp); options(val_param);
             begin gw := 0; if q^ in p^.b64 then gw := 1 end;"
                .replace("q: kp", "q: k64p")
                .as_str()
        ),
        [
            "movea.l\t(8,%fp),%a0",
            "movea.l\t(0xc,%fp),%a1",
            "clr.w\t(GW).l",
            "moveq\t#0x3f,%d0",
            "move.w\t(%a1),%d1",
            "lea\t(0x26,%a0),%a1",
            "sub.w\t%d1,%d0",
            "lsr.w\t#3,%d0",
            "btst\t%d1,(0,%a1,%d0.w)",
            "beq.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
}

/// `k in s` for a constant `k` on a set in memory (codegen-patterns 5.1,
/// "`in` on sets in memory, constant member"): the low-order part that
/// holds bit k at the same bit number, nothing renumbered.
#[test]
fn constant_members_in_memory() {
    let t = |cond: &str| {
        body(&format!(
            "procedure t(p: rp); options(val_param); begin gw := 0; if {cond} then gw := 1 end;"
        ))
    };
    // Member 0..6: `btst #k` on the set's last byte (tty_$i_kill_line
    // 0xE1B6B6 `btst #2,(0x1f,A2)`; TTY_$I_RCV 0xE1BAB6 `btst #0,(23,A2)`).
    assert_eq!(
        t("2 in p^.w"),
        [
            "movea.l\t(8,%fp),%a0",
            "clr.w\t(GW).l",
            "btst\t#2,(1,%a0)",
            "beq.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
    assert_eq!(
        t("e05 in p^.l"),
        [
            "movea.l\t(8,%fp),%a0",
            "clr.w\t(GW).l",
            "btst\t#5,(5,%a0)",
            "beq.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
    // Member 7: the byte's sign (0xE1B63E `tst.b (9,A2); bmi`).
    assert_eq!(
        t("not (7 in p^.w)"),
        [
            "movea.l\t(8,%fp),%a0",
            "clr.w\t(GW).l",
            "tst.b\t(1,%a0)",
            "bmi.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
    // Member 8..15 of a longword: its low word loaded (TTY_$I_RCV 0xE1B972
    // `move.w (22,A2),D0; btst #12,D0`).
    assert_eq!(
        t("e12 in p^.l"),
        [
            "movea.l\t(8,%fp),%a0",
            "clr.w\t(GW).l",
            "move.w\t(4,%a0),%d0",
            "btst\t#12,%d0",
            "beq.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
    // Member 8..14 of a word, 16..30 of a longword: the whole set loaded
    // (PROC2's word at -186, members 8..13; PROC2_$SET_SIG_MASK 0xE3F94A
    // `move.l; btst #17`).
    assert_eq!(
        t("12 in p^.w"),
        [
            "movea.l\t(8,%fp),%a0",
            "clr.w\t(GW).l",
            "move.w\t(%a0),%d0",
            "btst\t#12,%d0",
            "beq.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
    assert_eq!(
        t("e17 in p^.l"),
        [
            "movea.l\t(8,%fp),%a0",
            "clr.w\t(GW).l",
            "move.l\t(2,%a0),%d0",
            "btst\t#17,%d0",
            "beq.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
    // As a value: `btst; sne` (TTY_$K_GET 0xE1C54E `btst #1,(0x17,A2);
    // sne D1`), `tst.b; smi` for bit 7.
    assert_eq!(
        body("function t(p: rp): boolean; options(val_param); begin t := e01 in p^.l end;"),
        ["movea.l\t(8,%fp),%a0", "btst\t#1,(5,%a0)", "sne\t%d0"]
    );
    assert_eq!(
        body("function t(p: rp): boolean; options(val_param); begin t := 7 in p^.w2 end;"),
        ["movea.l\t(8,%fp),%a0", "tst.b\t(0x2f,%a0)", "smi\t%d0"]
    );
    // The sign member of the loaded part: no `btst #15`/`#31` in compiled
    // code; refused.
    assert!(error(
        "procedure t(p: rp); options(val_param); begin gw := 0; if 15 in p^.w then gw := 1 end;"
    )
    .contains("member 7, 15 or 31"));
    assert!(error(
        "procedure t(p: rp); options(val_param); begin gw := 0; if e31 in p^.l then gw := 1 end;"
    )
    .contains("member 7, 15 or 31"));
    // Member 7 of a set read whole (volatile here) is tested in a register,
    // where no `btst #7` exists either; refused.
    assert!(error(
        "type vw = [volatile] set of 0..15; var gv: extern vw;
         procedure t; begin gw := 0; if 7 in gv then gw := 1 end;"
    )
    .contains("member 7, 15 or 31"));
    // A volatile or device set is read whole, at its own size, never
    // narrowed (ref 3.15.1, 3.15.3: referenced exactly as written).
    assert_eq!(
        body(
            "type dw = [device] set of 0..15; var gd: extern dw;
             procedure t; begin gw := 0; if 3 in gd then gw := 1 end;"
        ),
        [
            "clr.w\t(GW).l",
            "move.w\t(GD).l,%d0",
            "btst\t#3,%d0",
            "beq.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
    assert_eq!(
        body(
            "type vl = [volatile] set of e32; var gv: extern vl;
             procedure t; begin gw := 0; if e12 in gv then gw := 1 end;"
        ),
        [
            "clr.w\t(GW).l",
            "move.l\t(GV).l,%d0",
            "btst\t#12,%d0",
            "beq.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
}

#[test]
fn values_and_inclusion() {
    // Bead domain-pascal-61fd.  `s - [c] + [d]`: PROC1_$TRY_TO_SUSPEND
    // 0xE1474C `moveq #-5,D0; and.w (84,A2),D0; ori.w #2,D0; move.w
    // D0,(84,A2)`.  `(s - a) + b`: PROC2_$SET_SIG_MASK 0xE3F850 `move.l;
    // not.l; and.l; or.l; move.l`.
    assert_eq!(
        body(
            "procedure t(p: rp); options(val_param);
             begin p^.w := p^.w - [2] + [1]; p^.l := (p^.l - p^.l2) + p^.l end;"
        ),
        [
            "movea.l\t(8,%fp),%a0",
            "moveq\t#-5,%d0",
            "and.w\t(%a0),%d0",
            "ori.w\t#2,%d0",
            "move.w\t%d0,(%a0)",
            "move.l\t(0x30,%a0),%d0",
            "not.l\t%d0",
            "and.l\t(2,%a0),%d0",
            "or.l\t(2,%a0),%d0",
            "move.l\t%d0,(2,%a0)"
        ]
    );
    // Best effort (no site): `s := s * c` not in place but as a value
    // stored; `x + y` and `c - x` as values; subset `<=` as `x - y = []`,
    // superset `>=` as `y - x = []`; a constant left operand swapped.  A
    // value just stored is read from its register (`crate::cache`).
    assert_eq!(
        body(
            "procedure t(p: rp); options(val_param);
             begin
               p^.w := p^.w * [1, 2];
               p^.w2 := p^.w + p^.w2;
               p^.w2 := [0, 5] - p^.w;
               if p^.w <= p^.w2 then gw := 1;
               if p^.l >= [e01] then gw := 2;
               if [1] = p^.w then gw := 3;
             end;"
        ),
        [
            "movea.l\t(8,%fp),%a0",
            "moveq\t#6,%d0",
            "and.w\t(%a0),%d0",
            "move.w\t%d0,(%a0)",
            "or.w\t(0x2e,%a0),%d0",
            "move.w\t%d0,(0x2e,%a0)",
            "move.w\t(%a0),%d0",
            "not.w\t%d0",
            "andi.w\t#0x21,%d0",
            "move.w\t%d0,(0x2e,%a0)",
            "not.w\t%d0",
            "and.w\t(%a0),%d0",
            "bne.s\t1f",
            "move.w\t#1,(GW).l",
            "1:",
            "move.l\t(2,%a0),%d0",
            "not.l\t%d0",
            "andi.l\t#2,%d0",
            "bne.s\t2f",
            "move.w\t#2,(GW).l",
            "2:",
            "cmpi.w\t#2,(%a0)",
            "bne.s\t3f",
            "move.w\t#3,(GW).l",
            "3:"
        ]
    );
}

#[test]
fn refusals() {
    // `[k]` assigned: built in memory with a guard (AST_$GET_SEG_MAP).
    assert!(
        error("procedure t(p: rp; k: integer); options(val_param); begin p^.w := [k] end;")
            .contains("assigning `[x]`")
    );
    // A run-time range.
    assert!(error(
        "procedure t(p: rp; k: integer); options(val_param); begin p^.w := p^.w + [0..k] end;"
    )
    .contains("run-time range"));
    // A set whose address contains a call is not updated in place: the
    // call runs once for the read and once for the write (bead
    // domain-pascal-61fd review); the value-then-store path refuses it.
    for stmt in [
        "ga[nx] := ga[nx] + [0]",
        "np^.w := np^.w + [0]",
        "np^.w := np^.w - [3]",
    ] {
        let e = error(&format!(
            "var ga: array[1..4] of w16;
             function nx: integer; extern;
             function np: rp; extern;
             procedure t; begin {stmt} end;"
        ));
        assert!(e.contains("with a call on either side"), "{stmt}: {e}");
    }
    // Comparing a set of more than a longword with a computed one.
    assert!(error(
        "procedure t(p: rp); options(val_param); begin if p^.b64 = p^.b64 + [1] then gw := 1 end;"
    )
    .contains("more than a longword"));
    // A constant member beyond the set's bits.
    assert!(
        error("procedure t(p: rp); options(val_param); begin p^.w := p^.w + [20] end;")
            .contains("outside the 16 bits")
    );
    // `in` on a large set with an element other than an `integer` that
    // may lie beyond it (the guarded form, NAME_$INIT 0xE3183E).
    assert!(error(
        "procedure t(c: char); options(val_param);
         type cs = set of 'a'..'z';
         var s: cs;
         begin if c in s then gw := 1 end;"
    )
    .contains("guarded test"));
}

#[test]
fn large_sets_in_memory() {
    // Strict (the library default): the forms without an image site
    // (bead domain-pascal-5kgg) are refused, the image's forms are not.
    assert!(error(
        "procedure t(p: rp; k: integer); options(val_param);
         begin p^.b64 := [k] end;"
    )
    .contains("frame temporary"));
    assert_eq!(
        body("procedure t(p: rp); options(val_param); begin p^.b64 := [58] end;").len(),
        3
    );
    // The best-effort forms, as `dpc` without `--strict`.
    dpc_syntax::diag::set_strict(false);
    // `in` with an `integer` element, unguarded (MSG_$FORK 0xE73F30..0xE73F3C
    // `moveq #63,D1; move.w (A2),D5; sub.w D5,D1; lsr.w #3,D1; lea
    // (0x1d8,A0),A3; btst D5,(0,A3,D1.w)`).
    assert_eq!(
        body(
            "procedure t(p: rp; k: integer); options(val_param);
             begin if k in p^.b64 then gw := 1 end;"
        ),
        [
            "movea.l\t(8,%fp),%a0",
            "move.w\t(0xc,%fp),%d0",
            "moveq\t#0x3f,%d1",
            "lea\t(0x26,%a0),%a1",
            "sub.w\t%d0,%d1",
            "lsr.w\t#3,%d1",
            "btst\t%d0,(0,%a1,%d1.w)",
            "beq.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
    // A constant: longword stores (MSG_$INIT 0xE31BD4 `move.l
    // #0x4000000,(0x1e0,A0); clr.l (0x1e4,A0)`).
    assert_eq!(
        body("procedure t(p: rp); options(val_param); begin p^.b64 := [58] end;"),
        [
            "movea.l\t(8,%fp),%a0",
            "move.l\t#0x4000000,(0x26,%a0)",
            "clr.l\t(0x2a,%a0)"
        ]
    );
    // `[k]`: cleared by longwords, then `bset` on the byte (MSG_$OPENI
    // 0xE59248..0xE59260, there in a frame temporary).
    assert_eq!(
        body(
            "procedure t(p: rp; k: integer); options(val_param);
             begin p^.b64 := [k] end;"
        ),
        [
            "movea.l\t(8,%fp),%a0",
            "lea\t(0x26,%a0),%a1",
            "clr.l\t(%a1)+",
            "clr.l\t(%a1)+",
            "moveq\t#0x3f,%d0",
            "move.w\t(0xc,%fp),%d1",
            "movea.l\t(8,%fp),%a0",
            "sub.w\t%d1,%d0",
            "lea\t(0x26,%a0),%a1",
            "lsr.w\t#3,%d0",
            "bset\t%d1,(0,%a1,%d0.w)"
        ]
    );
    // `s - [k]`: `bclr` on the byte; `s = []`: `tst.l (An)+` per longword
    // after an unused count (MSG_$CLOSEI 0xE59490..0xE5949C).
    assert_eq!(
        body(
            "procedure t(p: rp; k: integer); options(val_param);
             begin p^.b64 := p^.b64 - [k]; if p^.b64 = [] then gw := 1 end;"
        ),
        [
            "movea.l\t(8,%fp),%a0",
            "move.w\t(0xc,%fp),%d0",
            "moveq\t#0x3f,%d1",
            "lea\t(0x26,%a0),%a1",
            "sub.w\t%d0,%d1",
            "lsr.w\t#3,%d1",
            "bclr\t%d0,(0,%a1,%d1.w)",
            "lea\t(0x26,%a0),%a1",
            "moveq\t#1,%d0",
            "tst.l\t(%a1)+",
            "bne.s\t1f",
            "tst.l\t(%a1)+",
            "bne.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
    // `in` as a value: `sne` after the byte `btst`, into the offset's
    // register.
    assert_eq!(
        body(
            "function t(p: rp; c: char): boolean; options(val_param);
             begin t := c in p^.big end;"
        ),
        [
            "clr.w\t%d0",
            "move.w\t#0xff,%d1",
            "move.b\t(0xc,%fp),%d0",
            "movea.l\t(8,%fp),%a0",
            "sub.w\t%d0,%d1",
            "lea\t(6,%a0),%a1",
            "lsr.w\t#3,%d1",
            "btst\t%d0,(0,%a1,%d1.w)",
            "sne\t%d1",
            "move.b\t%d1,%d0"
        ]
    );
}

// ---------------------------------------------------------------------
// The meaning of the code: a model of the instructions used, against the
// bit model of ref 3.9.3.
// ---------------------------------------------------------------------

const FP: u32 = 0x1000;
const REC: u32 = 0x2000;
const GW: u32 = 0x3000;
/// The record's bytes: [`DECLS`]'s `r` is 52 bytes.
const REC_BYTES: u32 = 52;

struct Machine {
    mem: HashMap<u32, u8>,
    d: [u32; 8],
    a: [u32; 8],
    z: bool,
    n: bool,
}

fn bytes(s: Size) -> u32 {
    match s {
        Size::B => 1,
        Size::W => 2,
        Size::L => 4,
    }
}

fn mask(s: Size) -> u32 {
    match s {
        Size::B => 0xff,
        Size::W => 0xffff,
        Size::L => 0xffff_ffff,
    }
}

impl Machine {
    fn new() -> Machine {
        let mut a = [0; 8];
        a[6] = FP;
        a[7] = FP - 0x100;
        Machine {
            mem: HashMap::new(),
            d: [0x5a5a_5a5a; 8],
            a,
            z: false,
            n: false,
        }
    }
    fn rd(&self, at: u32, s: Size) -> u32 {
        assert!(bytes(s) == 1 || at % 2 == 0, "odd {s:?} access at {at:#x}");
        (0..bytes(s)).fold(0, |v, k| {
            (v << 8) | *self.mem.get(&(at + k)).unwrap_or(&0) as u32
        })
    }
    fn wr(&mut self, at: u32, s: Size, v: u32) {
        assert!(bytes(s) == 1 || at % 2 == 0, "odd {s:?} access at {at:#x}");
        let n = bytes(s);
        for k in 0..n {
            self.mem.insert(at + k, (v >> (8 * (n - 1 - k))) as u8);
        }
    }
    fn addr(&self, ea: &Ea) -> u32 {
        match ea {
            Ea::Ind(a) => self.a[*a as usize],
            Ea::Disp(d, a) => self.a[*a as usize].wrapping_add(*d as u32),
            Ea::Idx(d, a, x, long) => {
                let x = self.d[*x as usize];
                let x = if *long {
                    x
                } else {
                    x as u16 as i16 as i32 as u32
                };
                self.a[*a as usize].wrapping_add(*d as u32).wrapping_add(x)
            }
            Ea::AbsL(s, o) => {
                // GS and GL: [`DECLS`]'s record's `w` and `l` seen as
                // module variables.
                let at = match s.as_str() {
                    "GW" => GW,
                    "GS" => REC,
                    "GL" => REC + 2,
                    _ => panic!("symbol {s}"),
                };
                at.wrapping_add(*o as u32)
            }
            e => panic!("operand {e:?}"),
        }
    }
    fn get(&self, ea: &Ea, s: Size) -> u32 {
        match ea {
            Ea::Reg(Reg::D(n)) => self.d[*n as usize] & mask(s),
            Ea::Reg(Reg::A(n)) => self.a[*n as usize] & mask(s),
            Ea::Imm(v) => *v as u32 & mask(s),
            e => self.rd(self.addr(e), s),
        }
    }
    fn put(&mut self, ea: &Ea, s: Size, v: u32) {
        match ea {
            Ea::Reg(Reg::D(n)) => {
                let r = &mut self.d[*n as usize];
                *r = (*r & !mask(s)) | (v & mask(s));
            }
            e => {
                let at = self.addr(e);
                self.wr(at, s, v)
            }
        }
    }
    fn flags(&mut self, v: u32, s: Size) {
        let v = v & mask(s);
        self.z = v == 0;
        self.n = v & (mask(s) ^ (mask(s) >> 1)) != 0;
    }
    /// The bit `bit` of `dst` addressed as `bset`/`bclr`/`btst` do: mod
    /// 32 in a data register, mod 8 in a memory byte.
    fn bit_of(dst: &Ea, bit: u32) -> (Size, u32) {
        match dst {
            Ea::Reg(Reg::D(_)) => (Size::L, bit % 32),
            _ => (Size::B, bit % 8),
        }
    }
    fn holds(&self, cond: Cond) -> bool {
        match cond {
            Cond::T => true,
            Cond::Ne => !self.z,
            Cond::Eq => self.z,
            Cond::Mi => self.n,
            Cond::Pl => !self.n,
            c => panic!("condition {c:?}"),
        }
    }
    fn run(&mut self, code: &[Insn]) {
        let labels: HashMap<u32, usize> = code
            .iter()
            .enumerate()
            .filter_map(|(k, i)| match i {
                Insn::Label(l) => Some((*l, k)),
                _ => None,
            })
            .collect();
        let mut pc = 0;
        while pc < code.len() {
            let i = &code[pc];
            pc += 1;
            match i {
                Insn::Link { .. }
                | Insn::MovemSave { .. }
                | Insn::MovemRestore { .. }
                | Insn::Label(_) => {}
                Insn::Unlk | Insn::Rts => return,
                Insn::Branch { cond, label, .. } => {
                    if self.holds(*cond) {
                        pc = labels[label];
                    }
                }
                Insn::Movea { size, src, dst } => self.a[*dst as usize] = self.get(src, *size),
                Insn::Lea { src, dst } => self.a[*dst as usize] = self.addr(src),
                Insn::Move { size, src, dst } => {
                    let v = self.get(src, *size);
                    let dst = match dst {
                        // A register saved on the stack.
                        Ea::PreDec(r) => {
                            self.a[*r as usize] -= bytes(*size);
                            Ea::Ind(*r)
                        }
                        d => d.clone(),
                    };
                    self.put(&dst, *size, v);
                    self.flags(v, *size);
                }
                Insn::Moveq { value, dst } => self.d[*dst as usize] = *value as i32 as u32,
                Insn::Clr { size, dst } => {
                    self.put(dst, *size, 0);
                    self.flags(0, *size);
                }
                Insn::AluImm { op, size, imm, dst } => {
                    let v = alu(*op, self.get(dst, *size), *imm as u32);
                    self.put(dst, *size, v);
                    self.flags(v, *size);
                }
                Insn::Alu { op, size, src, dst } => {
                    let r = Ea::Reg(Reg::D(*dst));
                    let v = alu(*op, self.get(&r, *size), self.get(src, *size));
                    self.put(&r, *size, v);
                    self.flags(v, *size);
                }
                Insn::AluToMem { op, size, src, dst } => {
                    let r = Ea::Reg(Reg::D(*src));
                    let v = alu(*op, self.get(dst, *size), self.get(&r, *size));
                    self.put(dst, *size, v);
                    self.flags(v, *size);
                }
                Insn::Shift {
                    op: ShiftOp::Lsr,
                    size,
                    count: ShiftCount::Imm(k),
                    dst,
                } => {
                    let r = Ea::Reg(Reg::D(*dst));
                    let v = self.get(&r, *size) >> k;
                    self.put(&r, *size, v);
                    self.flags(v, *size);
                }
                Insn::Unary {
                    op: UnaryOp::Not,
                    size,
                    dst,
                } => {
                    let v = !self.get(dst, *size);
                    self.put(dst, *size, v);
                    self.flags(v, *size);
                }
                Insn::BitImm { op, bit, dst } => self.bit(Some(*op), *bit as u32, dst),
                Insn::Bit { op, bit, dst } => {
                    let k = self.d[*bit as usize];
                    self.bit(Some(*op), k, dst)
                }
                Insn::BtstImm { bit, dst } => self.bit(None, *bit as u32, dst),
                Insn::Btst { bit, dst } => {
                    let k = self.d[*bit as usize];
                    self.bit(None, k, dst)
                }
                Insn::Tst { size, dst } => {
                    let v = self.get(dst, *size);
                    self.flags(v, *size);
                }
                Insn::Scc { cond, dst } => {
                    let t = self.holds(*cond);
                    self.put(dst, Size::B, if t { 0xff } else { 0 });
                }
                i => panic!("not modelled: {i}"),
            }
        }
    }
    /// `bset`/`bclr` (`op`) or `btst` (`None`): Z is the bit's old value
    /// complemented.
    fn bit(&mut self, op: Option<BitOp>, k: u32, dst: &Ea) {
        let (s, b) = Self::bit_of(dst, k);
        let v = self.get(dst, s);
        self.z = v & (1 << b) == 0;
        match op {
            Some(BitOp::Set) => self.put(dst, s, v | (1 << b)),
            Some(BitOp::Clr) => self.put(dst, s, v & !(1 << b)),
            None => {}
        }
    }
}

fn alu(op: AluOp, a: u32, b: u32) -> u32 {
    match op {
        AluOp::And => a & b,
        AluOp::Or => a | b,
        AluOp::Add => a.wrapping_add(b),
        AluOp::Sub => a.wrapping_sub(b),
        AluOp::Eor => a ^ b,
    }
}

/// The selected code of routine `name` in `src`.
fn code(src: &str, name: &str) -> Vec<Insn> {
    let module = front(src).unwrap_or_else(|e| panic!("{e:?}"));
    let f = module
        .functions
        .iter()
        .find(|f| f.symbol == name)
        .unwrap_or_else(|| panic!("no {name}"));
    dpc_m68k::isel::select(f).unwrap_or_else(|e| panic!("{name}: {}", e.message))
}

/// A small deterministic generator (xorshift).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// The set of `bytes` bytes at `off` in the record, as the members it
/// holds by ref 3.9.3: the bytes read as one big-endian number, member k
/// its bit k.
fn members(m: &Machine, off: u32, bytes: u32) -> Vec<bool> {
    let raw: Vec<u8> = (0..bytes)
        .map(|k| *m.mem.get(&(REC + off + k)).unwrap_or(&0))
        .collect();
    (0..bytes * 8)
        .map(|k| raw[(bytes - 1 - k / 8) as usize] & (1 << (k % 8)) != 0)
        .collect()
}

fn random_record(m: &mut Machine, rng: &mut Rng) {
    for k in 0..REC_BYTES {
        m.mem.insert(REC + k, rng.next() as u8);
    }
}

/// The sets of [`DECLS`]'s record: field, offset, bytes, element type.
const SETS: &[(&str, u32, u32, &str)] = &[("w", 0, 2, "integer"), ("l", 2, 4, "e32")];

/// `s := s + [k]` and `s := s - [k]`, k a constant and a run-time member,
/// over random contents: exactly member k changes, and the rest of the
/// record is kept.
#[test]
fn adding_and_removing_members_follows_the_bit_model() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for &(f, off, n, ety) in SETS {
        let bits = n * 8;
        let lit = |k: u32| {
            if ety == "e32" {
                format!("e{k:02}")
            } else {
                k.to_string()
            }
        };
        let mut src = String::from(DECLS);
        for k in 0..bits {
            src += &format!(
                "procedure a{k}(p: rp); options(val_param); begin p^.{f} := p^.{f} + [{}] end;\n\
                 procedure r{k}(p: rp); options(val_param); begin p^.{f} := p^.{f} - [{}] end;\n",
                lit(k),
                lit(k)
            );
        }
        src += &format!(
            "procedure av(p: rp; k: {ety}); options(val_param); begin p^.{f} := p^.{f} + [k] end;\n\
             procedure rv(p: rp; k: {ety}); options(val_param); begin p^.{f} := p^.{f} - [k] end;\n"
        );
        let av = code(&src, "AV");
        let rv = code(&src, "RV");
        for k in 0..bits {
            let fixed = [
                (true, code(&src, &format!("A{k}"))),
                (false, code(&src, &format!("R{k}"))),
            ];
            for (add, c) in fixed
                .iter()
                .chain([(true, av.clone()), (false, rv.clone())].iter())
            {
                let mut m = Machine::new();
                random_record(&mut m, &mut rng);
                m.wr(FP + 8, Size::L, REC);
                m.wr(FP + 12, Size::W, k);
                let before: Vec<u8> = (0..REC_BYTES).map(|b| m.mem[&(REC + b)]).collect();
                let want: Vec<bool> = members(&m, off, n)
                    .into_iter()
                    .enumerate()
                    .map(|(j, x)| if j as u32 == k { *add } else { x })
                    .collect();
                m.run(c);
                assert_eq!(
                    members(&m, off, n),
                    want,
                    "{f} {} [{k}]",
                    if *add { '+' } else { '-' }
                );
                for b in (0..REC_BYTES).filter(|b| !(off..off + n).contains(b)) {
                    assert_eq!(
                        m.mem[&(REC + b)],
                        before[b as usize],
                        "{f}: byte {b} changed"
                    );
                }
            }
        }
    }
}

/// `s := s + c`, `s := s - c` and `s * c <> []` for constant sets of
/// several members, over random contents and random constants.
#[test]
fn constant_sets_follow_the_bit_model() {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    for &(f, off, n, ety) in SETS {
        let bits = n * 8;
        for t in 0..40 {
            let ms: Vec<u32> = (0..bits).filter(|_| rng.next() % 3 == 0).collect();
            if ms.len() < 2 {
                continue;
            }
            let lit = ms
                .iter()
                .map(|k| {
                    if ety == "e32" {
                        format!("e{k:02}")
                    } else {
                        k.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            let src = format!(
                "{DECLS}procedure ad(p: rp); options(val_param); begin p^.{f} := p^.{f} + [{lit}] end;\n\
                 procedure rm(p: rp); options(val_param); begin p^.{f} := p^.{f} - [{lit}] end;\n\
                 procedure it(p: rp); options(val_param);\n\
                 begin gw := 0; if p^.{f} * [{lit}] <> [] then gw := 1 end;\n"
            );
            for (name, op) in [("AD", '+'), ("RM", '-'), ("IT", '*')] {
                let c = code(&src, name);
                let mut m = Machine::new();
                random_record(&mut m, &mut rng);
                m.wr(FP + 8, Size::L, REC);
                let old = members(&m, off, n);
                m.run(&c);
                let new = members(&m, off, n);
                for k in 0..bits as usize {
                    let inc = ms.contains(&(k as u32));
                    let want = match op {
                        '+' => old[k] || inc,
                        '-' => old[k] && !inc,
                        _ => old[k],
                    };
                    assert_eq!(new[k], want, "{f} {op} case {t}: member {k}");
                }
                if op == '*' {
                    let any = ms.iter().any(|&k| old[k as usize]);
                    assert_eq!(m.rd(GW, Size::W), any as u32, "{f} * case {t}");
                }
            }
        }
    }
}

/// `c in s` for a `set of char` in memory, every char, over random
/// contents: the test reads the bit the model names.
#[test]
fn membership_of_a_large_set_follows_the_bit_model() {
    let src = format!(
        "{DECLS}procedure t(p: rp; c: char); options(val_param);\n\
         begin gw := 0; if c in p^.big then gw := 1 end;\n"
    );
    let c = code(&src, "T");
    let mut rng = Rng(0x0123_4567_89ab_cdef);
    for round in 0..4 {
        let mut m = Machine::new();
        random_record(&mut m, &mut rng);
        let big = members(&m, 6, 32);
        for ch in 0..256u32 {
            m.wr(FP + 8, Size::L, REC);
            // A char by value: the high byte of its word slot.
            m.wr(FP + 12, Size::W, ch << 8);
            m.d = [0x5a5a_5a5a; 8];
            m.run(&c);
            assert_eq!(
                m.rd(GW, Size::W) == 1,
                big[ch as usize],
                "round {round}: chr({ch})"
            );
        }
    }
}

/// `k in s` for a run-time member of a longword set, and of a constant
/// one, in a condition.
#[test]
fn membership_of_a_longword_set_follows_the_bit_model() {
    let src = format!(
        "{DECLS}procedure t(p: rp; k: e32); options(val_param);\n\
         begin gw := 0; if k in p^.l then gw := 1 end;\n\
         procedure u(k: e32); options(val_param);\n\
         begin gw := 0; if k in [e01, e05..e09, e30] then gw := 1 end;\n"
    );
    let t = code(&src, "T");
    let u = code(&src, "U");
    let mut rng = Rng(0x1357_9bdf_2468_ace0);
    let konst: Vec<u32> = [1, 5, 6, 7, 8, 9, 30].to_vec();
    for _ in 0..8 {
        let mut m = Machine::new();
        random_record(&mut m, &mut rng);
        let l = members(&m, 2, 4);
        for k in 0..32u32 {
            m.wr(FP + 8, Size::L, REC);
            m.wr(FP + 12, Size::W, k);
            m.run(&t);
            assert_eq!(m.rd(GW, Size::W) == 1, l[k as usize], "{k} in l");
            m.wr(FP + 8, Size::W, k);
            m.run(&u);
            assert_eq!(m.rd(GW, Size::W) == 1, konst.contains(&k), "{k} in [..]");
        }
    }
}

/// `k in s` for every constant member of a word and a longword set in
/// memory (all but the sign members, refused), narrowed or not, in a
/// condition and as a value.
#[test]
fn constant_membership_in_memory_follows_the_bit_model() {
    let mut src = DECLS.to_string();
    for k in 0..15 {
        src += &format!(
            "procedure w{k}(p: rp); options(val_param);\n\
             begin gw := 0; if {k} in p^.w2 then gw := 1 end;\n\
             function v{k}(p: rp): boolean; options(val_param);\n\
             begin v{k} := {k} in p^.w end;\n"
        );
    }
    for k in 0..31 {
        src += &format!(
            "procedure l{k}(p: rp); options(val_param);\n\
             begin gw := 0; if e{k:02} in p^.l2 then gw := 1 end;\n"
        );
    }
    let w: Vec<_> = (0..15).map(|k| code(&src, &format!("W{k}"))).collect();
    let v: Vec<_> = (0..15).map(|k| code(&src, &format!("V{k}"))).collect();
    // Member 15 of the longword is refused like 31 (its low word's sign).
    let lk: Vec<usize> = (0..31).filter(|&k| k != 15).collect();
    let l: Vec<_> = lk.iter().map(|k| code(&src, &format!("L{k}"))).collect();
    let mut rng = Rng(0x0f1e_2d3c_4b5a_6978);
    for _ in 0..8 {
        let mut m = Machine::new();
        random_record(&mut m, &mut rng);
        let (w2, ws, l2) = (members(&m, 46, 2), members(&m, 0, 2), members(&m, 48, 4));
        for k in 0..15 {
            m.wr(FP + 8, Size::L, REC);
            m.run(&w[k]);
            assert_eq!(m.rd(GW, Size::W) == 1, w2[k], "{k} in w2");
            m.run(&v[k]);
            assert_eq!(m.d[0] & 0xff == 0xff, ws[k], "{k} in w");
            assert!(m.d[0] & 0xff == 0xff || m.d[0] & 0xff == 0, "boolean {k}");
        }
        for (i, &k) in lk.iter().enumerate() {
            m.wr(FP + 8, Size::L, REC);
            m.run(&l[i]);
            assert_eq!(m.rd(GW, Size::W) == 1, l2[k], "e{k} in l2");
        }
    }
}

/// `k in s` for every constant member (but the refused 15 and 31) of a
/// word and a longword set at the other kinds of address: absolute (a
/// module variable), `(An)` (a `var` parameter) and the frame (a value
/// parameter's copy).
#[test]
fn constant_membership_at_other_addresses_follows_the_bit_model() {
    let mut src = DECLS.to_string();
    src += "type l32 = set of e32;\nvar gs: extern w16; gl: extern l32;\n";
    let wk: Vec<u32> = (0..15).collect();
    let lk: Vec<u32> = (0..31).filter(|&k| k != 15).collect();
    for &k in &wk {
        src += &format!(
            "procedure aw{k}; begin gw := 0; if {k} in gs then gw := 1 end;\n\
             procedure fw{k}(s: w16); begin gw := 0; if {k} in s then gw := 1 end;\n"
        );
    }
    for &k in &lk {
        src += &format!(
            "procedure al{k}; begin gw := 0; if e{k:02} in gl then gw := 1 end;\n\
             procedure il{k}(var s: l32); begin gw := 0; if e{k:02} in s then gw := 1 end;\n"
        );
    }
    let run = |m: &mut Machine, name: &str, arg: Option<u32>| -> bool {
        if let Some(a) = arg {
            m.wr(FP + 8, Size::L, a);
        }
        m.wr(GW, Size::W, 0x5a5a);
        m.run(&code(&src, name));
        m.rd(GW, Size::W) == 1
    };
    let mut rng = Rng(0x7a69_5848_3726_1504);
    for _ in 0..8 {
        let mut m = Machine::new();
        random_record(&mut m, &mut rng);
        let (w, l, w2, l2) = (
            members(&m, 0, 2),
            members(&m, 2, 4),
            members(&m, 46, 2),
            members(&m, 48, 4),
        );
        for &k in &wk {
            let k = k as usize;
            assert_eq!(run(&mut m, &format!("AW{k}"), None), w[k], "{k} in gs");
            assert_eq!(
                run(&mut m, &format!("FW{k}"), Some(REC + 46)),
                w2[k],
                "{k} in s (frame)"
            );
        }
        for &k in &lk {
            let k = k as usize;
            assert_eq!(run(&mut m, &format!("AL{k}"), None), l[k], "e{k} in gl");
            assert_eq!(
                run(&mut m, &format!("IL{k}"), Some(REC + 48)),
                l2[k],
                "e{k} in s (An)"
            );
        }
    }
}

/// A run-time member outside the set's bits is not checked: `bset` on a
/// register takes it mod 32, as the original's code does (no range check
/// at TTY_$K_SET_INPUT_FLAG 0xE67692, TTY_$K_ENABLE_FUNC 0xE675EA; ref
/// 3.9.3 has no member there).  So `[33]` of a longword set adds member
/// 1, and `[20]` of a word set changes nothing (bit 20 of the register is
/// not stored).
#[test]
fn run_time_members_beyond_the_set_wrap_as_in_the_original() {
    let src = format!(
        "{DECLS}procedure al(p: rp; k: integer); options(val_param);\n\
         begin p^.l := p^.l + [e32(k)] end;\n\
         procedure aw(p: rp; k: integer); options(val_param);\n\
         begin p^.w := p^.w + [k] end;\n"
    );
    for (name, off, n, k, changed) in [("AL", 2, 4, 33u32, Some(1usize)), ("AW", 0, 2, 20, None)] {
        let c = code(&src, name);
        let mut m = Machine::new();
        m.wr(FP + 8, Size::L, REC);
        m.wr(FP + 12, Size::W, k);
        m.run(&c);
        let got = members(&m, off, n);
        for (j, x) in got.iter().enumerate() {
            assert_eq!(*x, Some(j) == changed, "{name} [{k}]: member {j}");
        }
    }
}

/// `[k]` as a value argument: the register built, then pushed (DISK_$FORMAT
/// 0xE3D4E2..0xE3D4E6 `clr.w D1; bset D3,D1; move.w D1,-(SP)`).
#[test]
fn one_member_set_as_an_argument() {
    let lines = compile(&format!(
        "{DECLS}procedure pv(s: w16; x: integer); options(val_param, extern);\n\
         procedure t(k, j: integer); options(val_param); begin pv([k], j) end;\n"
    ))
    .unwrap();
    let start = lines.iter().position(|l| l == "T:").unwrap() + 2;
    assert_eq!(
        lines[start..start + 6],
        [
            "move.w\t(0xa,%fp),-(%sp)",
            "clr.w\t%d0",
            "move.w\t(8,%fp),%d1",
            "bset\t%d1,%d0",
            "move.w\t%d0,-(%sp)",
            "jsr\t(PV).l"
        ]
    );
}

/// Types for the constant-set tests (bead domain-pascal-bitu): Apollo's
/// `pinteger`, an enumeration of 16 values (a one-word set), a longword
/// set of integers.
const CONST_TYPES: &str = "type
  pinteger = 0..65535;
  n16 = (f00, f01, f02, f03, f04, f05, f06, f07, f08, f09, f10, f11, f12, f13,
         f14, f15);
  s32 = set of 0..31;
var
  gs32: extern s32;
";

/// `k in c` for a constant set of one word or one longword and a
/// run-time element (codegen-patterns 5.1, "`x in c`"): a `0..65535`
/// element of an integer constructor zero-extended around the longword
/// constant's load, a one-word set that `moveq` loads.
#[test]
fn constant_sets_with_a_run_time_element() {
    // `vol_idx in [1..10]`, AS_IO_SETUP 0xE6B75C..0xE6B76A.
    assert_eq!(
        body(&format!(
            "{CONST_TYPES}procedure t(var k: pinteger);
             begin gw := 0; if not (k in [1..10]) then gw := 1 end;"
        )),
        [
            "clr.w\t(GW).l",
            "movea.l\t(8,%fp),%a0",
            "clr.l\t%d0",
            "move.l\t#0x7fe,%d1",
            "move.w\t(%a0),%d0",
            "btst\t%d0,%d1",
            "bne.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
    // An `integer` element of a constant `moveq` loads: not extended, the
    // element first (TTY_$K_SET_DELAY 0xE67A18..0xE67A1C).
    assert_eq!(
        body(
            "procedure t(k: integer); options(val_param);
             begin gw := 0; if k in [1..6] then gw := 1 end;"
        ),
        [
            "move.w\t(8,%fp),%d0",
            "clr.w\t(GW).l",
            "moveq\t#0x7e,%d1",
            "btst\t%d0,%d1",
            "beq.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
    // A one-word set of an enumeration, a `moveq` constant.
    assert_eq!(
        body(&format!(
            "{CONST_TYPES}procedure t(e: n16); options(val_param);
             begin gw := 0; if e in [f01, f03..f06] then gw := 1 end;"
        )),
        [
            "move.w\t(8,%fp),%d0",
            "clr.w\t(GW).l",
            "moveq\t#0x7a,%d1",
            "btst\t%d0,%d1",
            "beq.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
    // As a value, `sne` in the set's register.
    assert_eq!(
        body(
            "function t(k: integer): boolean; options(val_param);
             begin t := k in [0, 1, 2, 4, 5] end;"
        ),
        [
            "move.w\t(8,%fp),%d0",
            "moveq\t#0x37,%d1",
            "btst\t%d0,%d1",
            "sne\t%d1",
            "move.b\t%d1,%d0"
        ]
    );
    // A longword constructor with a `0..65535` element: as it is (no
    // site shows one widened for a longword constant; dpc's earlier form
    // kept).
    assert_eq!(
        body(&format!(
            "{CONST_TYPES}function t(k: pinteger): boolean; options(val_param);
             begin t := k in [1, 31] end;"
        )),
        [
            "move.w\t(8,%fp),%d0",
            "move.l\t#-0x7ffffffe,%d1",
            "btst\t%d0,%d1",
            "sne\t%d1",
            "move.b\t%d1,%d0"
        ]
    );
    // A `0..65535` element of a longword set variable: as it is (not
    // extended: no site either way, dpc's earlier form kept).
    assert_eq!(
        body(&format!(
            "{CONST_TYPES}function t(k: pinteger): boolean; options(val_param);
             begin t := k in gs32 end;"
        )),
        [
            "move.w\t(8,%fp),%d0",
            "move.l\t(GS32).l,%d1",
            "btst\t%d0,%d1",
            "sne\t%d1",
            "move.b\t%d1,%d0"
        ]
    );
}

#[test]
fn constant_set_refusals() {
    let refused = |src: &str, want: &str| {
        let e = error(&format!("{CONST_TYPES}{src}"));
        assert!(e.contains(want), "{src}: {e}");
    };
    // A boolean element: true is 0xFF, `btst` would take bit 31.
    refused(
        "function t(b: boolean): boolean; options(val_param); begin t := b in [true] end;",
        "one-byte element",
    );
    // A one-word constant `moveq` cannot load, with an `integer` element
    // (`move.w #0xfff` at FILE_$PRIV_LOCK 0xE5F114, `move.l #0x7fe` with
    // the DISK_ `0..65535` elements) or an enumeration.
    refused(
        "function t(k: integer): boolean; options(val_param); begin t := k in [1..10] end;",
        "`moveq` cannot load",
    );
    refused(
        "procedure t(e: n16); options(val_param); begin if e in [f01, f08] then gw := 1 end;",
        "`moveq` cannot load",
    );
    // A `0..65535` element with a constant `moveq` loads (no site).
    refused(
        "function t(k: pinteger): boolean; options(val_param); begin t := k in [1..3] end;",
        "`moveq` loads with a `0..65535` element",
    );
    // A `0..65535` element on a declared one-word set type's constant.
    refused(
        "type w16c = set of 0..15;
         function t(k: pinteger): boolean; options(val_param); begin t := k in w16c([1..10]) end;",
        "of a declared type with a `0..65535` element",
    );
    // A 4-byte element on a one-word constant set (no site).
    refused(
        "function t(k: integer32): boolean; options(val_param); begin t := k in [1..3] end;",
        "4-byte element",
    );
    // An array element: the image loads the constant first (MAC_$OPEN
    // 0xE0B8FC), which dpc's scheduler does not do.
    refused(
        "var tab: extern array[0..7] of integer;
         function t(i: integer): boolean; options(val_param); begin t := tab[i] in [0, 1] end;",
        "array element",
    );
    refused(
        "var tab: extern array[0..7] of pinteger;
         function t(i: integer): boolean; options(val_param); begin t := tab[i] in [1..10] end;",
        "array element",
    );
    // An element of a `with` over an array element, its base set just
    // before (ROUTE_$INCOMING 0xE878F8: the constant before the `lea`).
    refused(
        "type pt = record pad: array[1..22] of integer; active: integer; end;
         var ports: extern array[0..7] of pt;
         procedure t(i: integer); options(val_param);
         begin with ports[i] do if active in [0, 1] then gw := 1 end;",
        "address register set just before",
    );
    // A one-word set variable (guarded or not: not settled).
    let e = error(
        "procedure t(p: rp; k: integer); options(val_param);
         begin if k in p^.w then gw := 1 end;",
    );
    assert!(e.contains("one-word set variable"), "{e}");
}

/// The constant-set tests against the bit model, and the original's
/// modulo 32 for elements above the set (../source's note on
/// DISK_$FORMAT_WHOLE: `vol_idx` 33..42 pass the test at 0xE3D2B8):
/// `btst Dk,Dn` takes bit k mod 32, no range check.
#[test]
fn constant_sets_take_the_element_mod_32() {
    let src = format!(
        "{DECLS}{CONST_TYPES}procedure ti(k: integer); options(val_param);\n\
         begin gw := 0; if k in [1..6] then gw := 1 end;\n\
         procedure tu(var k: pinteger);\n\
         begin gw := 0; if k in [1..10] then gw := 1 end;\n\
         procedure te(e: n16); options(val_param);\n\
         begin gw := 0; if e in [f00, f03..f06] then gw := 1 end;\n"
    );
    let ti = code(&src, "TI");
    let tu = code(&src, "TU");
    let te = code(&src, "TE");
    for k in (0..100u32).chain([0x7fff, 0x8000, 0xffe1, 0xffff]) {
        let mut m = Machine::new();
        m.wr(FP + 8, Size::W, k);
        m.run(&ti);
        assert_eq!(
            m.rd(GW, Size::W) == 1,
            (1..=6).contains(&(k % 32)),
            "integer {k:#x}"
        );
        m.wr(REC, Size::W, k);
        m.wr(FP + 8, Size::L, REC);
        m.run(&tu);
        assert_eq!(
            m.rd(GW, Size::W) == 1,
            (1..=10).contains(&(k % 32)),
            "pinteger {k:#x}"
        );
    }
    for k in 0..16u32 {
        let mut m = Machine::new();
        m.wr(FP + 8, Size::W, k);
        m.run(&te);
        assert_eq!(
            m.rd(GW, Size::W) == 1,
            [0, 3, 4, 5, 6].contains(&k),
            "n16 {k}"
        );
    }
}

/// A constructor's range with run-time bounds (beads domain-pascal-zif8):
/// the top, the set cleared by a counted loop, the count, the top no
/// higher than the set's last member, `sub.w` of the bottom and `bcs`
/// past the loop, the members set downward (SAU8 MST_$ALLOC_ASID
/// 0x3C4AE96..0x3C4AEDC; in the image into a frame temporary).
#[test]
fn run_time_ranges() {
    // Best effort: in place, where the image uses a frame temporary.
    let was = dpc_syntax::diag::set_strict(false);
    let decls = "var g128: extern set of 0..127; g256: extern set of 0..255;
  lo, n: extern integer;
";
    assert_eq!(
        body(&format!(
            "{decls}procedure t; begin g128 := [lo .. lo + n - 1] end;"
        )),
        [
            "move.l\t%d2,-(%sp)",
            "move.w\t(LO).l,%d0",
            "lea\t(G128).l,%a0",
            "add.w\t(N).l,%d0",
            "moveq\t#3,%d1",
            "subq.w\t#1,%d0",
            ".balignw\t4,0x2048",
            "1:",
            "clr.l\t(%a0)+",
            "dbf\t%d1,1b",
            "move.w\t%d0,%d1",
            "cmpi.w\t#0x7f,%d0",
            "bls.s\t2f",
            "moveq\t#0x7f,%d0",
            "move.w\t%d0,%d1",
            "2:",
            "lea\t(G128).l,%a0",
            "sub.w\t(LO).l,%d1",
            "bcs.s\t3f",
            ".balignw\t4,0x2048",
            "4:",
            "move.w\t%d0,%d2",
            "lsr.w\t#3,%d2",
            "neg.w\t%d2",
            "bset\t%d0,(0xf,%a0,%d2.w)",
            "subq.w\t#1,%d0",
            "dbf\t%d1,4b",
            "3:",
            "move.l\t(-4,%fp),%d2"
        ]
    );
    // A 32-byte set: the top no higher than 255 with `move.w #255`
    // (MST_$FREE_ASID 0x3C7F5FE..0x3C7F604); `s - [r]`: `bclr`, no
    // clearing.
    let b = body(&format!(
        "{decls}procedure t; begin g256 := g256 - [lo .. n] end;"
    ));
    assert!(b.contains(&"move.w\t#0xff,%d0".to_string()), "{b:#?}");
    assert!(
        b.iter().any(|l| l.starts_with("bclr\t%d0,(0x1f,")),
        "{b:#?}"
    );
    assert!(!b.iter().any(|l| l.starts_with("clr.l")), "{b:#?}");
    // A 16-byte set widened into a 32-byte one: its leading 16 bytes
    // cleared, the set copied after them (bead domain-pascal-35ph; MST_$FREE_ASID
    // 0x3C7F664..0x3C7F688).
    let b = body(&format!("{decls}procedure t; begin g256 := g128 end;"));
    assert_eq!(
        b,
        [
            "clr.l\t(G256).l",
            "clr.l\t(G256+4).l",
            "clr.l\t(G256+8).l",
            "clr.l\t(G256+0xc).l",
            "movea.l\t#G128,%a0",
            "lea\t(G256+0x10).l,%a1",
            "move.l\t(%a0)+,(%a1)+",
            "move.l\t(%a0)+,(%a1)+",
            "move.l\t(%a0)+,(%a1)+",
            "move.l\t(%a0)+,(%a1)+"
        ]
    );
    dpc_syntax::diag::set_strict(was);
}
