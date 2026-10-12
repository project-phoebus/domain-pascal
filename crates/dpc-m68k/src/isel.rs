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

/// A constant as the immediate of a `w`-sized instruction: a byte as
/// 0..=255 (see [`Ea::Imm`]), a word or longword as the signed number.
fn imm(value: i64, w: Width) -> i64 {
    match w {
        Width::Byte => value & 0xff,
        Width::Word => value as i16 as i64,
        Width::Long => value as i32 as i64,
    }
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
///
/// A body of one statement is selected in the statement's own evaluation
/// order (PROC1_$TST_LOCK 0xE148CA: the parameter is read where the
/// expression needs it, after the set's address).  A body of two or more
/// statements first loads the by-value parameters it reads into registers
/// ([`Selector::entry_loads`]; TIME_$Q_INIT_QUEUE 0xE16C62..0xE16C6A), and
/// its statements may then use only those registers: what the original
/// did with further registers in such a body (fresh registers per
/// statement, IIC_$RECEIVE 0xE70ABA; scheduling of entry loads with other
/// work, 0xE758C4) is not modelled yet (codegen-patterns 12.2).
pub fn select_body(f: &Function) -> Result<Body, Diagnostic> {
    let mut s = Selector {
        out: Vec::new(),
        regs: Regs::new(),
        span: f.span,
        pinned: Vec::new(),
        multi: false,
    };
    if f.body.len() >= 2 {
        if f.result.is_some() {
            // PARITY_$CHK_IO 0xE0B180..0xE0B18A loads its first parameter
            // into D1 and its second into D0: functions do not follow the
            // procedure order, and the rule is not known.
            return Err(unsupported(
                f.body[0].span,
                "a function body of more than one statement is",
            ));
        }
        reject_mergeable_stores(f)?;
        s.entry_loads(f)?;
        s.multi = true;
    }
    for st in &f.body {
        s.span = st.span;
        match &st.kind {
            StmtKind::SetResult(e) => s.set_result(f, e)?,
            StmtKind::Store { addr, value } => s.store(addr, value)?,
        }
        debug_assert!(
            s.regs
                .live_scratch()
                .iter()
                .all(|r| s.pinned.iter().any(|&(_, p)| p == r)),
            "a statement left a register busy"
        );
    }
    Ok(Body {
        insns: s.out,
        saves: s.regs.to_save(),
    })
}

/// Ref 3.15.3, p. 3-57 ("Device"): "By default, the compiler optimizes
/// certain adjacent references by merging them into one large reference"
/// (`a := 0; b := 0` on two `integer16`s becomes one 32-bit store).  Which
/// references it merged has not been catalogued: the `clr.b` of the
/// adjacent bytes 12 and 13 are *not* merged in XPD_$RESET_PTRACE_OPTS
/// (0xE5B15E, 0xE5B164), but they are not consecutive in the image
/// (offsets 12, 0, 13, 4, 8) and the source order is not known.  So two consecutive
/// stores to adjacent objects of the same base that fit together in a
/// longword are rejected rather than guessed at.  TIME_$Q_INIT_QUEUE's
/// stores (0..4, 4..8, 8, 10..12) never fit: 0xE16C6E..0xE16C78.
fn reject_mergeable_stores(f: &Function) -> Res<()> {
    let mut prev: Option<(&Addr, u32)> = None;
    for st in &f.body {
        let StmtKind::Store { addr, value } = &st.kind else {
            prev = None;
            continue;
        };
        let bytes = value.ty().bytes();
        if let Some((pa, pb)) = prev {
            if let (Some((k1, o1)), Some((k2, o2))) = (split_addr(pa), split_addr(addr)) {
                let adjacent = o1 + pb as i32 == o2 || o2 + bytes as i32 == o1;
                if k1 == k2 && adjacent && pb + bytes <= 4 {
                    return Err(unsupported(
                        st.span,
                        "a store adjacent to the previous statement's store (the original \
                         compiler may merge them, ref 3.15.3 p. 3-57) is",
                    ));
                }
            }
        }
        prev = Some((addr, bytes));
    }
    Ok(())
}

/// The base of `addr` (with offset 0) and the offset.
fn split_addr(addr: &Addr) -> Option<(Addr, i32)> {
    match addr {
        Addr::Frame(o) => Some((Addr::Frame(0), *o)),
        Addr::Symbol { name, offset } => Some((
            Addr::Symbol {
                name: name.clone(),
                offset: 0,
            },
            *offset,
        )),
        Addr::Indirect { base, offset } => Some((
            Addr::Indirect {
                base: base.clone(),
                offset: 0,
            },
            *offset,
        )),
    }
}

/// How the body uses a by-value parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParamUse {
    /// Its value is read as data.
    Data,
    /// It is a pointer the body dereferences (`p^`, `p^.f`).
    Base,
}

