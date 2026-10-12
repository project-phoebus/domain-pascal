//! Instruction selection: IR → m68k instructions (CLAUDE.md 4.4).
//!
//! Two phases per routine:
//!
//! 1. **Body.**  Pattern-directed, one IR node at a time, over physical
//!    registers handed out by [`Regs`] (`regs.rs`).  The default policy is
//!    scratch only, D0, D1 / A0, A1 in that order (codegen-patterns 2,
//!    12); an expression that needs more is reported as not supported
//!    yet.  A pattern that must keep a value across a call asks for a
//!    callee-saved register with [`Selector::hold`]; one that loads A5
//!    reports it with [`Regs::clobber`].
//! 2. **Frame.**  The callee-saved registers the body wrote decide the
//!    save set; [`crate::frame::Frame`] then gives the prologue (`link.w`,
//!    the save) and the epilogue (the restore from the frame, `unlk`,
//!    `rts`) around the body.
//!
//! Each pattern cites the image address(es) it reproduces.  Patterns the
//! image has not shown are rejected, not guessed.

use dpc_ir::{Addr, Expr, Function, ResultClass, Scalar, StmtKind, Width};
use dpc_syntax::{Diagnostic, Span};

use crate::frame::{frame_for, FP, SP};
use crate::insn::{Cond, Ea, Insn, Reg, RegSet, Size};
use crate::regs::{RegClass, Regs, DATA_BASE};

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

/// Select the instructions of one routine, prologue to `rts`.
pub fn select(f: &Function) -> Result<Vec<Insn>, Diagnostic> {
    let body = select_body(f)?;
    wrap(f, body)
}

/// Phase 2: the frame of `f` around `body`.
pub fn wrap(f: &Function, body: Body) -> Result<Vec<Insn>, Diagnostic> {
    let frame = frame_for(f.frame_size, body.saves, f.span)?;
    let mut out = frame.prologue();
    out.extend(body.insns);
    out.extend(frame.epilogue());
    Ok(out)
}

/// Phase 1's result: the body's instructions and the callee-saved
/// registers they write.
#[derive(Debug)]
pub struct Body {
    pub insns: Vec<Insn>,
    pub saves: RegSet,
}

/// Phase 1: the body of `f`, without prologue and epilogue.
pub fn select_body(f: &Function) -> Result<Body, Diagnostic> {
    let mut s = Selector {
        out: Vec::new(),
        regs: Regs::new(),
        span: f.span,
    };
    for st in &f.body {
        s.span = st.span;
        match &st.kind {
            StmtKind::SetResult(e) => s.set_result(f, e)?,
        }
    }
    Ok(Body {
        insns: s.out,
        saves: s.regs.to_save(),
    })
}

/// Selection state of one routine body.
pub struct Selector {
    out: Vec<Insn>,
    regs: Regs,
    /// The statement being selected, for diagnostics.
    span: Span,
}

impl Selector {
    fn alloc(&mut self, class: RegClass) -> Res<Reg> {
        self.regs.alloc_scratch(class).ok_or_else(|| {
            unsupported(
                self.span,
                "an expression needing more than two scratch registers of a kind is",
            )
        })
    }

    fn alloc_d(&mut self) -> Res<u8> {
        match self.alloc(RegClass::Data)? {
            Reg::D(n) => Ok(n),
            Reg::A(_) => unreachable!(),
        }
    }

    fn alloc_a(&mut self) -> Res<u8> {
        match self.alloc(RegClass::Addr)? {
            Reg::A(n) => Ok(n),
            Reg::D(_) => unreachable!(),
        }
    }

    /// A callee-saved register of `class` for a value that must survive
    /// a call (`regs.rs`, [`Regs::alloc_held`]); it joins the prologue's
    /// save set.  Free it with [`Self::free`].
    pub fn hold(&mut self, class: RegClass) -> Res<Reg> {
        self.regs.alloc_held(class).ok_or_else(|| {
            unsupported(
                self.span,
                "holding more values than there are callee-saved registers is",
            )
        })
    }

    /// Release a register from [`Self::hold`] or the scratch pools.
    pub fn free(&mut self, r: Reg) {
        self.regs.free(r);
    }

    fn free_d(&mut self, r: u8) {
        self.regs.free(Reg::D(r));
    }

    fn free_a(&mut self, r: u8) {
        self.regs.free(Reg::A(r));
    }

