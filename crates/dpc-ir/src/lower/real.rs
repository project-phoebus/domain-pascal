//! Real arithmetic (ref 3.3, 4.3), best effort (not byte-identical; bead
//! domain-pascal-338r, domain-c's gap 7; no kernel site): every real
//! computation is a call of the Pascal run time's software floating point
//! interpreter, syslib's fpp.asm of SR10.2 ([`Callee::Fpp`]), as the
//! DOMAIN C compiler (Pascal Rev 8.89) compiles one: `jsr fpp_$` and the
//! operation words after it, the operands pushed before, last-used first
//! (../source evidence/lib/syslib/fpp.codegen.md, "The interpreter"; the
//! words of fpp_ops.tsv there).  The interpreter has two accumulators, SA
//! (a single) and DA (a double).
//!
//! An expression is loaded into the accumulator of its precision; a
//! binary operation's right operand is a real variable of that precision
//! (`<op>_a`, its address pushed) or a constant (`<op>_i`, inline);
//! anything else is computed first into a frame temporary (`<s|d>store_a`)
//! and taken from there.  Constants are doubles (`dload_i`), converted to
//! a single where one is wanted (`dtos`), as the DOMAIN C compiler's
//! 8-byte inline constants are (reported by domain-c: `round(n * 0.9)`'s
//! `dmul_i`).  The sequence ends with a store (`<s|d>store_a_ret`), a
//! conversion to an integer (`<s|d>fix32`) or a compare (`<s|d>cmp`),
//! the last two leaving their result in D0.

use super::*;

/// The accumulator a value is in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Prec {
    S,
    D,
}

/// The operation words (fpp_ops.tsv's `op`).
mod op {
    pub const S_LOAD_A: u16 = 0xFF8A;
    pub const S_LOAD_V: u16 = 0xFF6A;
    pub const S_STORE_A: u16 = 0xFFD0;
    pub const S_STORE_RET: u16 = 0xFF3E;
    pub const S_FLT32_V: u16 = 0x005C;
    pub const S_FIX32: u16 = 0x0068;
    pub const S_NEG: u16 = 0x006C;
    pub const S_ABS: u16 = 0x0070;
    pub const S_ANINT: u16 = 0x0154;
    pub const D_LOAD_A: u16 = 0x0074;
    pub const D_LOAD_I: u16 = 0x0078;
    pub const D_STORE_A: u16 = 0x007C;
    pub const D_STORE_RET: u16 = 0x0080;
    pub const D_RSUB_I: u16 = 0x0098;
    pub const D_FLT32_V: u16 = 0x00C4;
    pub const D_FIX32: u16 = 0x00D0;
    pub const D_ANINT: u16 = 0x015C;
    pub const DTOS: u16 = 0x00D4;
    pub const STOD: u16 = 0x00D8;
}

/// `+ - * /` and compare: the `_a` and `_i` words of each precision.
fn arith_words(op: BinOp, p: Prec) -> Option<(u16, u16)> {
    Some(match (op, p) {
        (BinOp::Add, Prec::S) => (0x0004, 0x0008),
        (BinOp::Sub, Prec::S) => (0x0010, 0x0014),
        (BinOp::Mul, Prec::S) => (0x0028, 0x002C),
        (BinOp::RealDiv, Prec::S) => (0x0034, 0x0038),
        (BinOp::Add, Prec::D) => (0x0084, 0x0088),
        (BinOp::Sub, Prec::D) => (0x008C, 0x0090),
        (BinOp::Mul, Prec::D) => (0x009C, 0x00A0),
        (BinOp::RealDiv, Prec::D) => (0x00A4, 0x00A8),
        // `**`: spow_a/spow_i, dpow_a/dpow_i (`exp(x * ln SA)`).
        (BinOp::Pow, Prec::S) => (0x012C, 0x0134),
        (BinOp::Pow, Prec::D) => (0x0148, 0x014C),
        _ => return None,
    })
}

