//! Instruction selection: IR → m68k instructions (CLAUDE.md 4.4).
//!
//! Pattern-directed, one IR node at a time, with a fixed register policy:
//!
//! - only the scratch registers D0, D1, A0, A1 are allocated
//!   (codegen-patterns 2), each pool in ascending order, so no
//!   callee-saved register is ever used and no `movem` is emitted;
//! - an expression that needs more registers than that is reported as not
//!   supported yet.
//!
//! Each pattern cites the image address(es) it reproduces.  Patterns the
//! image has not shown are rejected, not guessed.

use dpc_ir::{Addr, Expr, Function, ResultClass, StmtKind, Width};
use dpc_syntax::{Diagnostic, Span};

use crate::insn::{Cond, Ea, Insn, Reg, Size};
use crate::M68k;

type Res<T> = Result<T, Diagnostic>;

fn unsupported(span: Span, what: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::error(span, format!("{what} not supported yet"))
}

fn size_of(w: Width) -> Size {
    match w {
        Width::Byte => Size::B,
        Width::Word => Size::W,
        Width::Long => Size::L,
    }
}

/// The frame pointer, A6.
const FP: u8 = 6;
/// Scratch data and address registers, in allocation order.
const SCRATCH_D: [u8; 2] = [0, 1];
const SCRATCH_A: [u8; 2] = [0, 1];

/// Select the instructions of one routine, prologue to `rts`.
pub fn select(f: &Function) -> Result<Vec<Insn>, Diagnostic> {
    let mut s = Selector {
        out: Vec::new(),
        d_busy: [false; 2],
        a_busy: [false; 2],
        span: f.span,
    };
    // Prologue (codegen-patterns 2): `link.w A6,#-n`; no `movem`, since
    // only scratch registers are used (TIME_$Q_INIT_QUEUE 0xE16C5E,
    // PROC1_$TST_LOCK 0xE148CA).
    let link = M68k::link_size(f.frame_size);
    if link > 0x8000 {
        return Err(unsupported(
            f.span,
            format!("a {link}-byte frame (beyond `link.w`) is"),
        ));
    }
    s.out.push(Insn::Link {
        disp: -(link as i32),
    });
    for st in &f.body {
        s.span = st.span;
        match &st.kind {
            StmtKind::SetResult(e) => s.set_result(f, e)?,
        }
    }
    // Epilogue: `unlk A6; rts` (0xE148E2, 0xE16C7C).
    s.out.push(Insn::Unlk);
    s.out.push(Insn::Rts);
    Ok(s.out)
}

struct Selector {
    out: Vec<Insn>,
    d_busy: [bool; 2],
    a_busy: [bool; 2],
    /// The statement being selected, for diagnostics.
    span: Span,
}

impl Selector {
    fn alloc(busy: &mut [bool; 2], pool: &[u8; 2], span: Span) -> Res<u8> {
        for (i, b) in busy.iter_mut().enumerate() {
            if !*b {
                *b = true;
                return Ok(pool[i]);
            }
        }
        Err(unsupported(
            span,
            "an expression needing more than two scratch registers of a kind is",
        ))
    }

    fn alloc_d(&mut self) -> Res<u8> {
        Self::alloc(&mut self.d_busy, &SCRATCH_D, self.span)
    }

    fn alloc_a(&mut self) -> Res<u8> {
        Self::alloc(&mut self.a_busy, &SCRATCH_A, self.span)
    }

    fn free_d(&mut self, r: u8) {
        let i = SCRATCH_D.iter().position(|&x| x == r).expect("scratch D");
        debug_assert!(self.d_busy[i]);
        self.d_busy[i] = false;
    }

    fn free_a(&mut self, r: u8) {
        let i = SCRATCH_A.iter().position(|&x| x == r).expect("scratch A");
        debug_assert!(self.a_busy[i]);
        self.a_busy[i] = false;
    }

    /// Release the scratch address register an effective address uses.
    fn free_ea(&mut self, ea: &Ea) {
        match ea {
            Ea::Ind(a) | Ea::Disp(_, a) if *a != FP => self.free_a(*a),
            Ea::Reg(Reg::A(a)) => self.free_a(*a),
            Ea::Reg(Reg::D(d)) => self.free_d(*d),
            _ => {}
        }
    }

