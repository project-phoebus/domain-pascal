//! Packed records (ref 3.10.7; codegen-patterns 5.3), end to end from
//! Pascal source: the instruction forms, the refusals, and the meaning of
//! the code.  The last is checked by running the selected instructions
//! on a small model of the 68010 (the few instructions bit fields use):
//! a store must leave every other bit of the record as it was and put the
//! value in the field's bits; a load must give the field's value.

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
        ds.iter().map(|d| d.render(&sm, "dpc")).collect()
    };
    let program = dpc_sema::analyze(&unit, &dpc_m68k::M68k).map_err(render)?;
    dpc_ir::lower(&program).map_err(render)
}

/// Compile `src` to assembly lines without comments and blank lines, or
/// the rendered diagnostics.
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

/// A record with a field in each kind of unit and position: the byte
/// offsets and units are given per field (ref 3.10.7 layout; units by
/// `lower.rs`, `bit_field`).
const DECLS: &str = "module m;
type
  colour = (c0, c1, c2, c3, c4);
  r = packed record
    b7: boolean;          { bit 0: byte 0, bit 7 }
    e3: colour;           { bits 1..3: byte 0, bits 6..4 }
    b3: boolean;          { bit 4: byte 0, bit 3 }
    n3: 0..7;             { bits 5..7: byte 0, bits 2..0 }
    w9: 0..511;           { bits 8..16: the longword at 0, bits 23..15 }
    hi: 0..127;           { bits 17..23: byte 2, bits 6..0 }
    u16: 0..65535;        { bits 24..39: the longword at 2, bits 23..8 }
    b0: boolean;          { bit 40: byte 5, bit 7 }
    m12: 0..4095;         { bits 41..52: the longword at 4, bits 22..11 }
    top: 0..15;           { bits 53..56: the word at 6, bits 10..7 }
    l20: 0..1048575;      { bits 57..76: the longword at 6, bits 22..3 }
    w12: 0..4095;         { bits 77..88: the longword at 8, bits 18..7 }
    c: char;              { bits 96..103: byte 12, a whole byte }
    i: integer;           { bits 112..127: the word at 14, a whole word }
  end;
  rp = ^r;
var
  gw: extern integer;
  gb: extern boolean;
";

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

/// [`body`] with the best-effort forms taken (not `--strict`; bead
/// domain-pascal-5kgg), and their warnings, which contain `warns`.
fn lax_body(routine: &str, warns: &str) -> Vec<String> {
    let was = dpc_syntax::diag::set_strict(false);
    let b = std::panic::catch_unwind(|| body(routine));
    dpc_syntax::diag::set_strict(was);
    let w = dpc_syntax::diag::take_best_effort();
    let b = b.unwrap_or_else(|e| std::panic::resume_unwind(e));
    assert!(w.iter().any(|d| d.message.contains(warns)), "{w:?}");
    b
}

fn error(routine: &str) -> String {
    let e = compile(&format!("{DECLS}{routine}")).unwrap_err();
    assert_eq!(e.len(), 1, "{e:?}");
    e[0].clone()
}