fn cmp_words(p: Prec) -> (u16, u16) {
    match p {
        Prec::S => (0x004C, 0x0050),
        Prec::D => (0x00B4, 0x00B8),
    }
}

/// The precision of a real type, `None` for another type.
pub(super) fn prec_of(p: &Program, ty: TypeId) -> Option<Prec> {
    match p.types.kind(p.types.canon(ty)) {
        TypeKind::Real { size: 4 } => Some(Prec::S),
        TypeKind::Real { .. } => Some(Prec::D),
        _ => None,
    }
}

/// The precision real arithmetic on `a` and `b` is done in: a single when
/// both are singles or one is a single and the other an integer (as
/// `dpc-sema`'s `real_result`), else a double.
fn common(p: &Program, a: TypeId, b: TypeId) -> Prec {
    match (prec_of(p, a), prec_of(p, b)) {
        (Some(Prec::S), Some(Prec::S)) | (Some(Prec::S), None) | (None, Some(Prec::S)) => Prec::S,
        _ => Prec::D,
    }
}

/// An interpreter call being formed: its arguments in use order and the
/// words after the `jsr`.
struct Seq {
    args: Vec<Arg>,
    words: Vec<u8>,
}

impl Seq {
    fn new() -> Seq {
        Seq {
            args: Vec::new(),
            words: Vec::new(),
        }
    }

    fn op(&mut self, w: u16) {
        self.words.extend(w.to_be_bytes());
    }

    fn imm(&mut self, p: Prec, v: f64) {
        match p {
            Prec::S => self.words.extend((v as f32).to_bits().to_be_bytes()),
            Prec::D => self.words.extend(v.to_bits().to_be_bytes()),
        }
    }

    fn addr(&mut self, a: Addr, writable: bool) {
        self.args.push(Arg::Address {
            addr: a,
            slot: 4,
            writable,
        });
    }

    fn convert(&mut self, from: Prec, to: Prec) {
        match (from, to) {
            (Prec::S, Prec::D) => self.op(op::STOD),
            (Prec::D, Prec::S) => self.op(op::DTOS),
            _ => {}
        }
    }

    fn call(self) -> Call {
        Call {
            callee: Callee::Fpp {
                entry: "FPP_$".into(),
                words: self.words,
            },
            args: self.args,
            pad: 0,
            link: None,
        }
    }
}

/// The value of a numeric constant, if `e` is one.
fn number(e: &SExpr) -> Option<f64> {
    e.value.as_ref().and_then(|v| v.number())
}

/// Whether `e` is a real variable (a designator, not volatile) of
/// precision `p`.
fn real_var(p: &Program, e: &SExpr, prec: Prec) -> bool {
    e.value.is_none()
        && e.is_designator()
        && prec_of(p, e.ty) == Some(prec)
        && volatile_access(p, e).is_none()
}