    /// `f := e` as the last statement: the value goes straight to the
    /// result register (PROC1_$TST_LOCK 0xE148E0 `move.b D1,D0`; the
    /// frame slot of the result variable is not used).
    fn set_result(&mut self, f: &Function, e: &Expr) -> Res<()> {
        let res = f.result.expect("SetResult in a function");
        match res.class {
            ResultClass::Data => {}
            _ => return Err(unsupported(self.span, "address-register results are")),
        }
        let r = self.eval_d(e)?;
        if r != 0 {
            // The value is moved at the result's width; the upper bytes
            // of D0 are left as they are (codegen-patterns 3: 0xE148E0).
            self.out.push(Insn::Move {
                size: size_of(res.width),
                src: Ea::Reg(Reg::D(r)),
                dst: Ea::Reg(Reg::D(0)),
            });
        }
        self.free_d(r);
        Ok(())
    }

    /// Evaluate `e` into a scratch data register.
    fn eval_d(&mut self, e: &Expr) -> Res<u8> {
        match e {
            // Load a scalar: `move.<w> <ea>,Dn` (0xE148D4 `move.w
            // (8,A6),D0`; 0xE148D8 `move.l (0x40,A0),D1`).  The address is
            // formed first, its register released, then Dn allocated.
            Expr::Load { addr, width } => {
                let ea = self.ea_of(addr)?;
                self.free_ea(&ea);
                let r = self.alloc_d()?;
                self.out.push(Insn::Move {
                    size: size_of(*width),
                    src: ea,
                    dst: Ea::Reg(Reg::D(r)),
                });
                Ok(r)
            }
            Expr::SetMember {
                elem,
                set,
                set_width: Width::Long,
            } => self.set_member_long(elem, set),
            Expr::SetMember { .. } => Err(unsupported(
                self.span,
                "`in` on a set that is not one longword is",
            )),
            // `moveq` / `clr` / `move #imm` choice not catalogued yet.
            Expr::Const { .. } => Err(unsupported(self.span, "loading a constant is")),
        }
    }

    /// `elem in set` for a one-longword set in memory, as a boolean in a
    /// data register.  PROC1_$TST_LOCK 0xE148CE..0xE148DE:
    ///
    /// ```text
    /// movea.l PROC1_$CURRENT_PCB.l,A0   ; the set's address first
    /// move.w  (8,A6),D0                 ; then the element
    /// move.l  (0x40,A0),D1              ; then the set's value
    /// btst    D0,D1                     ; register btst: bit mod 32
    /// sne     D1                        ; boolean in the set's register
    /// ```
    ///
    /// The order (the set operand's address before the element) is seen
    /// at this one site only (codegen-patterns 5.1).
    fn set_member_long(&mut self, elem: &Expr, set: &Expr) -> Res<u8> {
        let Expr::Load {
            addr,
            width: Width::Long,
        } = set
        else {
            return Err(unsupported(
                self.span,
                "`in` on a set that is not a variable is",
            ));
        };
        let ea = self.ea_of(addr)?;
        let bit = self.eval_d(elem)?;
        self.free_ea(&ea);
        let r = self.alloc_d()?;
        self.out.push(Insn::Move {
            size: Size::L,
            src: ea,
            dst: Ea::Reg(Reg::D(r)),
        });
        self.out.push(Insn::Btst {
            bit,
            dst: Ea::Reg(Reg::D(r)),
        });
        self.free_d(bit);
        self.out.push(Insn::Scc {
            cond: Cond::Ne,
            dst: Ea::Reg(Reg::D(r)),
        });
        Ok(r)
    }

    /// Evaluate a pointer into a scratch address register.
    fn eval_a(&mut self, e: &Expr) -> Res<u8> {
        match e {
            // `movea.l <ea>,An`: 0xE148CE `movea.l abs.l,A0` (an extern
            // pointer variable); LOG_$READ 0xE17810 `movea.l (0xc,A6),A0`
            // (a by-address parameter).
            Expr::Load {
                addr,
                width: Width::Long,
            } => {
                let ea = self.ea_of(addr)?;
                // TODO(codegen): reusing the base register for a pointer
                // chain (`movea.l (d,A0),A0`) follows from releasing it
                // first; no chain has been checked against the image yet.
                self.free_ea(&ea);
                let a = self.alloc_a()?;
                self.out.push(Insn::Movea {
                    size: Size::L,
                    src: ea,
                    dst: a,
                });
                Ok(a)
            }
            _ => Err(unsupported(self.span, "this pointer expression is")),
        }
    }