/// Every use of a by-value parameter in `f`'s body, as (frame offset,
/// use), in body order.
fn param_uses(f: &Function) -> Vec<(i32, ParamUse)> {
    fn is_param(f: &Function, o: i32) -> bool {
        f.value_params.iter().any(|p| p.offset == o)
    }
    fn expr(f: &Function, e: &Expr, out: &mut Vec<(i32, ParamUse)>) {
        match e {
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } if is_param(f, *o) => out.push((*o, ParamUse::Data)),
            Expr::Load { addr, .. } => address(f, addr, out),
            Expr::SetMember { elem, set, .. } => {
                expr(f, elem, out);
                expr(f, set, out);
            }
            Expr::Const { .. } => {}
        }
    }
    fn address(f: &Function, a: &Addr, out: &mut Vec<(i32, ParamUse)>) {
        if let Addr::Indirect { base, .. } = a {
            match &**base {
                Expr::Load {
                    addr: Addr::Frame(o),
                    ..
                } if is_param(f, *o) => out.push((*o, ParamUse::Base)),
                b => expr(f, b, out),
            }
        }
    }
    let mut out = Vec::new();
    for st in &f.body {
        match &st.kind {
            StmtKind::SetResult(e) => expr(f, e, &mut out),
            StmtKind::Store { addr, value } => {
                address(f, addr, &mut out);
                expr(f, value, &mut out);
            }
        }
    }
    out
}

/// Selection state of one routine body.
pub struct Selector {
    out: Vec<Insn>,
    regs: Regs,
    /// The statement being selected, for diagnostics.
    span: Span,
    /// By-value parameters loaded at entry: frame offset and the register
    /// that holds the value for the whole body.
    pinned: Vec<(i32, Reg)>,
    /// The body has more than one statement: statements may use only the
    /// pinned registers (see [`select_body`]).
    multi: bool,
}

