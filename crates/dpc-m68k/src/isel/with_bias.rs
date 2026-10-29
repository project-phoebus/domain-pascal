//! MC68020: a `with` temporary of an element of another unit's array whose
//! field displacements pass 16 bits (bead domain-pascal-2ys5; ../source
//! gap 360).
//!
//! The SAU9 kernel's MMAP is indexed from page 16#10000 (`extern
//! array[16#10000..16#1FFFF]` of 16-byte entries), so an element's offset
//! from the symbol is -0x100000 and beyond.  The MC68010 form of a `with`
//! over an element folds the whole offset into every field's displacement
//! after `lea (0,An,Dx),Am` (MMAP_$AVAIL 0xE0CC76; `dpc-ir`'s lowering),
//! which the word displacement cannot hold.  The SAU9 image splits the
//! offset the way an overlay section's arrays are reached from 60000
//! bytes in ([`super::Selector::symbol_bias`]; codegen-patterns 14,
//! "Symbols far from an array's first element"): a multiple of 60000 in
//! the symbol, the rest in the fields' displacements.  AST_$PTOV
//! 0x3C408712 `movea.l #MMAP-1020000,A0; lsl.l #4,D1; lea (0,A0,D1.l),A4;
//! tst.b (-28571,A4)` (-1020000 = -0x100000 + 28576; the field at +5);
//! AST_$INVALIDATE_PAGE 0x3C402302 and MMAP_$FREE 0x3C40EA20 the same
//! (reported by ../source, gap 360; not re-read here).
//!
//! This pass rewrites the function before selection: the temporary's
//! `Index` base becomes `Symbol - bias`, which [`super::Selector::index_ea`]
//! forms as `movea.l #SYM-bias,An` (its 60000 rule), and every reference
//! through the temporary has `bias` taken off its displacement.  The
//! bias is the multiple of 60000 nearest the lowest displacement
//! referenced ([`super::OVERLAY_BIAS`]; the sites show one field each,
//! all within a 16-byte entry, so which displacement the original rounds
//! is not visible: Uncertain near a 30000 boundary).  A temporary whose
//! references fit 16 bits, one not formed from a symbol-based element, or
//! one whose references would not fit after the bias is left as it is
//! (and refused later as before).  The MC68010 code is not changed: the
//! SAU2 image has no such array.

use std::borrow::Cow;

use dpc_ir::{Addr, Arg, Call, Cond, Expr, Function, Stmt, StmtKind};

use super::{walk, Node, OVERLAY_BIAS};

/// The `with` temporaries of `f` whose references pass 16 bits, biased as
/// the module doc says.  -> the function to select (`f` itself when none
/// needs it).
pub(super) fn far_with_temps(f: &Function) -> Cow<'_, Function> {
    // The displacement range referenced through each `with` temporary.
    let mut ranges: Vec<(i32, i32, i32)> = Vec::new();
    walk(&f.body, &mut |n| {
        if let Node::Addr(Addr::Indirect { base, offset }) = n {
            if let Some(t) = with_temp(f, base) {
                match ranges.iter_mut().find(|(x, _, _)| *x == t) {
                    Some(r) => {
                        r.1 = r.1.min(*offset);
                        r.2 = r.2.max(*offset);
                    }
                    None => ranges.push((t, *offset, *offset)),
                }
            }
        }
    });
    let fits = |d: i32| i16::try_from(d).is_ok();
    let biases: Vec<(i32, i32)> = ranges
        .iter()
        .filter(|&&(t, lo, hi)| {
            (!fits(lo) || !fits(hi)) && temp_store_symbol(&f.body, t)
        })
        .filter_map(|&(t, lo, hi)| {
            let bias = (lo + OVERLAY_BIAS / 2).div_euclid(OVERLAY_BIAS) * OVERLAY_BIAS;
            (fits(lo - bias) && fits(hi - bias)).then_some((t, bias))
        })
        .collect();
    if biases.is_empty() {
        return Cow::Borrowed(f);
    }
    let mut g = f.clone();
    let bias_of = |t: i32| biases.iter().find(|(x, _)| *x == t).map(|(_, b)| *b);
    stmts_mut(&mut g.body, &mut |st| {
        // The temporary's store: the symbol biased.
        if let StmtKind::Store {
            addr: Addr::Frame(t),
            value: Expr::AddrOf(Addr::Index { base, .. }),
        } = &mut st.kind
        {
            if let (Some(bias), Addr::Symbol { offset, .. }) = (bias_of(*t), &mut **base) {
                *offset += bias;
            }
        }
        // Every reference through it: the bias taken off.
        stmt_addrs_mut(st, &mut |a| {
            if let Addr::Indirect { base, offset } = a {
                if let Some(bias) = with_temp(f, base).and_then(bias_of) {
                    *offset -= bias;
                }
            }
        });
    });
    Cow::Owned(g)
}

/// The `with` temporary `base` loads, if it is one.
fn with_temp(f: &Function, base: &Expr) -> Option<i32> {
    match base {
        Expr::Load {
            addr: Addr::Frame(t),
            ..
        } if f.with_temps.contains(t) => Some(*t),
        _ => None,
    }
}

/// Whether temporary `t` is stored (once) with the address of an element
/// of another unit's array at a constant symbol, index offset 0 (the
/// `with` lowering's form for an element temporary).
fn temp_store_symbol(body: &[Stmt], t: i32) -> bool {
    let mut stores = 0;
    let mut symbol = false;
    dpc_ir::visit_stmts(body, &mut |st| {
        if let StmtKind::Store {
            addr: Addr::Frame(x),
            value,
        } = &st.kind
        {
            if *x == t {
                stores += 1;
                symbol = matches!(value, Expr::AddrOf(Addr::Index { base, offset: 0, .. })
                    if matches!(&**base, Addr::Symbol { .. }));
            }
        }
    });
    stores == 1 && symbol
}