#[test]
fn boolean_fields() {
    // PROC2_$RESUME 0xE41444 `bset #7,(-0x4,A6)`; 0xE00E02 `bclr #6`.
    assert_eq!(
        body("procedure t(p: rp); options(val_param); begin p^.b7 := true end;"),
        ["movea.l\t(8,%fp),%a0", "bset\t#7,(%a0)"]
    );
    assert_eq!(
        body("procedure t(p: rp); options(val_param); begin p^.b3 := false end;"),
        ["movea.l\t(8,%fp),%a0", "bclr\t#3,(%a0)"]
    );
    // XPD_$INHERIT_PTRACE_OPTIONS 0xE5B17C `btst #3,(13,A0); sne D0`;
    // bit 7 as the sign, 0xE066B2 `tst.b (185,A0); smi D0`.
    assert_eq!(
        body("function t(p: rp): boolean; options(val_param); begin t := p^.b3 end;"),
        ["movea.l\t(8,%fp),%a0", "btst\t#3,(%a0)", "sne\t%d0"]
    );
    assert_eq!(
        body("function t(p: rp): boolean; options(val_param); begin t := p^.b0 end;"),
        ["movea.l\t(8,%fp),%a0", "tst.b\t(5,%a0)", "smi\t%d0"]
    );
    // Conditions: `btst; beq` (0xE0088A), `tst.b; bpl` for bit 7.
    assert_eq!(
        body("procedure t(var x: r); begin if x.b3 then gw := 1 end;"),
        [
            "movea.l\t(8,%fp),%a0",
            "btst\t#3,(%a0)",
            "beq.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
    assert_eq!(
        body("procedure t(var x: r); begin if not x.b7 then gw := 1 end;"),
        [
            "movea.l\t(8,%fp),%a0",
            "tst.b\t(%a0)",
            "bmi.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
}

#[test]
fn boolean_values_merged() {
    // Bit 7: the variable read after the `andi` (0xE5BFA2).
    assert_eq!(
        body("procedure t(var x: r); begin x.b7 := gb end;"),
        [
            "movea.l\t(8,%fp),%a0",
            "andi.b\t#0x7f,(%a0)",
            "move.b\t(GB).l,%d0",
            "andi.b\t#0x80,%d0",
            "or.b\t%d0,(%a0)"
        ]
    );
    // Bit 3: made 0 or 1 first (0xE04C5C `move.b (A1),D1; lsr.b #7,D1;
    // andi.b #-9,(11,A2); lsl.b #3,D1; or.b D1,(11,A2)`).
    assert_eq!(
        body("procedure t(var x: r); begin x.b3 := gb end;"),
        [
            "move.b\t(GB).l,%d0",
            "movea.l\t(8,%fp),%a0",
            "lsr.b\t#7,%d0",
            "andi.b\t#0xf7,(%a0)",
            "lsl.b\t#3,%d0",
            "or.b\t%d0,(%a0)"
        ]
    );
    // A computed boolean (OS_$GET_EC 0xE6D6DE..0xE6D6EA: `tst.l (A2);
    // sne D0; andi.b #127,(A2); andi.b #-128,D0; or.b D0,(A2)`, the
    // status held in A2; here a local).
    assert_eq!(
        body("procedure t; var x: r; begin x.b7 := gw <> 0; gb := x.b3 end;")[..5],
        [
            "tst.w\t(GW).l",
            "sne\t%d0",
            "andi.b\t#0x7f,(-0x10,%fp)",
            "andi.b\t#0x80,%d0",
            "or.b\t%d0,(-0x10,%fp)"
        ]
    );
}

#[test]
fn multi_bit_constants() {
    // 0xE158CE `andi.b #63,(16,A0); ori.b #-128,(16,A0)`.
    assert_eq!(
        body("procedure t(var x: r); begin x.e3 := c2 end;"),
        [
            "movea.l\t(8,%fp),%a0",
            "andi.b\t#0x8f,(%a0)",
            "ori.b\t#0x20,(%a0)"
        ]
    );
    // 0 is only the `andi` (0xE1565E); all ones only the `ori` (0xE15656).
    assert_eq!(
        body("procedure t(var x: r); begin x.n3 := 0 end;"),
        ["movea.l\t(8,%fp),%a0", "andi.b\t#0xf8,(%a0)"]
    );
    assert_eq!(
        body("procedure t(var x: r); begin x.n3 := 7 end;"),
        ["movea.l\t(8,%fp),%a0", "ori.b\t#7,(%a0)"]
    );
    // A word's `ori` with a zero low byte is a byte `ori` (0xE008C8
    // `andi.w #-0x7f81,(40,A2); ori.b #1,(40,A2)`).
    assert_eq!(
        body("procedure t(var x: r); begin x.top := 2 end;"),
        [
            "movea.l\t(8,%fp),%a0",
            "andi.w\t#-0x781,(6,%a0)",
            "ori.b\t#1,(6,%a0)"
        ]
    );
    // A longword's is not narrowed (0xE0A1DE `ori.l #16,(8,A0)`).
    assert_eq!(
        body("procedure t(var x: r); begin x.w12 := 1 end;"),
        [
            "movea.l\t(8,%fp),%a0",
            "andi.l\t#-0x7ff81,(8,%a0)",
            "ori.l\t#0x80,(8,%a0)"
        ]
    );
    // Nor is one with a zero low byte whose bits lie in both halves
    // (0xE4A834 `andi.l #255,(50,A0)`, 0xE4A83C `ori.l
    // #0x8001e00,(50,A0)`); bits in one half only are refused under
    // `--strict`, `ori.l` too otherwise (best effort, bead
    // domain-pascal-eoc6).
    assert_eq!(
        body("procedure t(var x: r); begin x.l20 := 16#2020 end;"),
        [
            "movea.l\t(8,%fp),%a0",
            "andi.l\t#-0x7ffff9,(6,%a0)",
            "ori.l\t#0x10100,(6,%a0)"
        ]
    );
    assert!(error("procedure t(var x: r); begin x.l20 := 16#2000 end;").contains("lie in one half"));
    assert_eq!(
        lax_body(
            "procedure t(var x: r); begin x.l20 := 16#2000 end;",
            "lie in one half"
        ),
        [
            "movea.l\t(8,%fp),%a0",
            "andi.l\t#-0x7ffff9,(6,%a0)",
            "ori.l\t#0x10000,(6,%a0)"
        ]
    );
}

#[test]
fn multi_bit_loads() {
    // 0xE158B6 `move.w #192,D0; and.b (16,A0),D0; lsr.w #6,D0`, the
    // mask a `moveq` when it fits (0xE0E00A).
    assert_eq!(
        body("function t(p: rp): integer; options(val_param); begin t := ord(p^.e3) end;"),
        [
            "movea.l\t(8,%fp),%a0",
            "moveq\t#0x70,%d0",
            "and.b\t(%a0),%d0",
            "lsr.w\t#4,%d0"
        ]
    );
    // A longword unit, the shift split at 8 (0xE106A0).
    assert_eq!(
        body("function t(p: rp): integer; options(val_param); begin t := p^.w9 end;"),
        [
            "movea.l\t(8,%fp),%a0",
            "move.l\t#0xff8000,%d0",
            "and.l\t(%a0),%d0",
            "lsr.l\t#8,%d0",
            "lsr.l\t#7,%d0"
        ]
    );
    // Against 0: the flags of the `and` or the shift (0xE5BFBE).
    assert_eq!(
        body("procedure t(var x: r); begin if x.n3 = 0 then gw := 1 end;"),
        [
            "movea.l\t(8,%fp),%a0",
            "moveq\t#7,%d0",
            "and.b\t(%a0),%d0",
            "bne.s\t1f",
            "move.w\t#1,(GW).l",
            "1:"
        ]
    );
}

#[test]
fn multi_bit_values_merged() {
    // A word variable into a byte unit: its low byte, read after the
    // `andi` (0xE1D220 `andi.b #15,(-6,A6); move.b (133,A3),D1; lsl.b
    // #4,D1; or.b D1,(-6,A6)`); `integer' admits negative values, so the
    // value is masked (0xE31DF2).
    assert_eq!(
        body("procedure t(var x: r); begin x.e3 := colour(gw) end;"),
        [
            "movea.l\t(8,%fp),%a0",
            "andi.b\t#0x8f,(%a0)",
            "move.b\t(GW+1).l,%d0",
            "lsl.b\t#4,%d0",
            "or.b\t%d0,(%a0)"
        ]
    );
    assert_eq!(
        body("procedure t(var x: r); begin x.n3 := gw end;"),
        [
            "movea.l\t(8,%fp),%a0",
            "andi.b\t#0xf8,(%a0)",
            "move.b\t(GW+1).l,%d0",
            "andi.b\t#7,%d0",
            "or.b\t%d0,(%a0)"
        ]
    );
    // A word into a longword unit: zero-extended first (0xE0E346 `clr.l
    // D0; move.w (-80,A6),D0; andi.l ...; lsl.l #8,D0; or.l`).
    assert_eq!(
        body("procedure t(var x: r; in v: integer); options(val_param); begin x.u16 := v end;"),
        [
            "clr.l\t%d0",
            "movea.l\t(8,%fp),%a0",
            "move.w\t(0xc,%fp),%d0",
            "andi.l\t#-0xffff01,(2,%a0)",
            "lsl.l\t#8,%d0",
            "andi.l\t#0xffff00,%d0",
            "or.l\t%d0,(2,%a0)"
        ]
    );
}

#[test]
fn whole_fields_are_plain() {
    assert_eq!(
        body("procedure t(var x: r); begin x.c := 'a'; x.i := gw end;"),
        [
            "movea.l\t(8,%fp),%a0",
            "move.b\t#0x61,(0xc,%a0)",
            "move.w\t(GW).l,(0xe,%a0)"
        ]
    );
}

#[test]
fn refusals() {
    assert!(error(
        "procedure s(var b: boolean); extern;\nprocedure t(var x: r); begin s(x.b3) end;"
    )
    .contains("cannot be passed to a `var`"));
    assert!(
        error("procedure t(var x: r); var u: univ_ptr; begin u := addr(x.n3) end;")
            .contains("has no address")
    );
    assert!(
        error("procedure t; var x, y: r; begin x.n3 := y.n3; gw := x.i end;")
            .contains("copying a packed field that is not a boolean")
    );
    assert!(error("procedure t(var x: r); begin x.b7 := gw <> 0 end;")
        .contains("destination and value both need a register"));
    assert!(error(
        "type sr = packed record s: -8..7 end;\nprocedure t(var x: sr); begin gw := x.s end;"
    )
    .contains("negative values"));
    assert!(error(
        "type br = packed record a: 0..1; b: 0..1 end;\nprocedure t(var x: br); begin x.b := 1 end;"
    )
    .contains("one-bit packed field that is not a boolean"));
    let set = "type sr = packed record a: boolean; s: set of 0..3 end;\n";
    assert!(error(&format!(
        "{set}procedure t(var x: sr); begin x.s := [1] end;"
    ))
    .contains("not a whole byte, word or longword"));
}

/// An 8-bit field of a word type on a byte boundary is the byte as a
/// `[byte] 0..255` (bead domain-pascal-eoc6; DBUF 0xE3AC6A `clr.b
/// (13,A2)`; `status_$t.modc` 0xE1AAB8 `clr.w D0; move.b (1,A0),D0`).
#[test]
fn byte_fields() {
    let fill = "type yr = packed record x: 0..255; y: 0..255 end;\n";
    assert_eq!(
        body(&format!(
            "{fill}procedure t(var x: yr); begin x.y := 0 end;"
        )),
        ["movea.l\t(8,%fp),%a0", "clr.b\t(1,%a0)"]
    );
    assert_eq!(
        body(&format!(
            "{fill}procedure t(var x: yr); begin gw := x.y end;"
        )),
        [
            "movea.l\t(8,%fp),%a0",
            "clr.w\t%d0",
            "move.b\t(1,%a0),%d0",
            "move.w\t%d0,(GW).l"
        ]
    );
}

/// Forms accepted best effort, refused under `--strict` (above; bead
/// domain-pascal-eoc6): no site; their meaning is checked by
/// tests/exec/bit_fields.pas.
#[test]
fn best_effort_forms() {
    // A copy between bit fields: the value extracted, merged (5kgg's
    // form, unmasked: the value is zero-extended from its bits).
    assert_eq!(
        lax_body(
            "procedure t; var x, y: r; begin x.n3 := y.n3; gw := x.i end;",
            "copying a packed field"
        )[..4],
        [
            "moveq\t#7,%d0",
            "and.b\t(-0x20,%fp),%d0",
            "andi.b\t#0xf8,(-0x10,%fp)",
            "or.b\t%d0,(-0x10,%fp)"
        ]
    );
    // A field of negative values: zero-extended, then `lsl`/`asr`.
    assert_eq!(
        lax_body(
            "type sr = packed record s: -8..7 end;\nprocedure t(var x: sr); begin gw := x.s end;",
            "negative values"
        ),
        [
            "movea.l\t(8,%fp),%a0",
            "move.w\t#0xf0,%d0",
            "and.b\t(%a0),%d0",
            "lsr.w\t#4,%d0",
            "lsl.w\t#8,%d0",
            "lsl.w\t#4,%d0",
            "asr.w\t#8,%d0",
            "asr.w\t#4,%d0",
            "move.w\t%d0,(GW).l"
        ]
    );
    // A set field: a constant member as a boolean field's bit, a
    // run-time one on the value extracted (a `moveq` mask: the register
    // all clear above it).
    let set = "type sr = packed record a: boolean; s: set of 0..3 end;\n";
    assert_eq!(
        lax_body(
            &format!("{set}procedure t(var x: sr); begin gb := 2 in x.s end;"),
            "set of 0..3"
        ),
        [
            "movea.l\t(8,%fp),%a0",
            "btst\t#5,(%a0)",
            "sne\t%d0",
            "move.b\t%d0,(GB).l"
        ]
    );
    assert_eq!(
        lax_body(
            &format!("{set}procedure t(var x: sr); begin gb := gw in x.s end;"),
            "set of 0..3"
        ),
        [
            "move.w\t(GW).l,%d0",
            "movea.l\t(8,%fp),%a0",
            "moveq\t#0x78,%d1",
            "and.b\t(%a0),%d1",
            "lsr.w\t#3,%d1",
            "btst\t%d0,%d1",
            "sne\t%d1",
            "move.b\t%d1,(GB).l"
        ]
    );
}

// ---------------------------------------------------------------------
// The meaning of the code: a model of the instructions used.
// ---------------------------------------------------------------------

const FP: u32 = 0x1000;
const REC: u32 = 0x2000;
const GW: u32 = 0x3000;

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
        Machine {
            mem: HashMap::new(),
            d: [0; 8],
            a,
            z: false,
            n: false,
        }
    }
    fn rd(&self, at: u32, s: Size) -> u32 {
        // MC68010: a word or longword access at an odd address is an
        // address error.
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
            Ea::AbsL(s, o) => {
                assert_eq!(s, "GW", "symbol {s}");
                GW.wrapping_add(*o as u32)
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
    fn run(&mut self, code: &[Insn]) {
        for i in code {
            match i {
                Insn::Link { .. } | Insn::MovemSave { .. } | Insn::MovemRestore { .. } => {}
                Insn::Unlk | Insn::Rts => return,
                Insn::Movea { size, src, dst } => self.a[*dst as usize] = self.get(src, *size),
                Insn::Move { size, src, dst } => {
                    let v = self.get(src, *size);
                    self.put(dst, *size, v);
                    self.flags(v, *size);
                }
                Insn::Moveq { value, dst } => self.d[*dst as usize] = *value as i32 as u32,
                Insn::Clr { size, dst } => self.put(dst, *size, 0),
                Insn::AluImm { op, size, imm, dst } => {
                    let a = self.get(dst, *size);
                    let b = *imm as u32;
                    let v = alu(*op, a, b);
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
                    op,
                    size,
                    count: ShiftCount::Imm(k),
                    dst,
                } => {
                    let r = Ea::Reg(Reg::D(*dst));
                    let a = self.get(&r, *size);
                    let v = match op {
                        ShiftOp::Lsl => a << k,
                        ShiftOp::Lsr => a >> k,
                        ShiftOp::Asr => panic!("asr"),
                    };
                    self.put(&r, *size, v);
                    self.flags(v, *size);
                }
                Insn::BitImm { op, bit, dst } => {
                    // The 68010 takes the number mod 8 on a byte in memory
                    // (an unreduced #15 on abs.l or indexed operands, bead
                    // domain-pascal-6zdt).
                    let bit = if matches!(dst, Ea::Reg(_)) { *bit } else { *bit % 8 };
                    let a = self.get(dst, Size::B);
                    self.z = a & (1 << bit) == 0;
                    let v = match op {
                        BitOp::Set => a | (1 << bit),
                        BitOp::Clr => a & !(1 << bit),
                    };
                    self.put(dst, Size::B, v);
                }
                Insn::BtstImm { bit, dst } => {
                    let bit = if matches!(dst, Ea::Reg(_)) { *bit } else { *bit % 8 };
                    self.z = self.get(dst, Size::B) & (1 << bit) == 0;
                }
                Insn::Tst { size, dst } => {
                    let v = self.get(dst, *size);
                    self.flags(v, *size);
                }
                Insn::Scc { cond, dst } => {
                    let t = match cond {
                        Cond::Ne => !self.z,
                        Cond::Eq => self.z,
                        Cond::Mi => self.n,
                        Cond::Pl => !self.n,
                        Cond::T => true,
                        c => panic!("s{c:?}"),
                    };
                    self.put(dst, Size::B, if t { 0xff } else { 0 });
                }
                Insn::Unary {
                    op: UnaryOp::Not,
                    size,
                    dst,
                } => {
                    let v = !self.get(dst, *size);
                    self.put(dst, *size, v);
                }
                i => panic!("not modelled: {i}"),
            }
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

/// The fields of [`DECLS`]'s record: name, bit, width, Pascal value type
/// as a parameter's (`b` boolean, `w` word, `l` longword).
const FIELDS: &[(&str, u32, u32, char)] = &[
    ("b7", 0, 1, 'b'),
    ("e3", 1, 3, 'e'),
    ("b3", 4, 1, 'b'),
    ("n3", 5, 3, 'w'),
    ("w9", 8, 9, 'w'),
    ("hi", 17, 7, 'w'),
    ("u16", 24, 16, 'w'),
    ("b0", 40, 1, 'b'),
    ("m12", 41, 12, 'w'),
    ("top", 53, 4, 'w'),
    ("l20", 57, 20, 'l'),
    ("w12", 77, 12, 'w'),
];

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

fn record_bits(m: &Machine) -> u128 {
    (0..16).fold(0u128, |v, k| {
        (v << 8) | *m.mem.get(&(REC + k)).unwrap_or(&0) as u128
    })
}

fn set_record(m: &mut Machine, bits: u128) {
    for k in 0..16 {
        m.mem.insert(REC + k, (bits >> (8 * (15 - k))) as u8);
    }
}

fn field_of(bits: u128, bit: u32, width: u32) -> u128 {
    (bits >> (128 - bit - width)) & ((1u128 << width) - 1)
}

/// Every field stored from a parameter (by value, `val_param`) and from a
/// constant, and loaded, over random record contents: the other bits of
/// the record are kept, the field gets the value, the load gives it.
///
/// `exact`: the values are of the field's own (non-negative) type, which
/// are merged unmasked; else of `integer`/`integer32`, which are masked.
fn stores_and_loads(exact: bool) {
    let mut src = String::from(DECLS);
    for &(f, _, width, t) in FIELDS {
        if exact && matches!(t, 'w' | 'l') {
            src += &format!("type t_{f} = 0..{};\n", (1u64 << width) - 1);
        }
        let ty = match t {
            'b' => "boolean".to_string(),
            'e' => "colour".to_string(),
            _ if exact => format!("t_{f}"),
            'l' => "integer32".to_string(),
            _ => "integer".to_string(),
        };
        let val = if t == 'e' { "colour(v)" } else { "v" };
        let pty = if t == 'e' { "integer" } else { &ty };
        src += &format!(
            "procedure s_{f}(p: rp; v: {pty}); options(val_param); begin p^.{f} := {val} end;\n\
             function l_{f}(p: rp): {ty}; options(val_param); begin l_{f} := p^.{f} end;\n"
        );
    }
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for &(f, bit, width, t) in FIELDS {
        let store = code(&src, &format!("S_{}", f.to_uppercase()));
        let load = code(&src, &format!("L_{}", f.to_uppercase()));
        for _ in 0..200 {
            let before = ((rng.next() as u128) << 64) | rng.next() as u128;
            let v = rng.next() as u128 & ((1u128 << width) - 1);
            let v = if t == 'e' { v % 5 } else { v };
            let mut m = Machine::new();
            set_record(&mut m, before);
            m.wr(FP + 8, Size::L, REC);
            match t {
                'b' => m.wr(FP + 12, Size::B, if v != 0 { 0xff } else { 0 }),
                'l' => m.wr(FP + 12, Size::L, v as u32),
                _ => m.wr(FP + 12, Size::W, v as u32),
            }
            m.run(&store);
            let after = record_bits(&m);
            let fmask = ((1u128 << width) - 1) << (128 - bit - width);
            assert_eq!(
                after & !fmask,
                before & !fmask,
                "{f}: neighbouring bits changed"
            );
            assert_eq!(field_of(after, bit, width), v, "{f}: stored value");
            m.d = [0x5a5a_5a5a; 8];
            m.run(&load);
            let got = match t {
                'b' => (m.d[0] & 0xff == 0xff) as u128,
                'l' => m.d[0] as u128,
                _ => (m.d[0] & 0xffff) as u128,
            };
            if t == 'b' {
                assert!(matches!(m.d[0] & 0xff, 0 | 0xff), "{f}: a boolean");
            }
            assert_eq!(got, v, "{f}: loaded value");
        }
    }
}

#[test]
fn stores_and_loads_keep_the_neighbouring_bits() {
    stores_and_loads(false);
}

#[test]
fn unmasked_stores_keep_the_neighbouring_bits() {
    stores_and_loads(true);
}

/// Constants into every non-boolean field, over random record contents.
#[test]
fn constant_stores_keep_the_neighbouring_bits() {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    for &(f, bit, width, t) in FIELDS {
        if t == 'b' {
            continue;
        }
        let max: u128 = if t == 'e' { 4 } else { (1u128 << width) - 1 };
        for v in [0, 1, max, max / 2 + 1, (rng.next() as u128) % (max + 1)] {
            let lit = if t == 'e' {
                format!("c{v}")
            } else {
                v.to_string()
            };
            let src = format!(
                "{DECLS}procedure k(p: rp); options(val_param); begin p^.{f} := {lit} end;\n"
            );
            let store = match front(&src) {
                Ok(m) => dpc_m68k::isel::select(&m.functions[0]),
                Err(e) => panic!("{e:?}"),
            };
            let store = match store {
                Ok(c) => c,
                // A longword's `ori` with a zero low byte is not
                // catalogued: refused, not checked.
                Err(e) if e.message.contains("leaves 0") => continue,
                Err(e) => panic!("{f} := {v}: {}", e.message),
            };
            let before = ((rng.next() as u128) << 64) | rng.next() as u128;
            let mut m = Machine::new();
            set_record(&mut m, before);
            m.wr(FP + 8, Size::L, REC);
            m.run(&store);
            let after = record_bits(&m);
            let fmask = ((1u128 << width) - 1) << (128 - bit - width);
            assert_eq!(after & !fmask, before & !fmask, "{f} := {v}: neighbours");
            assert_eq!(field_of(after, bit, width), v, "{f} := {v}");
        }
    }
}

/// A value of a type with negative values is masked to the field: an
/// `integer` of -1 into a 3-bit field gives 7 and leaves the rest.
#[test]
fn negative_values_are_masked() {
    let src = format!(
        "{DECLS}procedure s(p: rp; v: integer); options(val_param); begin p^.n3 := v end;\n"
    );
    let store = code(&src, "S");
    let mut m = Machine::new();
    set_record(&mut m, 0);
    m.wr(FP + 8, Size::L, REC);
    m.wr(FP + 12, Size::W, 0xffff);
    m.run(&store);
    assert_eq!(record_bits(&m), 7u128 << (128 - 8));
    let _ = GW;
}