impl Selector {
    fn alloc(&mut self, class: RegClass) -> Res<Reg> {
        if self.multi {
            return Err(unsupported(
                self.span,
                "in a body of more than one statement, a statement that needs a register \
                 of its own is",
            ));
        }
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
    /// not released; nor are registers pinned by [`Self::entry_loads`].
    fn free_ea(&mut self, ea: &Ea) {
        match ea {
            Ea::Ind(a) | Ea::Disp(_, a)
                if Reg::A(*a) != DATA_BASE
                    && *a != FP
                    && *a != SP
                    && !self.is_pinned(Reg::A(*a)) =>
            {
                self.free_a(*a)
            }
            Ea::Reg(r) if !self.is_pinned(*r) => self.regs.free(*r),
            _ => {}
        }
    }

    /// The register a by-value parameter at frame offset `o` was loaded
    /// into at entry, if it was.
    fn pinned(&self, o: i32) -> Option<Reg> {
        self.pinned.iter().find(|&&(p, _)| p == o).map(|&(_, r)| r)
    }

    fn is_pinned(&self, r: Reg) -> bool {
        self.pinned.iter().any(|&(_, p)| p == r)
    }

    /// Load the by-value parameters the body reads into registers, in
    /// parameter order, before the first statement.
    ///
    /// TIME_$Q_INIT_QUEUE 0xE16C62..0xE16C6A:
    ///
    /// ```text
    /// move.b  (8,A6),D0      ; is_vt, read once, by the third statement
    /// move.w  (0xa,A6),D1    ; queue_id, read once, by the fourth
    /// movea.l (0xc,A6),A0    ; queue, the base of all four stores
    /// ```
    ///
    /// The same in other bodies of several statements: module-local
    /// 0xE0A328 (DMA_; `dma/init_m68450_channel.c`) at 0xE0A32C, `move.w
    /// (0xc,A6),D0`, used only by the last of five statements; module-local
    /// 0xE758C4 (RING) at 0xE758C8..0xE758D2, two longwords into D0, D1;
    /// MMAP_$SET_WS_MAX 0xE0CA88..0xE0CA90, SIO6509_$INQ_LINE
    /// 0xE7266C..0xE72674 (codegen-patterns 12.2).  Registers: the lowest
    /// free scratch register of the class, data values in D registers,
    /// pointers the body dereferences in A registers.  A pointer also used
    /// as data, more than two parameters of a class (the original then
    /// takes callee-saved registers even without calls: MST_$VA_TO_SEGNO
    /// 0xE0DCD6, ROUTE_$FIND_PORT 0xE15B06) and parameters that must
    /// survive a call (PROC1_$SET_TYPE 0xE152F2) are not supported yet.
    fn entry_loads(&mut self, f: &Function) -> Res<()> {
        let uses = param_uses(f);
        for p in &f.value_params {
            let mine: Vec<ParamUse> = uses
                .iter()
                .filter(|(o, _)| *o == p.offset)
                .map(|&(_, u)| u)
                .collect();
            let Some(&first) = mine.first() else {
                continue;
            };
            if mine.iter().any(|&u| u != first) {
                // ACL_$ADD_PROJ 0xE47EC0 / EC2_$INIT 0xE42C64: a pointer
                // that is also data is kept in a D register and copied to
                // an A register for each dereference.
                return Err(unsupported(
                    f.span,
                    "a parameter used both as a pointer and as a value is",
                ));
            }
            let class = match first {
                ParamUse::Data => RegClass::Data,
                ParamUse::Base => RegClass::Addr,
            };
            let r = self.regs.alloc_scratch(class).ok_or_else(|| {
                unsupported(
                    f.span,
                    "loading more than two parameters of a register class at entry is",
                )
            })?;
            let src = Ea::Disp(p.offset, FP);
            self.out.push(match r {
                Reg::D(_) => Insn::Move {
                    size: size_of(p.ty.width),
                    src,
                    dst: Ea::Reg(r),
                },
                Reg::A(a) => Insn::Movea {
                    size: Size::L,
                    src,
                    dst: a,
                },
            });
            self.pinned.push((p.offset, r));
        }
        Ok(())
    }

    /// `d := e`: one move into the destination's effective address.
    ///
    /// The value, by kind:
    ///
    /// - zero: `clr.<s> <ea>` (TIME_$Q_INIT_QUEUE 0xE16C6E `clr.l (A0)`,
    ///   0xE16C70 `clr.l (4,A0)`; `clr.w` IIC_$RECEIVE 0xE70ACE; `clr.b`
    ///   XPD_$RESET_PTRACE_OPTS 0xE5B15E).  Device registers are cleared
    ///   with `move.b #0` instead (WIN_ module 0xE19112, the controller's
    ///   GO register per `win/wait_for_controller.c`; RING 0xE758E2), which
    ///   fits ref 3.15.3 (p. 3-57): the `device` attribute stops "gratuitous
    ///   read-modify-write references", and `clr` reads its operand on the
    ///   68000.  How those objects were declared is not known; `dpc-sema`
    ///   rejects `device`, `volatile` and `atomic` (ref 3.15), so no
    ///   store reaching here is to such an object.
    /// - boolean true: `st <ea>` (SMD_$ENABLE_TRACKING 0xE6E468 `st
    ///   (0xe0,A5)`, a boolean field per `smd/smd_internal.h`).
    /// - another constant: `move.<s> #n,<ea>` (module-local 0xE0A328 at
    ///   0xE0A334, `move.b #0x10,(7,A0)`; MMAP_$ALLOC_CONTIG 0xE0D906
    ///   `move.l #0x6000e,(A1)`; 0xE758D8 `move.w #0x200,(0xa,A0)`).
    /// - a by-value parameter loaded at entry: from its register
    ///   (0xE16C74 `move.b D0,(8,A0)`, 0xE16C78 `move.w D1,(0xa,A0)`).
    /// - another object in memory: memory to memory (PROC2_$GET_BOOT_FLAGS
    ///   0xE41B66 `move.w (0x1e4,A5),(A0)`; CAL_$SET_DRIFT 0xE68588
    ///   `move.l (A0),abs.l`; TIME_$Q_REMOVE_ELEM 0xE16E88 `move.l
    ///   (-4,A6),(A0)`).  A parameter that was not loaded at entry is not
    ///   a source yet: in a one-statement body its code is not known.
    /// - anything else: computed into a data register first.
    ///
    /// When both the destination and the value need a register of their
    /// own, which one the original loaded first is not known: rejected.
    fn store(&mut self, addr: &Addr, value: &Expr) -> Res<()> {
        let ty = value.ty();
        let size = size_of(ty.width);
        if matches!(addr, Addr::Frame(o) if *o > 0) {
            return Err(unsupported(self.span, "assigning to a parameter is"));
        }
        if self.addr_needs_reg(addr) && self.value_needs_reg(value) {
            return Err(unsupported(
                self.span,
                "a store whose destination and value both need a register is",
            ));
        }
        enum Src {
            Zero,
            True,
            Ea(Ea),
        }
        let src = match value {
            Expr::Const { value: 0, .. } => Src::Zero,
            Expr::Const { value: -1, ty } if *ty == Scalar::BOOLEAN => Src::True,
            Expr::Const { value, ty } => Src::Ea(Ea::Imm(imm(*value, ty.width))),
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } if *o > 0 => match self.pinned(*o) {
                Some(r) => Src::Ea(Ea::Reg(r)),
                None => {
                    return Err(unsupported(
                        self.span,
                        "storing a parameter that was not loaded at entry is",
                    ))
                }
            },
            Expr::Load { addr: a, .. } => Src::Ea(self.ea_of(a)?),
            e => Src::Ea(Ea::Reg(Reg::D(self.eval_d(e)?))),
        };
        let dst = self.ea_of(addr)?;
        self.out.push(match &src {
            Src::Zero => Insn::Clr {
                size,
                dst: dst.clone(),
            },
            Src::True => Insn::Scc {
                cond: Cond::T,
                dst: dst.clone(),
            },
            Src::Ea(ea) => Insn::Move {
                size,
                src: ea.clone(),
                dst: dst.clone(),
            },
        });
        if let Src::Ea(ea) = &src {
            self.free_ea(ea);
        }
        self.free_ea(&dst);
        Ok(())
    }