/// Load `e` into the accumulator of precision `want`.
fn load(p: &Program, s: &mut Seq, e: &SExpr, want: Prec) -> Res<()> {
    if let Some(v) = number(e) {
        s.op(op::D_LOAD_I);
        s.imm(Prec::D, v);
        s.convert(Prec::D, want);
        return Ok(());
    }
    let Some(q) = prec_of(p, e.ty) else {
        if !p.types.is_integer(e.ty) {
            return Err(unsupported(e.span, "this operand of real arithmetic is"));
        }
        // An integer: its value as an integer32, converted.
        s.args.push(Arg::Value {
            value: convert(value_of(p, e)?, Scalar::signed(Width::Long)),
            slot: 4,
        });
        s.op(match want {
            Prec::S => op::S_FLT32_V,
            Prec::D => op::D_FLT32_V,
        });
        return Ok(());
    };
    if real_var(p, e, q) {
        s.addr(addr_of(p, e)?, false);
        s.op(match q {
            Prec::S => op::S_LOAD_A,
            Prec::D => op::D_LOAD_A,
        });
        s.convert(q, want);
        return Ok(());
    }
    match &e.kind {
        ExprKind::Unary {
            op: UnOp::Plus,
            operand,
        } => load(p, s, operand, want),
        ExprKind::Unary {
            op: UnOp::Neg,
            operand,
        } => {
            load(p, s, operand, q)?;
            match q {
                Prec::S => s.op(op::S_NEG),
                // No double negation in the table: `0 - DA`.
                Prec::D => {
                    s.op(op::D_RSUB_I);
                    s.imm(Prec::D, 0.0);
                }
            }
            s.convert(q, want);
            Ok(())
        }
        ExprKind::Binary { op, lhs, rhs } => {
            arith(p, s, *op, lhs, rhs, q)?;
            s.convert(q, want);
            Ok(())
        }
        ExprKind::Builtin {
            builtin: Builtin::Abs,
            args,
        } if q == Prec::S => {
            load(p, s, &args[0], q)?;
            s.op(op::S_ABS);
            s.convert(q, want);
            Ok(())
        }
        // No double `abs` in the table: `max(-x, x)`, x through a
        // temporary (`drsub_i 0.0`, then `dmax_a`).
        ExprKind::Builtin {
            builtin: Builtin::Abs,
            args,
        } => {
            let t = temp(p, s, &args[0], q)?;
            load_temp(s, &t, q);
            s.op(op::D_RSUB_I);
            s.imm(Prec::D, 0.0);
            s.addr(t, false);
            s.op(0x0180);
            s.convert(q, want);
            Ok(())
        }
        ExprKind::Builtin {
            builtin: Builtin::Sqr,
            args,
        } => {
            let x = &args[0];
            let (mul_a, _) = arith_words(BinOp::Mul, q).expect("a multiply");
            if real_var(p, x, q) {
                load(p, s, x, q)?;
                s.addr(addr_of(p, x)?, false);
                s.op(mul_a);
            } else {
                let t = temp(p, s, x, q)?;
                load_temp(s, &t, q);
                s.addr(t, false);
                s.op(mul_a);
            }
            s.convert(q, want);
            Ok(())
        }
        // The real functions: the argument loaded, then the function's
        // word (fpp_ops.tsv: ssqrt/dsqrt, sexp/dexp, slog/dlog, ssin/dsin,
        // scos/dcos, satan/datan).
        ExprKind::Builtin { builtin, args }
            if matches!(
                builtin,
                Builtin::Sqrt
                    | Builtin::Sin
                    | Builtin::Cos
                    | Builtin::Exp
                    | Builtin::Ln
                    | Builtin::Arctan
            ) =>
        {
            load(p, s, &args[0], q)?;
            let (ws, wd) = match builtin {
                Builtin::Sqrt => (0x00DC, 0x00E0),
                Builtin::Exp => (0x00E4, 0x00E8),
                Builtin::Ln => (0x00EC, 0x00F0),
                Builtin::Sin => (0x00F4, 0x00F8),
                Builtin::Cos => (0x00FC, 0x0100),
                _ => (0x010C, 0x0110),
            };
            s.op(if q == Prec::S { ws } else { wd });
            s.convert(q, want);
            Ok(())
        }
        // A single function's result (in D0): popped as a value
        // (`sload_v`).  The DOMAIN C compiler's dbl_to_real returns its
        // single in D0 (reported: cc.m68k 0x0ACDC2; domain-c's gap 19).
        ExprKind::Call(_) if q == Prec::S => {
            s.args.push(Arg::Value {
                value: value_of(p, e)?,
                slot: 4,
            });
            s.op(op::S_LOAD_V);
            s.convert(q, want);
            Ok(())
        }
        _ => Err(unsupported(e.span, "this real expression is")),
    }
}

/// `x` computed into a new frame temporary of precision `q`, its address
/// returned (`<s|d>store_a`, the address pushed for the store).
fn temp(p: &Program, s: &mut Seq, x: &SExpr, q: Prec) -> Res<Addr> {
    load(p, s, x, q)?;
    let t = new_result_temp(match q {
        Prec::S => 4,
        Prec::D => 8,
    });
    s.addr(t.clone(), true);
    s.op(match q {
        Prec::S => op::S_STORE_A,
        Prec::D => op::D_STORE_A,
    });
    Ok(t)
}

