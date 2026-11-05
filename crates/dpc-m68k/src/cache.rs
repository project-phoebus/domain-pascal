//! The register-contents cache, store entries (register model F1,
//! `docs/register-model.md`; epic domain-pascal-suii).
//!
//! After an assignment stores a register into a variable, the image reads
//! that register for the variable at 88% of later uses of every kind
//! (SAU2 305 / 42: pushes 83 / 4, operands 43 / 0, stores elsewhere 67 /
//! 3, compares 44 / 11), until something ends it.  It is opportunistic:
//! the allocator does not keep the register for it, and a value that takes
//! the register in between ends it (PROC1_$CREATE_P 0xE151A0: D0 stored
//! to `(-4,A6)`, then a `case` selector computed in D0, then `move.l
//! (-4,A6),-(SP)` twice).  So this runs on the allocated body, physical
//! registers, in the instructions' order:
//!
//! - an entry "register R holds the `size` bytes at location L" is made by
//!   `move.<size> R,M` (not into a volatile object).  L is M, or, for M
//!   through an address register that a `movea.l` loaded from a variable
//!   V and that has not changed since, "V's pointer + d", so that the
//!   entry outlives the base register (`p^.f := x; ...; y := p^.f` with
//!   `p` loaded again);
//! - it ends at a write of R, a write of a register M's address uses
//!   (when L is not named by V), a memory write that may overlap L or V
//!   (below), a label some branch to which comes from before the entry or
//!   from after the label (R7's rule; F4), the instruction after an
//!   unconditional transfer, a call (a stored local is pushed from its
//!   slot after one 38 times, from a callee-saved register 10: 12.5), and
//!   `Words`/`Encoded` instructions;
//! - while it lives, a read of L at its size is a read of R, where the
//!   instruction takes a register there (`tst`, `cmpi` and the logical
//!   operations only a data register; a push from an address register is
//!   `pea (An)`, codegen-patterns 1.5), but for a `tst` right after the
//!   store, which stays on memory (codegen-patterns 7.1);
//! - a load into the register that holds the value already is dropped,
//!   and so is a register load made dead by the rewrite (the base of the
//!   operand: `movea.l (-4,A6),A0` before `move.l (A0),-(SP)`).
//!
//! A push of a local stored by an earlier statement is also formed at
//! selection (`Selector::forwarded`): there the original keeps the
//! register (`docs/register-model.md`, "Step 3, F1").
//!
//! Overlap: a frame write ends the frame locations it overlaps (all of
//! them at a run-time index); a write of the unit's data (`(d,A5)`) the
//! unit locations it overlaps and every location through a pointer; a
//! write through a pointer the locations through the same pointer
//! variable at overlapping offsets, through any other pointer, the unit
//! and absolute ones, and the frame ones once a frame address is taken.

use std::collections::HashMap;

use crate::insn::{AluOp, Cond, Ea, Insn, Reg, Size};
use crate::regs::{access, liveness, RegKey};

const FP: u8 = 6;
const A5: u8 = 5;
const SP: u8 = 7;

/// What a plain memory operand is rooted at.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Root {
    Frame,
    Unit,
    Abs,
    Ptr(u8),
}

/// A plain memory operand at an exact offset.
fn root(ea: &Ea) -> Option<(Root, i32)> {
    match ea {
        Ea::Disp(d, FP) => Some((Root::Frame, *d)),
        Ea::Ind(FP) => Some((Root::Frame, 0)),
        Ea::Disp(d, A5) => Some((Root::Unit, *d)),
        Ea::Ind(A5) => Some((Root::Unit, 0)),
        Ea::Disp(d, a) if *a != SP => Some((Root::Ptr(*a), *d)),
        Ea::Ind(a) if *a != SP => Some((Root::Ptr(*a), 0)),
        Ea::AbsL(_, d) => Some((Root::Abs, *d)),
        Ea::AbsW(d) => Some((Root::Abs, *d)),
        _ => None,
    }
}