    /// Whether forming `addr` takes a register that is not already
    /// holding the base.
    fn addr_needs_reg(&self, addr: &Addr) -> bool {
        match addr {
            Addr::Frame(_) | Addr::Symbol { .. } => false,
            Addr::Indirect { base, .. } => !matches!(
                &**base,
                Expr::Load { addr: Addr::Frame(o), .. } if self.pinned(*o).is_some()
            ),
        }
    }

    /// Whether `value` as a store's source takes a register of its own.
    fn value_needs_reg(&self, value: &Expr) -> bool {
        match value {
            Expr::Const { .. } => false,
            Expr::Load { addr, .. } => self.addr_needs_reg(addr),
            Expr::SetMember { .. } => true,
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
            // A value held in a register for the whole body is copied before
            // it is used as a temporary (BAT_$GET_BAT_STEP 0xE3BB68 `move.w
            // D0,D1`); the copy rule has not been settled.
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } if self.pinned(*o).is_some() => Err(unsupported(
                self.span,
                "computing with a parameter loaded at entry is",
            )),
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

    /// Evaluate a pointer into a scratch address register, or give the
    /// register a by-value parameter was loaded into at entry.
    fn eval_a(&mut self, e: &Expr) -> Res<u8> {
        if let Expr::Load {
            addr: Addr::Frame(o),
            ..
        } = e
        {
            if let Some(r) = self.pinned(*o) {
                return match r {
                    Reg::A(a) => Ok(a),
                    // `entry_loads` puts dereferenced pointers in A
                    // registers.
                    Reg::D(_) => unreachable!("pointer parameter pinned in a data register"),
                };
            }
        }
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
            value_params: Vec::new(),
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
            pinned: Vec::new(),
            multi: false,
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
            pinned: Vec::new(),
            multi: false,
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

    // ---- statement sequences and stores (codegen-patterns 12.2, 13) ----

    fn word() -> Scalar {
        Scalar::signed(Width::Word)
    }

    fn param(offset: i32, ty: Scalar) -> Expr {
        load(Addr::Frame(offset), ty)
    }

    /// `(*p).offset` for the pointer parameter at `p`.
    fn through(p: i32, offset: i32) -> Addr {
        Addr::Indirect {
            base: Box::new(param(p, Scalar::POINTER)),
            offset,
        }
    }

    fn store(addr: Addr, value: Expr) -> StmtKind {
        StmtKind::Store { addr, value }
    }

    fn konst(value: i64, ty: Scalar) -> Expr {
        Expr::Const { value, ty }
    }

    fn vparams(f: &mut Function, ps: &[(i32, Scalar)]) {
        f.value_params = ps
            .iter()
            .map(|&(offset, ty)| dpc_ir::ValueParam { offset, ty })
            .collect();
    }

    fn err(f: &Function) -> String {
        select(f).unwrap_err().message
    }

    /// TIME_$Q_INIT_QUEUE 0xE16C5E..0xE16C7E.
    #[test]
    fn q_init_queue_shape() {
        let long = Scalar::signed(Width::Long);
        let mut f = func(
            0,
            None,
            vec![
                store(through(12, 0), konst(0, long)),
                store(through(12, 4), konst(0, long)),
                store(through(12, 8), param(8, Scalar::BOOLEAN)),
                store(through(12, 10), param(10, word())),
            ],
        );
        vparams(
            &mut f,
            &[(8, Scalar::BOOLEAN), (10, word()), (12, Scalar::POINTER)],
        );
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#0",
                "move.b\t(8,%fp),%d0",
                "move.w\t(0xa,%fp),%d1",
                "movea.l\t(0xc,%fp),%a0",
                "clr.l\t(%a0)",
                "clr.l\t(4,%a0)",
                "move.b\t%d0,(8,%a0)",
                "move.w\t%d1,(0xa,%a0)",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    /// Entry loads follow parameter order, not the order of first use,
    /// and a parameter the body does not read is not loaded.
    #[test]
    fn entry_loads_in_parameter_order() {
        let mut f = func(
            0,
            None,
            vec![
                store(through(8, 0), param(14, word())),
                store(through(8, 4), param(12, word())),
            ],
        );
        vparams(
            &mut f,
            &[
                (8, Scalar::POINTER),
                (12, word()),
                (14, word()),
                (16, word()),
            ],
        );
        assert_eq!(
            &text(&f)[1..4],
            [
                "movea.l\t(8,%fp),%a0",
                "move.w\t(0xc,%fp),%d0",
                "move.w\t(0xe,%fp),%d1",
            ]
        );
    }

    /// A one-statement body is not preloaded: the pointer is read where
    /// the store needs it (TIME_$Q_FLUSH_QUEUE 0xE16C84..0xE16C88).
    #[test]
    fn one_statement_store_through_pointer() {
        let mut f = func(
            0,
            None,
            vec![store(through(8, 0), konst(0, Scalar::POINTER))],
        );
        vparams(&mut f, &[(8, Scalar::POINTER)]);
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#0",
                "movea.l\t(8,%fp),%a0",
                "clr.l\t(%a0)",
                "unlk\t%fp",
                "rts"
            ]
        );
    }

    #[test]
    fn constant_store_forms() {
        let cases: Vec<(Expr, &str)> = vec![
            (konst(0, Scalar::BOOLEAN), "clr.b\t(%a0)"),
            (konst(-1, Scalar::BOOLEAN), "st\t(%a0)"),
            // A char 0xFF is not a boolean: move.b with 0..255.
            (
                konst(255, Scalar::unsigned(Width::Byte)),
                "move.b\t#0xff,(%a0)",
            ),
            (
                konst(0x10, Scalar::unsigned(Width::Byte)),
                "move.b\t#0x10,(%a0)",
            ),
            (konst(-1, word()), "move.w\t#-1,(%a0)"),
            (
                konst(40000, Scalar::unsigned(Width::Word)),
                "move.w\t#-0x63c0,(%a0)",
            ),
            (
                konst(0x6000e, Scalar::signed(Width::Long)),
                "move.l\t#0x6000e,(%a0)",
            ),
            (konst(0, word()), "clr.w\t(%a0)"),
        ];
        for (value, want) in cases {
            let f = func(0, None, vec![store(through(8, 0), value)]);
            assert_eq!(text(&f)[2], want);
        }
    }

    #[test]
    fn multi_statement_functions_are_rejected() {
        let f = func(
            0,
            Some(ResultSpec {
                class: ResultClass::Data,
                ty: word(),
            }),
            vec![
                store(through(8, 0), konst(0, word())),
                StmtKind::SetResult(param(12, word())),
            ],
        );
        assert_eq!(
            err(&f),
            "a function body of more than one statement is not supported yet"
        );
    }

    /// Ref 3.15.3 p. 3-57: the original merged some adjacent stores.
    #[test]
    fn adjacent_stores_are_rejected() {
        let f = func(
            0,
            None,
            vec![
                store(through(8, 0), konst(0, word())),
                store(through(8, 2), konst(0, word())),
            ],
        );
        assert!(err(&f).starts_with("a store adjacent to the previous"));
        // Not adjacent (a gap), or too big together: accepted.
        for (o, ty) in [(4, word()), (2, Scalar::signed(Width::Long))] {
            let mut f = func(
                0,
                None,
                vec![
                    store(through(8, 0), konst(0, word())),
                    store(through(8, o), konst(0, ty)),
                ],
            );
            vparams(&mut f, &[(8, Scalar::POINTER)]);
            assert!(select(&f).is_ok(), "{o}: {:?}", select(&f));
        }
    }

    #[test]
    fn too_many_entry_loads_are_rejected() {
        let mut f = func(
            0,
            None,
            vec![
                store(through(8, 0), param(12, word())),
                store(through(8, 4), param(14, word())),
                store(through(8, 8), param(16, word())),
            ],
        );
        vparams(
            &mut f,
            &[
                (8, Scalar::POINTER),
                (12, word()),
                (14, word()),
                (16, word()),
            ],
        );
        assert_eq!(
            err(&f),
            "loading more than two parameters of a register class at entry is not supported yet"
        );
    }

    #[test]
    fn pointer_used_as_data_is_rejected() {
        let mut f = func(
            0,
            None,
            vec![
                store(through(8, 0), konst(0, word())),
                store(
                    Addr::Symbol {
                        name: "X".into(),
                        offset: 0,
                    },
                    param(8, Scalar::POINTER),
                ),
            ],
        );
        vparams(&mut f, &[(8, Scalar::POINTER)]);
        assert_eq!(
            err(&f),
            "a parameter used both as a pointer and as a value is not supported yet"
        );
    }

    /// In a body of several statements, a statement may use only the
    /// registers loaded at entry.
    #[test]
    fn statement_registers_in_multi_statement_bodies_are_rejected() {
        let ext = load(
            Addr::Symbol {
                name: "P".into(),
                offset: 0,
            },
            Scalar::POINTER,
        );
        let f = func(
            0,
            None,
            vec![
                store(
                    Addr::Indirect {
                        base: Box::new(ext),
                        offset: 0,
                    },
                    konst(0, word()),
                ),
                store(through(8, 4), konst(0, word())),
            ],
        );
        assert!(err(&f).starts_with("in a body of more than one statement"));
    }

    #[test]
    fn unsettled_one_statement_stores_are_rejected() {
        // The value is a parameter that was not loaded at entry.
        let mut f = func(0, None, vec![store(through(8, 0), param(12, word()))]);
        vparams(&mut f, &[(8, Scalar::POINTER), (12, word())]);
        assert_eq!(
            err(&f),
            "storing a parameter that was not loaded at entry is not supported yet"
        );
        // Both sides need an address register.
        let f = func(
            0,
            None,
            vec![store(through(8, 0), load(through(12, 0), word()))],
        );
        assert_eq!(
            err(&f),
            "a store whose destination and value both need a register is not supported yet"
        );
        // Assigning to a parameter.
        let f = func(0, None, vec![store(Addr::Frame(8), konst(1, word()))]);
        assert_eq!(err(&f), "assigning to a parameter is not supported yet");
    }

    /// Memory to memory: a local into the object a pointer parameter
    /// points to (TIME_$Q_REMOVE_ELEM 0xE16E80..0xE16E88).
    #[test]
    fn memory_to_memory_store() {
        let f = func(
            4,
            None,
            vec![store(
                through(0x10, 0),
                load(Addr::Frame(-4), Scalar::signed(Width::Long)),
            )],
        );
        assert_eq!(
            &text(&f)[1..3],
            ["movea.l\t(0x10,%fp),%a0", "move.l\t(-4,%fp),(%a0)"]
        );
    }
}