/// Load the temporary `t` of precision `q`.
fn load_temp(s: &mut Seq, t: &Addr, q: Prec) {
    s.addr(t.clone(), false);
    s.op(match q {
        Prec::S => op::S_LOAD_A,
        Prec::D => op::D_LOAD_A,
    });
}

/// `l op r` in the accumulator of precision `q`.
fn arith(p: &Program, s: &mut Seq, op: BinOp, l: &SExpr, r: &SExpr, q: Prec) -> Res<()> {
    let Some((w_a, w_i)) = arith_words(op, q) else {
        return Err(unsupported(l.span, "this real operator is"));
    };
    if let Some(v) = number(r) {
        load(p, s, l, q)?;
        s.op(w_i);
        s.imm(q, v);
    } else if real_var(p, r, q) {
        load(p, s, l, q)?;
        s.addr(addr_of(p, r)?, false);
        s.op(w_a);
    } else {
        let t = temp(p, s, r, q)?;
        load(p, s, l, q)?;
        s.addr(t, false);
        s.op(w_a);
    }
    Ok(())
}

/// `target := value` for a real target.
pub(super) fn assign(p: &Program, st: &SStmt, target: &SExpr, value: &SExpr) -> Res<Vec<Stmt>> {
    best_effort(
        st.span,
        "real arithmetic (the Pascal run time's floating point interpreter, `jsr FPP_$`; no \
         kernel site) is",
    )?;
    let q = prec_of(p, target.ty).expect("a real target");
    let dst = addr_of(p, target)?;
    // A constant: its bits stored (the DOMAIN C compiler's ra_utils, `move.l
    // #0x3F800000` for a single 1.0, reported).
    if let Some(v) = number(value) {
        let stmts = match q {
            Prec::S => vec![(dst, (v as f32).to_bits() as i64)],
            Prec::D => {
                // The high longword first in memory on a big-endian
                // target, the low one on a little-endian one.
                let b = v.to_bits();
                let (first, second) = if p.big_endian {
                    (b >> 32, b)
                } else {
                    (b, b >> 32)
                };
                vec![
                    (dst.clone(), first as u32 as i64),
                    (dst.offset_by(4), second as u32 as i64),
                ]
            }
        };
        return Ok(stmts
            .into_iter()
            .map(|(addr, v)| Stmt {
                kind: StmtKind::Store {
                    addr,
                    value: Expr::Const {
                        value: imm_value(v as u64, Scalar::unsigned(Width::Long)),
                        ty: Scalar::unsigned(Width::Long),
                    },
                },
                span: st.span,
            })
            .collect());
    }
    let mut s = Seq::new();
    load(p, &mut s, value, q)?;
    s.addr(dst, true);
    s.op(match q {
        Prec::S => op::S_STORE_RET,
        Prec::D => op::D_STORE_RET,
    });
    Ok(vec![Stmt {
        kind: StmtKind::Call(s.call()),
        span: st.span,
    }])
}

/// `trunc(x)` or `round(x)`: an integer32 in D0 (`<s|d>fix32`, after
/// `<s|d>anint` for `round`).
pub(super) fn fix(p: &Program, e: &SExpr, x: &SExpr, round: bool) -> Res<Expr> {
    best_effort(
        e.span,
        "real arithmetic (the Pascal run time's floating point interpreter, `jsr FPP_$`; no \
         kernel site) is",
    )?;
    let q = prec_of(p, x.ty).unwrap_or(Prec::D);
    let mut s = Seq::new();
    load(p, &mut s, x, q)?;
    if round {
        s.op(match q {
            Prec::S => op::S_ANINT,
            Prec::D => op::D_ANINT,
        });
    }
    s.op(match q {
        Prec::S => op::S_FIX32,
        Prec::D => op::D_FIX32,
    });
    Ok(Expr::Call {
        call: s.call(),
        ty: Scalar::signed(Width::Long),
        reg: ResultClass::Data,
    })
}