/// A location: a plain operand that does not go through a pointer
/// register, or a field through the pointer a variable holds.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Loc {
    Plain(Ea),
    Field(Ea, i32),
}

fn bytes(s: Size) -> i32 {
    match s {
        Size::B => 1,
        Size::W => 2,
        Size::L => 4,
    }
}

/// The memory operand an instruction writes and its size, if any.
fn mem_write(i: &Insn) -> Option<(&Ea, Size)> {
    let (dst, size) = match i {
        Insn::Move { dst, size, .. }
        | Insn::Clr { dst, size }
        | Insn::Unary { dst, size, .. }
        | Insn::Addq { dst, size, .. }
        | Insn::Subq { dst, size, .. }
        | Insn::AluImm { dst, size, .. }
        | Insn::AluToMem { dst, size, .. } => (dst, *size),
        Insn::Scc { dst, .. } => (dst, Size::B),
        Insn::BitImm { dst, .. } | Insn::Bit { dst, .. } => (dst, Size::L),
        _ => return None,
    };
    (!matches!(dst, Ea::Reg(_))).then_some((dst, size))
}

/// The registers an operand's address uses (`8 + n` for Dn).
fn addr_regs(ea: &Ea) -> Vec<u8> {
    match ea {
        Ea::Ind(a) | Ea::Disp(_, a) | Ea::PostInc(a) | Ea::PreDec(a) => vec![*a],
        Ea::Idx(_, a, x, _) | Ea::Idx2(_, a, x, _, _) => vec![*a, 8 + *x],
        _ => Vec::new(),
    }
}

fn reg_code(r: Reg) -> u8 {
    match r {
        Reg::A(a) => a,
        Reg::D(d) => 8 + d,
    }
}

/// Whether a write of `wb` bytes at `w` may change the `eb` bytes at `e`.
fn may_overlap(w: &Loc, wb: i32, e: &Loc, eb: i32, frame_escaped: bool) -> bool {
    let over = |wo: i32, eo: i32| wo < eo + eb && eo < wo + wb;
    match (w, e) {
        (Loc::Field(wv, wo), Loc::Field(ev, eo)) => wv != ev || over(*wo, *eo),
        (Loc::Field(..), Loc::Plain(ea)) => match root(ea) {
            Some((Root::Frame, _)) => frame_escaped,
            _ => true,
        },
        (Loc::Plain(wa), Loc::Field(..)) => match root(wa) {
            Some((Root::Frame, _)) => false,
            Some(_) => true,
            None => match wa {
                Ea::PreDec(SP) => false,
                Ea::Idx(_, FP, ..) | Ea::Idx2(_, FP, ..) => false,
                _ => true,
            },
        },
        (Loc::Plain(wa), Loc::Plain(ea)) => {
            let Some((er, eo)) = root(ea) else {
                return true;
            };
            match root(wa) {
                Some((Root::Frame, wo)) => er == Root::Frame && over(wo, eo),
                Some((Root::Unit, wo)) => match er {
                    Root::Unit => over(wo, eo),
                    Root::Ptr(_) => true,
                    _ => false,
                },
                Some((Root::Abs, wo)) => match er {
                    Root::Abs => match (wa, ea) {
                        // Distinct symbols are distinct objects.
                        (Ea::AbsL(a, _), Ea::AbsL(b, _)) => a == b && over(wo, eo),
                        _ => over(wo, eo),
                    },
                    Root::Ptr(_) => true,
                    _ => false,
                },
                Some((Root::Ptr(a), wo)) => match er {
                    Root::Ptr(b) if a == b => over(wo, eo),
                    Root::Frame => frame_escaped,
                    _ => true,
                },
                None => match wa {
                    Ea::PreDec(SP) => false,
                    Ea::Idx(_, FP, ..) | Ea::Idx2(_, FP, ..) => er == Root::Frame,
                    Ea::Idx(_, A5, ..) | Ea::Idx2(_, A5, ..) => {
                        matches!(er, Root::Unit | Root::Ptr(_))
                    }
                    _ => er != Root::Frame || frame_escaped,
                },
            }
        }
    }
}

