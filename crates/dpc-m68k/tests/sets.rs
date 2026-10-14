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
    let program = dpc_sema::analyze(&unit, &dpc_m68k::M68k).map_err(render)?;
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
            "move.l\t(0x30,%a0),%d0",
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

#[test]
fn refusals() {
    // `s := s * c` in place: `andi` or the mask in a register (0xE1C006).
    assert!(
        error("procedure t(p: rp); options(val_param); begin p^.w := p^.w * [1, 2] end;")
            .contains("`s := s * x` in place")
    );
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
    // Set comparisons other than with `[]`.
    assert!(
        error("procedure t(p: rp); options(val_param); begin if p^.w = [1] then gw := 1 end;")
            .contains("set comparison")
    );
    assert!(error(
        "procedure t(p: rp); options(val_param); begin if p^.w <= p^.w2 then gw := 1 end;"
    )
    .contains("set comparison"));
    // A union as a value.
    assert!(error(
        "procedure t(p: rp); options(val_param); begin gw := 0; p^.w2 := p^.w + p^.w2 end;"
    )
    .contains("union of sets as a value"));
    // A constant member beyond the set's bits.
    assert!(
        error("procedure t(p: rp); options(val_param); begin p^.w := p^.w + [20] end;")
            .contains("outside the 16 bits")
    );
    // `in` on a large set with an element that may lie beyond it (the
    // guarded form, NAME_$INIT 0xE3183E).
    assert!(error(
        "procedure t(p: rp; k: integer); options(val_param);
         begin if k in p^.b64 then gw := 1 end;"
    )
    .contains("guarded test"));
    // ... and as a value.
    assert!(error(
        "function t(p: rp; c: char): boolean; options(val_param);
         begin t := c in p^.big end;"
    )
    .contains("as a value"));
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