/// `lhs rel rhs` with a real operand: the compare's sign in D0 against 0.
pub(super) fn compare(p: &Program, e: &SExpr, rel: Rel, lhs: &SExpr, rhs: &SExpr) -> Res<Cond> {
    best_effort(
        e.span,
        "real arithmetic (the Pascal run time's floating point interpreter, `jsr FPP_$`; no \
         kernel site) is",
    )?;
    let q = common(p, lhs.ty, rhs.ty);
    let (w_a, w_i) = cmp_words(q);
    let mut s = Seq::new();
    if let Some(v) = number(rhs) {
        load(p, &mut s, lhs, q)?;
        s.op(w_i);
        s.imm(q, v);
    } else if real_var(p, rhs, q) {
        load(p, &mut s, lhs, q)?;
        s.addr(addr_of(p, rhs)?, false);
        s.op(w_a);
    } else {
        let t = temp(p, &mut s, rhs, q)?;
        load(p, &mut s, lhs, q)?;
        s.addr(t, false);
        s.op(w_a);
    }
    Ok(Cond::Compare {
        op: rel,
        lhs: Expr::Call {
            call: s.call(),
            ty: Scalar::signed(Width::Long),
            reg: ResultClass::Data,
        },
        rhs: Expr::Const {
            value: 0,
            ty: Scalar::signed(Width::Long),
        },
    })
}

/// `f := value` in a function whose result is a single (in D0, its bits:
/// `dpc_sema::conv::result_reg`): a constant's bits, a single variable's
/// longword, or the value computed into a frame temporary first.
pub(super) fn result(p: &Program, st: &SStmt, value: &SExpr) -> Res<Vec<Stmt>> {
    best_effort(
        st.span,
        "a function result of type real (in D0, its bits; no kernel site) is",
    )?;
    let long = Scalar::unsigned(Width::Long);
    let set = |e: Expr| Stmt {
        kind: StmtKind::SetResult(e),
        span: st.span,
    };
    if let Some(v) = number(value) {
        return Ok(vec![set(Expr::Const {
            value: (v as f32).to_bits() as i64,
            ty: long,
        })]);
    }
    if real_var(p, value, Prec::S) {
        return Ok(vec![set(Expr::Load {
            addr: addr_of(p, value)?,
            ty: long,
        })]);
    }
    let mut s = Seq::new();
    let t = temp(p, &mut s, value, Prec::S)?;
    // The temporary's store ends the sequence: a `ret` word after it.
    let last = s.words.len() - 2;
    s.words[last..].copy_from_slice(&op::S_STORE_RET.to_be_bytes());
    Ok(vec![
        Stmt {
            kind: StmtKind::Call(s.call()),
            span: st.span,
        },
        set(Expr::Load { addr: t, ty: long }),
    ])
}

/// A real argument passed by address to a real parameter of precision
/// `q` that is not a variable of that precision (an expression, an
/// integer, the other precision, a constant): computed into a frame
/// temporary, whose address is passed ([`Arg::Call`]; the DOMAIN C
/// compiler widens a single for an `in double` parameter with
/// `FPP_$ESLV` into a temporary, reported: token.pas 0x073716; domain-c's
/// gap 6).
pub(super) fn arg(p: &Program, x: &SExpr, q: Prec) -> Res<Option<Arg>> {
    if real_var(p, x, q) {
        return Ok(None);
    }
    best_effort(
        x.span,
        "real arithmetic (the Pascal run time's floating point interpreter, `jsr FPP_$`; no \
         kernel site) is",
    )?;
    let mut s = Seq::new();
    load(p, &mut s, x, q)?;
    let t = new_result_temp(match q {
        Prec::S => 4,
        Prec::D => 8,
    });
    s.addr(t, true);
    s.op(match q {
        Prec::S => op::S_STORE_RET,
        Prec::D => op::D_STORE_RET,
    });
    Ok(Some(Arg::Call {
        call: Box::new(s.call()),
        slot: 4,
    }))
}