    /// Release the register an effective address uses.  A5, A6 and A7
    /// are bases the allocator never hands out (`regs.rs`), so they are
    /// not released.
    fn free_ea(&mut self, ea: &Ea) {
        match ea {
            Ea::Ind(a) | Ea::Disp(_, a) if Reg::A(*a) != DATA_BASE && *a != FP && *a != SP => {
                self.free_a(*a)
            }
            Ea::Reg(r) => self.regs.free(*r),
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
                size: size_of(res.ty.width),
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
            Expr::Load { addr, ty } => {
                let ea = self.ea_of(addr)?;
                self.free_ea(&ea);
                let r = self.alloc_d()?;
                self.out.push(Insn::Move {
                    size: size_of(ty.width),
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
            ty: Scalar {
                width: Width::Long, ..
            },
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
                ty: Scalar {
                    width: Width::Long, ..
                },
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

    fn load(addr: Addr, ty: Scalar) -> Expr {
        Expr::Load { addr, ty }
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
            Scalar::POINTER,
        );
        let e = Expr::SetMember {
            elem: Box::new(load(Addr::Frame(8), Scalar::signed(Width::Word))),
            set: Box::new(load(
                Addr::Indirect {
                    base: Box::new(pcb),
                    offset: 0x40,
                },
                Scalar::unsigned(Width::Long),
            )),
            set_width: Width::Long,
        };
        let f = func(
            1,
            Some(ResultSpec {
                class: ResultClass::Data,
                ty: Scalar::BOOLEAN,
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
                ty: Scalar::signed(Width::Long),
            }),
            vec![StmtKind::SetResult(load(
                Addr::Indirect {
                    base: Box::new(load(Addr::Frame(8), Scalar::POINTER)),
                    offset: 0,
                },
                Scalar::signed(Width::Long),
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
                ty: Scalar::signed(Width::Word),
            }),
            vec![StmtKind::SetResult(Expr::Const {
                value: 0,
                ty: Scalar::signed(Width::Word),
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

    /// Phase 2 puts the save the body needs between `link` and the body,
    /// and the restore from the frame before `unlk`.
    #[test]
    fn two_phase_frame_around_a_body_with_held_registers() {
        let f = func(16, None, vec![]);
        // A synthetic body: a value held across a (pretend) call.
        let mut s = Selector {
            out: Vec::new(),
            regs: Regs::new(),
            span: span(),
        };
        let d = s.hold(RegClass::Data).unwrap();
        let a = s.hold(RegClass::Addr).unwrap();
        let d2 = s.hold(RegClass::Data).unwrap();
        assert_eq!((d, a, d2), (Reg::D(2), Reg::A(2), Reg::D(3)));
        s.out.push(Insn::Move {
            size: Size::L,
            src: Ea::Disp(0xc, FP),
            dst: Ea::Reg(d),
        });
        s.free(d);
        s.free(a);
        s.free(d2);
        let body = Body {
            insns: s.out,
            saves: s.regs.to_save(),
        };
        let text: Vec<String> = wrap(&f, body)
            .unwrap()
            .iter()
            .map(|i| i.to_string())
            .collect();
        // 0xE17778..0xE17780, 0xE177F6..0xE177FE.
        assert_eq!(
            text,
            [
                "link.w\t%fp,#-0x10",
                "movem.l\t%d2-%d3/%a2,-(%sp)",
                "move.l\t(0xc,%fp),%d2",
                "movem.l\t(-0x1c,%fp),%d2-%d3/%a2",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    /// Effective addresses based on A5, A6 or A7 release nothing (a
    /// module-data operand `(d,A5)` must not free a register the
    /// allocator never gave out).
    #[test]
    fn fixed_base_registers_are_not_released() {
        let mut s = Selector {
            out: Vec::new(),
            regs: Regs::new(),
            span: span(),
        };
        for a in [5, FP, SP] {
            s.free_ea(&Ea::Disp(4, a));
            s.free_ea(&Ea::Ind(a));
        }
        assert!(s.regs.live_scratch().is_empty());
        let a = s.alloc_a().unwrap();
        s.free_ea(&Ea::Disp(4, a));
        assert!(!s.regs.is_busy(Reg::A(a)));
    }

    #[test]
    fn scratch_only_bodies_save_nothing() {
        let f = func(
            0,
            Some(ResultSpec {
                class: ResultClass::Data,
                ty: Scalar::signed(Width::Word),
            }),
            vec![StmtKind::SetResult(load(
                Addr::Frame(8),
                Scalar::signed(Width::Word),
            ))],
        );
        let body = select_body(&f).unwrap();
        assert!(body.saves.is_empty());
        assert_eq!(body.insns.len(), 1);
    }
}