struct Entry {
    loc: Loc,
    size: Size,
    reg: Reg,
    at: usize,
}

/// The cache's state over the instructions walked so far.
struct State {
    entries: Vec<Entry>,
    /// The variable each address register was last loaded from with
    /// `movea.l`, while neither changed.
    base: HashMap<u8, Ea>,
}

impl State {
    fn loc(&self, ea: &Ea) -> Option<Loc> {
        let (r, d) = root(ea)?;
        Some(match r {
            Root::Ptr(a) => match self.base.get(&a) {
                Some(v) => Loc::Field(v.clone(), d),
                None => Loc::Plain(ea.clone()),
            },
            _ => Loc::Plain(ea.clone()),
        })
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.base.clear();
    }
}

/// Whether instruction `k` comes right after the store of a live entry,
/// only loads of address registers between.
fn right_after_store(out: &[Insn], st: &State, k: usize) -> bool {
    st.entries.iter().any(|e| {
        e.at < k
            && out[e.at + 1..k]
                .iter()
                .all(|i| matches!(i, Insn::Movea { src, .. } if root(src).is_some()))
    })
}

/// Rewrite the reads `out` can take from registers stores left holding
/// them, and return the indices of the loads that made dead (to drop).
/// `skip`: instructions already dropped; `volatile`: instructions that
/// reference volatile objects (never rewritten, never entries); `exit`:
/// the registers live at the end.
pub fn store_cache(
    out: &mut [Insn],
    exit: &[Reg],
    volatile: &[usize],
    skip: &[usize],
) -> Vec<usize> {
    let frame_escaped = out.iter().any(|i| {
        matches!(
            i,
            Insn::Pea(Ea::Disp(_, FP) | Ea::Ind(FP) | Ea::Idx(_, FP, ..))
                | Insn::Lea {
                    src: Ea::Disp(_, FP) | Ea::Ind(FP) | Ea::Idx(_, FP, ..),
                    ..
                }
        )
    });
    // The branches into each label, by index.
    let mut sources: HashMap<u32, Vec<usize>> = HashMap::new();
    for (k, i) in out.iter().enumerate() {
        match i {
            Insn::Branch { label, .. } | Insn::Dbf { label, .. } | Insn::DbfFwd { label, .. } => {
                sources.entry(*label).or_default().push(k)
            }
            Insn::TableJump { targets, .. } => {
                for t in targets {
                    sources.entry(*t).or_default().push(k);
                }
            }
            Insn::TableEntry { target, .. } => sources.entry(*target).or_default().push(k),
            _ => {}
        }
    }
    let before = liveness(out, exit, &[]);
    let mut rewritten = false;
    let mut st = State {
        entries: Vec::new(),
        base: HashMap::new(),
    };
    let mut prev_transfer = false;
    let mut self_loads: Vec<usize> = Vec::new();
    for k in 0..out.len() {
        if skip.contains(&k) {
            continue;
        }
        if prev_transfer {
            st.clear();
        }
        prev_transfer = matches!(
            out[k],
            Insn::Branch { cond: Cond::T, .. } | Insn::Jmp(_) | Insn::Rts | Insn::TableJump { .. }
        );
        if let Insn::Label(l) = out[k] {
            if let Some(src) = sources.get(&l) {
                st.entries.retain(|e| !src.iter().any(|&s| s < e.at || s > k));
                // Which register holds which pointer is not tracked across
                // joins.
                st.base.clear();
            }
            continue;
        }
        if matches!(out[k], Insn::Words(_) | Insn::Encoded { .. }) {
            st.clear();
            continue;
        }
        if matches!(out[k], Insn::Jsr(_) | Insn::Bsr { .. }) {
            // A call ends every entry: a stored local is pushed from its
            // slot after a call 38 times, from a callee-saved register 10
            // (codegen-patterns 12.5, "Stored locals pushed from the
            // register").
            st.clear();
            continue;
        }
        let vol = volatile.contains(&k);
        // Reads.
        if !vol && !st.entries.is_empty() {
            let find = |m: &Ea, s: Size, data: bool| -> Option<Reg> {
                let l = st.loc(m)?;
                st.entries
                    .iter()
                    .rev()
                    .find(|e| e.loc == l && e.size == s)
                    .map(|e| e.reg)
                    .filter(|r| !data || matches!(r, Reg::D(_)))
                    .filter(|r| !(s == Size::B && matches!(r, Reg::A(_))))
            };
            let new = match &out[k] {
                // A push from an address register is `pea (An)`, never
                // `move.l An,-(SP)` (codegen-patterns 1.5, "A longword
                // held in an address register": 99 by-value pointer
                // pushes, none by `move.l`); a word or byte one is not
                // taken from an address register.
                Insn::Move {
                    size,
                    src,
                    dst: Ea::PreDec(SP),
                } => find(src, *size, false).and_then(|r| match (r, size) {
                    (Reg::A(a), Size::L) => Some(Insn::Pea(Ea::Ind(a))),
                    (Reg::A(_), _) => None,
                    (r, _) => Some(Insn::Move {
                        size: *size,
                        src: Ea::Reg(r),
                        dst: Ea::PreDec(SP),
                    }),
                }),
                Insn::Move { size, src, dst } => find(src, *size, false).map(|r| Insn::Move {
                    size: *size,
                    src: Ea::Reg(r),
                    dst: dst.clone(),
                }),
                Insn::Movea { size, src, dst } => find(src, *size, false).map(|r| Insn::Movea {
                    size: *size,
                    src: Ea::Reg(r),
                    dst: *dst,
                }),
                // A `tst` right after the store (but for loads of the
                // pointer it goes through) stays on memory: 11 test the
                // memory against 9 the register (codegen-patterns 7.1, "A
                // test right after a store": 0xE0280A `move.w
                // D0,(1042,A0); tst.w (1042,A0)`; SAU9 CT_$CONTROL
                // 0x3C478198 `move.l D0,(A0); movea.l (20,A6),A0; tst.l
                // (A0)`).
                Insn::Tst { size, dst } if !right_after_store(out, &st, k) => {
                    find(dst, *size, true).map(|r| Insn::Tst {
                        size: *size,
                        dst: Ea::Reg(r),
                    })
                }
                Insn::Cmpi { size, imm, dst } => find(dst, *size, true).map(|r| Insn::Cmpi {
                    size: *size,
                    imm: *imm,
                    dst: Ea::Reg(r),
                }),
                Insn::Cmp { size, src, dst } => find(src, *size, false).map(|r| Insn::Cmp {
                    size: *size,
                    src: Ea::Reg(r),
                    dst: *dst,
                }),
                Insn::Cmpa { src, dst } => find(src, Size::L, false).map(|r| Insn::Cmpa {
                    src: Ea::Reg(r),
                    dst: *dst,
                }),
                Insn::Alu { op, size, src, dst } => {
                    let data = !matches!(op, AluOp::Add | AluOp::Sub);
                    find(src, *size, data).map(|r| Insn::Alu {
                        op: *op,
                        size: *size,
                        src: Ea::Reg(r),
                        dst: *dst,
                    })
                }
                _ => None,
            };
            if let Some(n) = new {
                let noop = match &n {
                    Insn::Move {
                        src: Ea::Reg(a),
                        dst: Ea::Reg(b),
                        ..
                    } => a == b,
                    Insn::Movea {
                        src: Ea::Reg(Reg::A(a)),
                        dst,
                        ..
                    } => a == dst,
                    _ => false,
                };
                if !noop {
                    out[k] = n;
                    rewritten = true;
                } else if !matches!(
                    out.get(k + 1),
                    Some(Insn::Branch { .. } | Insn::Scc { .. } | Insn::Dbf { .. })
                ) {
                    // The register holds the value already: the load is
                    // dropped (not where the next instruction reads its
                    // flags).
                    self_loads.push(k);
                    rewritten = true;
                }
            }
        }
        // The location a store writes, before its registers change.
        let written = mem_write(&out[k]).and_then(|(w, ws)| Some((st.loc(w)?, ws, w.clone())));
        let unknown_write = mem_write(&out[k]).is_some() && written.is_none();
        // Writes of registers.
        let acc = access(&out[k]);
        if !acc.defs.is_empty() {
            let defs: Vec<u8> = acc.defs.iter().map(|&r| reg_code(r)).collect();
            st.entries.retain(|e| {
                !defs.contains(&reg_code(e.reg))
                    && match &e.loc {
                        Loc::Plain(ea) => !addr_regs(ea).iter().any(|r| defs.contains(r)),
                        Loc::Field(..) => true,
                    }
            });
            for d in &defs {
                st.base.remove(d);
            }
        }
        // Writes of memory.
        if let Some((w, ws, _)) = &written {
            let wb = bytes(*ws);
            st.entries
                .retain(|e| !may_overlap(w, wb, &e.loc, bytes(e.size), frame_escaped));
            // The pointer variables the write may change.
            let gone: Vec<Ea> = st
                .base
                .values()
                .filter(|v| may_overlap(w, wb, &Loc::Plain((*v).clone()), 4, frame_escaped))
                .cloned()
                .collect();
            st.entries
                .retain(|e| !matches!(&e.loc, Loc::Field(v, _) if gone.contains(v)));
            st.base.retain(|_, v| !gone.contains(v));
        } else if unknown_write {
            if let Some((w, _)) = mem_write(&out[k]) {
                let w = Loc::Plain(w.clone());
                st.entries
                    .retain(|e| !may_overlap(&w, 4, &e.loc, bytes(e.size), frame_escaped));
                let gone: Vec<Ea> = st
                    .base
                    .values()
                    .filter(|v| may_overlap(&w, 4, &Loc::Plain((*v).clone()), 4, frame_escaped))
                    .cloned()
                    .collect();
                st.entries
                    .retain(|e| !matches!(&e.loc, Loc::Field(v, _) if gone.contains(v)));
                st.base.retain(|_, v| !gone.contains(v));
            }
        }
        // A pointer register loaded from a variable.
        if let Insn::Movea {
            size: Size::L,
            src,
            dst,
        } = &out[k]
        {
            if !vol {
                if let Some((r, _)) = root(src) {
                    if !matches!(r, Root::Ptr(a) if a == *dst) {
                        if let Some(Loc::Plain(v)) = st.loc(src) {
                            st.base.insert(*dst, v);
                        }
                    }
                }
            }
        }
        // A new entry.
        if !vol {
            if let (
                Insn::Move {
                    size,
                    src: Ea::Reg(r),
                    ..
                },
                Some((l, _, dst)),
            ) = (&out[k], &written)
            {
                if !addr_regs(dst).contains(&reg_code(*r)) {
                    st.entries.push(Entry {
                        loc: l.clone(),
                        size: *size,
                        reg: *r,
                        at: k,
                    });
                }
            }
        }
    }
    if !rewritten {
        return Vec::new();
    }
    // The loads the rewrite left dead.
    // Liveness without the dropped self-loads (they define nothing new).
    let kept: Vec<Insn> = out
        .iter()
        .enumerate()
        .map(|(k, i)| {
            if self_loads.contains(&k) {
                Insn::Label(u32::MAX - 1)
            } else {
                i.clone()
            }
        })
        .collect();
    let after = liveness(&kept, exit, &[]);
    let mut dead = self_loads;
    for (k, i) in out.iter().enumerate() {
        if skip.contains(&k) || volatile.contains(&k) || dead.contains(&k) {
            continue;
        }
        let r = match i {
            Insn::Movea { src, dst, .. } if root(src).is_some() => Reg::A(*dst),
            Insn::Move {
                src,
                dst: Ea::Reg(r),
                ..
            } if root(src).is_some() => *r,
            _ => continue,
        };
        let key = RegKey(r);
        if before[k].contains(&key) && !after[k].contains(&key) {
            dead.push(k);
        }
    }
    dead
}