    /// The effective address of `addr`; any address register it needs is
    /// loaded now and stays allocated until [`Self::free_ea`].
    fn ea_of(&mut self, addr: &Addr) -> Res<Ea> {
        match addr {
            // Frame objects and arguments: `(d,A6)` (0xE148D4).
            Addr::Frame(o) => Ok(Ea::Disp(*o, FP)),
            // Other units' objects: absolute long (0xE148CE).
            Addr::Symbol { name, offset } => Ok(Ea::AbsL(name.clone(), *offset)),
            // Through a pointer: `(d,An)` (0xE148D8), `(An)` when d = 0
            // (LOG_$READ 0xE17814 `move.w (A0),-(SP)`).
            Addr::Indirect { base, offset } => {
                if !(-0x8000..0x8000).contains(offset) {
                    return Err(unsupported(self.span, "a displacement beyond 16 bits is"));
                }
                let a = self.eval_a(base)?;
                Ok(if *offset == 0 {
                    Ea::Ind(a)
                } else {
                    Ea::Disp(*offset, a)
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpc_ir::{ResultSpec, Stmt};
    use dpc_syntax::SourceMap;

    fn span() -> Span {
        let f = SourceMap::new().add("t.pas", String::new());
        Span::new(f, 0, 0)
    }

    fn func(frame_size: u32, result: Option<ResultSpec>, body: Vec<StmtKind>) -> Function {
        Function {
            symbol: "F".into(),
            global: true,
            code_section: ".text".into(),
            frame_size,
            result,
            body: body
                .into_iter()
                .map(|kind| Stmt { kind, span: span() })
                .collect(),
            span: span(),
        }
    }

    fn text(f: &Function) -> Vec<String> {
        select(f).unwrap().iter().map(|i| i.to_string()).collect()
    }

    fn load(addr: Addr, width: Width) -> Expr {
        Expr::Load { addr, width }
    }

    #[test]
    fn empty_procedure() {
        assert_eq!(
            text(&func(0, None, vec![])),
            ["link.w\t%fp,#0", "unlk\t%fp", "rts"]
        );
    }

    #[test]
    fn link_rounds_frame_up_to_4() {
        for (raw, disp) in [(1, "#-4"), (4, "#-4"), (5, "#-8"), (12, "#-0xc")] {
            assert_eq!(
                text(&func(raw, None, vec![]))[0],
                format!("link.w\t%fp,{disp}")
            );
        }
    }

    #[test]
    fn tst_lock_shape() {
        let pcb = load(
            Addr::Symbol {
                name: "PROC1_$CURRENT_PCB".into(),
                offset: 0,
            },
            Width::Long,
        );
        let e = Expr::SetMember {
            elem: Box::new(load(Addr::Frame(8), Width::Word)),
            set: Box::new(load(
                Addr::Indirect {
                    base: Box::new(pcb),
                    offset: 0x40,
                },
                Width::Long,
            )),
            set_width: Width::Long,
        };
        let f = func(
            1,
            Some(ResultSpec {
                class: ResultClass::Data,
                width: Width::Byte,
            }),
            vec![StmtKind::SetResult(e)],
        );
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#-4",
                "movea.l\t(PROC1_$CURRENT_PCB).l,%a0",
                "move.w\t(8,%fp),%d0",
                "move.l\t(0x40,%a0),%d1",
                "btst\t%d0,%d1",
                "sne\t%d1",
                "move.b\t%d1,%d0",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    #[test]
    fn result_already_in_d0_is_not_moved() {
        let f = func(
            0,
            Some(ResultSpec {
                class: ResultClass::Data,
                width: Width::Long,
            }),
            vec![StmtKind::SetResult(load(
                Addr::Indirect {
                    base: Box::new(load(Addr::Frame(8), Width::Long)),
                    offset: 0,
                },
                Width::Long,
            ))],
        );
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#0",
                "movea.l\t(8,%fp),%a0",
                "move.l\t(%a0),%d0",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    #[test]
    fn constants_are_rejected() {
        let f = func(
            0,
            Some(ResultSpec {
                class: ResultClass::Data,
                width: Width::Word,
            }),
            vec![StmtKind::SetResult(Expr::Const {
                value: 0,
                width: Width::Word,
            })],
        );
        let d = select(&f).unwrap_err();
        assert_eq!(d.message, "loading a constant is not supported yet");
    }

    #[test]
    fn oversized_frame_is_rejected() {
        let d = select(&func(0x8001, None, vec![])).unwrap_err();
        assert!(d.message.ends_with("not supported yet"), "{}", d.message);
    }
}