/// Visit every statement of `body`, nested statements included.
fn stmts_mut(body: &mut [Stmt], v: &mut dyn FnMut(&mut Stmt)) {
    for st in body.iter_mut() {
        v(st);
        match &mut st.kind {
            StmtKind::If { then_, else_, .. } => {
                stmts_mut(then_, v);
                stmts_mut(else_, v);
            }
            StmtKind::Repeat { body, .. } | StmtKind::While { body, .. } => stmts_mut(body, v),
            StmtKind::For(l) => stmts_mut(&mut l.body, v),
            StmtKind::Case {
                arms, otherwise, ..
            } => {
                for a in arms.iter_mut() {
                    stmts_mut(&mut a.body, v);
                }
                if let Some(o) = otherwise {
                    stmts_mut(o, v);
                }
            }
            _ => {}
        }
    }
}

/// Visit every address of `st` itself (not of its nested statements).
fn stmt_addrs_mut(st: &mut Stmt, v: &mut dyn FnMut(&mut Addr)) {
    match &mut st.kind {
        StmtKind::SetResult(e) | StmtKind::Discard(e) => expr_mut(e, v),
        StmtKind::Store { addr, value }
        | StmtKind::StoreBits { addr, value, .. }
        | StmtKind::StoreBitElem {
            array: addr,
            index: value,
            ..
        }
        | StmtKind::StoreSetMember {
            set: addr,
            elem: value,
            ..
        }
        | StmtKind::Fill {
            start: addr, value, ..
        } => {
            addr_mut(addr, v);
            expr_mut(value, v);
        }
        StmtKind::StoreSetConst { set, .. } => addr_mut(set, v),
        StmtKind::Copy { dst, src, .. } => {
            addr_mut(dst, v);
            addr_mut(src, v);
        }
        StmtKind::Call(c) => call_mut(c, v),
        StmtKind::If { cond, .. } | StmtKind::While { cond, .. } => cond_mut(cond, v),
        StmtKind::Repeat { until, .. } => {
            if let Some(u) = until {
                cond_mut(u, v);
            }
        }
        StmtKind::For(l) => {
            if let Some(h) = &mut l.home {
                addr_mut(h, v);
            }
            expr_mut(&mut l.from, v);
            expr_mut(&mut l.to, v);
        }
        StmtKind::Case { selector, .. } => expr_mut(selector, v),
        StmtKind::Return
        | StmtKind::Disable
        | StmtKind::Enable
        | StmtKind::Exit
        | StmtKind::Next
        | StmtKind::Label(_)
        | StmtKind::Goto(_) => {}
    }
}

fn expr_mut(e: &mut Expr, v: &mut dyn FnMut(&mut Addr)) {
    match e {
        Expr::Const { .. } => {}
        Expr::Load { addr, .. } | Expr::Bits { addr, .. } | Expr::AddrOf(addr) => addr_mut(addr, v),
        Expr::SetMember { elem, set, .. } => {
            expr_mut(elem, v);
            expr_mut(set, v);
        }
        Expr::SetMemberAt { elem, set, .. }
        | Expr::BitElem {
            index: elem,
            array: set,
            ..
        } => {
            expr_mut(elem, v);
            addr_mut(set, v);
        }
        Expr::Binary { lhs, rhs, .. } | Expr::SetOp { lhs, rhs, .. } => {
            expr_mut(lhs, v);
            expr_mut(rhs, v);
        }
        Expr::Unary { operand: x, .. }
        | Expr::Convert { value: x, .. }
        | Expr::SetSr(x)
        | Expr::SetOf { elem: x, .. } => expr_mut(x, v),
        Expr::Call { call, .. } => call_mut(call, v),
        Expr::Test(c) => cond_mut(c, v),
    }
}

fn addr_mut(a: &mut Addr, v: &mut dyn FnMut(&mut Addr)) {
    v(a);
    match a {
        Addr::Frame(_)
        | Addr::Enclosing(_)
        | Addr::Static(_)
        | Addr::OtherStatic { .. }
        | Addr::Symbol { .. }
        | Addr::Const(_) => {}
        Addr::Volatile { addr, .. } => addr_mut(addr, v),
        Addr::Indirect { base, .. } => expr_mut(base, v),
        Addr::Index { base, index, .. } => {
            addr_mut(base, v);
            expr_mut(index, v);
        }
    }
}

fn call_mut(c: &mut Call, v: &mut dyn FnMut(&mut Addr)) {
    for a in c.args.iter_mut() {
        match a {
            Arg::Value { value, .. } | Arg::Temp { value, .. } => expr_mut(value, v),
            Arg::Address { addr, .. } => addr_mut(addr, v),
            Arg::Call { call, .. } => call_mut(call, v),
        }
    }
    if let Some(x) = c.pointer_mut() {
        expr_mut(x, v);
    }
}

fn cond_mut(c: &mut Cond, v: &mut dyn FnMut(&mut Addr)) {
    match c {
        Cond::Compare { lhs, rhs, .. } => {
            expr_mut(lhs, v);
            expr_mut(rhs, v);
        }
        Cond::Test(e) | Cond::Odd(e) => expr_mut(e, v),
        Cond::Not(x) => cond_mut(x, v),
        Cond::AndThen(a, b) | Cond::OrElse(a, b) | Cond::And(a, b) | Cond::Or(a, b) => {
            cond_mut(a, v);
            cond_mut(b, v);
        }
        Cond::Block { lhs, rhs, .. } => {
            addr_mut(lhs, v);
            addr_mut(rhs, v);
        }
        Cond::SetEmpty { set, .. } => addr_mut(set, v),
    }
}
