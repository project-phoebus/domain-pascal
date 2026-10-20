//! Spilling (bead domain-pascal-qea9).
//!
//! **Best effort (not byte-identical): the original has no conventional
//! spills** (bead domain-pascal-ei42 asks what it does instead where dpc
//! runs out of registers; codegen-patterns 12.4, "Spilling").  When a
//! body does not colour even without its constraints of form and without
//! its holds through calls (`isel.rs`, `select_body_with`), values are
//! taken out of registers until it does:
//!
//! - the value is chosen among the one that found no register and the
//!   values of its class live where it is: the one live longest per
//!   instruction that names it;
//! - each instruction that reads it is preceded by a reload into a new
//!   value of the same class, which the instruction reads instead, and
//!   each one that writes it is followed by a store (and, when it writes
//!   only part of the register, a `move.w` or `move.b` into a data
//!   register, also preceded by a reload where an earlier write may have
//!   stored the rest; an `s<cc>`, which reads the flags, has its
//!   destination spilled only where no earlier write reaches it).  Where it goes ([`Form`]): an address loaded once from
//!   a by-address parameter's slot is loaded from that slot again (no
//!   store); another address value is parked in a new data value (`move.l
//!   At,Dw` / `movea.l Dw,At`) where data registers look free over its
//!   range (the image keeps address values in data registers,
//!   codegen-patterns 12.2, "Pointers in data registers"); any other value
//!   goes to a 4-byte frame slot of its own below the locals (`move.l
//!   Rt,(d,A6)`, `move.l (d,A6),Dt` / `movea.l (d,A6),At`);
//! - the condition codes are kept: a reload into a data register, which
//!   sets them, is not placed where they are live, and a store, which sets
//!   them too, only where they are dead or after a `move`, `moveq` or
//!   `clr` into the data register, whose flags a `tst` of that register
//!   after the store sets again; a value that would need either
//!   elsewhere, a `dbf` counter and a `case` dispatch's register are not
//!   spilled;
//! - the body is coloured again, until it colours (each spill removes one
//!   value of the body; the reloads' and stores' values are never
//!   spilled; a parked value may go to a slot later).
//!
//! The body's instructions keep their indices: the result gives them
//! (renamed) and, apart, the reloads before and the stores after each,
//! which the selector splices in after its checks (`isel.rs`).

use std::collections::{BTreeSet, HashMap};

use crate::insn::{Ea, Insn, Reg, RegSet, Size};
use crate::regs::{access, allocate_blame, is_virtual, liveness, rename, RegKey};

/// Where an instruction of the rewritten body comes from: the body's own
/// instruction `k`, or a reload or store of a spilled value placed next to
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tag {
    Orig(usize),
    Spill(usize),
}

/// A body coloured with spills.
#[derive(Debug)]
pub struct Spilled {
    /// The body's own instructions, renamed (as many as the input).
    pub insns: Vec<Insn>,
    /// The reloads before each instruction (also before a partial write
    /// that may follow an earlier write).
    pub before: Vec<Vec<Insn>>,
    /// The stores (and `tst`s) after each instruction.
    pub after: Vec<Vec<Insn>>,
    /// The callee-saved registers the colouring uses.
    pub saves: RegSet,
    /// The frame bytes the spill slots take below `base`.
    pub bytes: u32,
}

/// Colour `body` (over virtual registers, as `allocate_apart` takes it,
/// with no constraints of form and no holds) with values spilled to frame
/// slots at `(-(base + 4 (k + 1)),A6)`.  `Err` when a value finds no
/// register and nothing it interferes with can be spilled.
pub fn allocate_spilling(
    body: &[Insn],
    exit: &[Reg],
    coalesce: &[(Reg, Reg)],
    base: u32,
    remat: &[i32],
) -> Result<Spilled, String> {
    let mut code: Vec<(Tag, Insn)> = body
        .iter()
        .cloned()
        .enumerate()
        .map(|(k, i)| (Tag::Orig(k), i))
        .collect();
    let mut next = Fresh::new(body);
    let mut temps: BTreeSet<RegKey> = BTreeSet::new();
    let mut slots: u32 = 0;
    loop {
        let mut insns: Vec<Insn> = code.iter().map(|(_, i)| i.clone()).collect();
        let failed = match allocate_blame(&mut insns, exit, &[], &[], None, &[], coalesce, &[]) {
            Ok(saves) => {
                let n = body.len();
                let mut own: Vec<Option<Insn>> = vec![None; n];
                let mut before: Vec<Vec<Insn>> = vec![Vec::new(); n];
                let mut after: Vec<Vec<Insn>> = vec![Vec::new(); n];
                for ((tag, _), insn) in code.iter().zip(insns) {
                    match *tag {
                        Tag::Orig(k) => own[k] = Some(insn),
                        Tag::Spill(k) if own[k].is_none() => before[k].push(insn),
                        Tag::Spill(k) => after[k].push(insn),
                    }
                }
                return Ok(Spilled {
                    insns: own.into_iter().map(|i| i.expect("every instruction kept")).collect(),
                    before,
                    after,
                    saves,
                    bytes: 4 * slots,
                });
            }
            Err((_, Some(v))) => v,
            Err((what, None)) => return Err(what),
        };
        let plain: Vec<Insn> = code.iter().map(|(_, i)| i.clone()).collect();
        let Some((v, form)) = choose(&plain, exit, failed, &temps, remat) else {
            return Err("more values live at once than there are registers".into());
        };
        let off = base
            .checked_add(4 * (slots + 1))
            .filter(|&o| o <= 0x7fff)
            .ok_or("a spill slot beyond `link.w`'s reach")?;
        let off = -(off as i32);
        code = spill(&code, v, &form, off, &mut next, &mut temps)
            .ok_or("more values live at once than there are registers (no new virtual register)")?;
        if form == Form::Slot {
            slots += 1;
        }
    }
}

/// New virtual register numbers, after the highest the body uses.
struct Fresh {
    next_d: u16,
    next_a: u16,
}

impl Fresh {
    fn new(body: &[Insn]) -> Fresh {
        let mut f = Fresh {
            next_d: crate::regs::VBASE as u16,
            next_a: crate::regs::VBASE as u16,
        };
        for i in body {
            let a = access(i);
            for r in a.uses.iter().chain(a.defs.iter()) {
                match *r {
                    Reg::D(n) if is_virtual(*r) => f.next_d = f.next_d.max(n as u16 + 1),
                    Reg::A(n) if is_virtual(*r) => f.next_a = f.next_a.max(n as u16 + 1),
                    _ => {}
                }
            }
        }
        f
    }

    fn take(&mut self, like: Reg) -> Option<Reg> {
        let n = match like {
            Reg::D(_) => &mut self.next_d,
            Reg::A(_) => &mut self.next_a,
        };
        let r = u8::try_from(*n).ok()?;
        *n += 1;
        Some(match like {
            Reg::D(_) => Reg::D(r),
            Reg::A(_) => Reg::A(r),
        })
    }
}

/// Whether the condition codes are live into and out of each instruction
/// (`crate::sched`'s model, which takes physical registers: each virtual
/// one is read as the first of its class).
fn cc_live(insns: &[Insn]) -> (Vec<bool>, Vec<bool>) {
    let phys: Vec<Insn> = insns
        .iter()
        .map(|i| {
            let mut i = i.clone();
            rename(&mut i, &|r| match r {
                Reg::D(_) if is_virtual(r) => Reg::D(0),
                Reg::A(_) if is_virtual(r) => Reg::A(0),
                _ => r,
            });
            i
        })
        .collect();
    let out = crate::sched::cc_live_out(&phys);
    let inn = phys
        .iter()
        .zip(&out)
        .map(|(i, &o)| {
            let (reads, sets) = crate::sched::cc_effect(i);
            reads || (o && !sets)
        })
        .collect();
    (inn, out)
}

fn same_class(a: Reg, b: Reg) -> bool {
    std::mem::discriminant(&a) == std::mem::discriminant(&b)
}

/// Per instruction, whether some path from the entry to it passes a
/// write of `v`: where none does, a partial write needs no reload first
/// (the rest of the register is undefined either way).
fn written_before(insns: &[Insn], v: Reg) -> Vec<bool> {
    let succ = crate::regs::successors(insns);
    let defs: Vec<bool> = insns.iter().map(|i| access(i).defs.contains(&v)).collect();
    let mut inn = vec![false; insns.len()];
    loop {
        let mut changed = false;
        for j in 0..insns.len() {
            if !(inn[j] || defs[j]) {
                continue;
            }
            for s in succ[j].iter().flatten() {
                if !inn[*s] {
                    inn[*s] = true;
                    changed = true;
                }
            }
        }
        if !changed {
            return inn;
        }
    }
}

/// Whether `insn` writes all of `v` without reading it (no reload is
/// needed before it).
fn full_def(insn: &Insn, v: Reg) -> bool {
    match insn {
        Insn::Moveq { .. } | Insn::Movea { .. } | Insn::Lea { .. } | Insn::SubaSelf(_) => true,
        Insn::Move {
            size: Size::L,
            dst: Ea::Reg(r),
            ..
        }
        | Insn::Clr {
            size: Size::L,
            dst: Ea::Reg(r),
        } => *r == v,
        _ => false,
    }
}

/// The `tst` after a store that sets the flags `insn` set by writing the
/// data register `v` (`move`, `moveq`, `clr`: N and Z of the value, V and
/// C clear, X kept), or `None`.
fn retest(insn: &Insn, v: Reg) -> Option<Size> {
    match insn {
        Insn::Moveq { dst, .. } if Reg::D(*dst) == v => Some(Size::L),
        Insn::Move {
            size,
            dst: Ea::Reg(r),
            ..
        }
        | Insn::Clr {
            size,
            dst: Ea::Reg(r),
        } if *r == v && matches!(v, Reg::D(_)) => Some(*size),
        _ => None,
    }
}

/// How a value is spilled.
#[derive(Clone, Debug, PartialEq)]
enum Form {
    /// To a frame slot of its own: reloads before its reads, stores after
    /// its writes.
    Slot,
    /// Loaded again from where it was loaded: its one write is `insn`, a
    /// whole-register load of a parameter slot that holds an address
    /// (`remat`) and that the body never writes.  No slot and no store.
    Remat(Insn),
    /// An address value parked in a new data value: `move.l At,Dw` after
    /// its writes, `movea.l Dw,At` before its reads (the image keeps such
    /// values in data registers: the `with` temporary of msg_$openi in D2
    /// across SOCK_$OPEN; codegen-patterns 12.2, "Pointers in data
    /// registers").  Chosen where data registers are free over its range.
    Cross,
}

/// Whether the body writes, or takes the address of, any frame byte of
/// `d..d+4` (or of an unknown part of the frame).
fn frame_touched(insns: &[Insn], d: i32) -> bool {
    let hits = |ea: &Ea, n: i32| match ea {
        Ea::Disp(o, 6) => *o < d + 4 && d < *o + n,
        Ea::Ind(6) => 0 < d + 4 && d < n,
        Ea::Idx(_, 6, ..) | Ea::PreDec(6) | Ea::PostInc(6) => true,
        _ => false,
    };
    insns.iter().any(|i| match i {
        // A local's address reaches the locals only; another's anything
        // after it.
        Insn::Pea(ea) | Insn::Lea { src: ea, .. } => match ea {
            Ea::Disp(o, 6) if *o < 0 => hits(ea, -*o),
            _ => hits(ea, 1 << 16),
        },
        _ => {
            let eas = i.eas();
            // Every operand but a whole-register load's source may be
            // written (conservative: also those only read).
            let reads_only = match i {
                Insn::Move { .. } | Insn::Movea { .. } | Insn::Cmp { .. } | Insn::Alu { .. } => 1,
                _ => 0,
            };
            eas.iter().skip(reads_only).any(|ea| hits(ea, 4))
        }
    })
}

/// How `v` can be spilled in `insns`, or `None`: every reload and store
/// it needs keeps the condition codes, and no instruction that names it is
/// a `dbf` or a `case` dispatch.
fn spill_form(
    insns: &[Insn],
    v: Reg,
    cc_in: &[bool],
    cc_out: &[bool],
    live_out: &[BTreeSet<RegKey>],
    remat: &[i32],
) -> Option<Form> {
    if !is_virtual(v) {
        return None;
    }
    let mut defs: Vec<usize> = Vec::new();
    let written = written_before(insns, v);
    for (j, insn) in insns.iter().enumerate() {
        let a = access(insn);
        let (u, d) = (a.uses.contains(&v), a.defs.contains(&v));
        if !u && !d {
            continue;
        }
        if matches!(
            insn,
            Insn::Dbf { .. } | Insn::TableIndex { .. } | Insn::TableJump { .. }
        ) {
            return None;
        }
        if d {
            defs.push(j);
        }
        let reload = u || (written[j] && !full_def(insn, v));
        // `movea` keeps the flags; `move` into a data register does not.
        if reload && matches!(v, Reg::D(_)) && cc_in[j] {
            return None;
        }
    }
    // A parameter slot's address loaded once, at full width.
    if let [j] = defs[..] {
        let load = match &insns[j] {
            Insn::Movea {
                size: Size::L,
                src: Ea::Disp(d, 6),
                dst,
            } if Reg::A(*dst) == v => Some(*d),
            Insn::Move {
                size: Size::L,
                src: Ea::Disp(d, 6),
                dst: Ea::Reg(r),
            } if *r == v => Some(*d),
            _ => None,
        };
        if let Some(d) = load {
            if remat.contains(&d) && !frame_touched(insns, d) {
                return Some(Form::Remat(insns[j].clone()));
            }
        }
    }
    for &j in &defs {
        if cc_out[j] && retest(&insns[j], v).is_none() {
            return None;
        }
    }
    if matches!(v, Reg::A(_)) && data_free(insns, v, live_out) {
        return Some(Form::Cross);
    }
    Some(Form::Slot)
}

/// Whether a data register looks free wherever `v` is live: fewer than
/// eight data values live there, and fewer than six (D2-D7) across a
/// call.
fn data_free(insns: &[Insn], v: Reg, live_out: &[BTreeSet<RegKey>]) -> bool {
    insns.iter().zip(live_out).all(|(i, out)| {
        if !out.contains(&RegKey(v)) {
            return true;
        }
        let n = out.iter().filter(|r| matches!(r.0, Reg::D(_))).count();
        let limit = if matches!(i, Insn::Jsr(_) | Insn::Bsr { .. }) { 6 } else { 8 };
        n < limit
    })
}

/// The value to spill when `failed` found no register, and how: among
/// `failed` and the values of its class that interfere with it, those
/// that can be spilled, the one live longest per instruction that names
/// it.
fn choose(
    insns: &[Insn],
    exit: &[Reg],
    failed: Reg,
    temps: &BTreeSet<RegKey>,
    remat: &[i32],
) -> Option<(Reg, Form)> {
    let live_out = liveness(insns, exit, &[]);
    let (cc_in, cc_out) = cc_live(insns);
    let mut near: BTreeSet<RegKey> = BTreeSet::new();
    near.insert(RegKey(failed));
    for (j, insn) in insns.iter().enumerate() {
        let a = access(insn);
        if a.defs.contains(&failed) {
            near.extend(live_out[j].iter().copied());
        }
        if live_out[j].contains(&RegKey(failed)) {
            near.extend(a.defs.iter().map(|&r| RegKey(r)));
        }
    }
    let mut span: HashMap<RegKey, usize> = HashMap::new();
    let mut names: HashMap<RegKey, usize> = HashMap::new();
    for (j, insn) in insns.iter().enumerate() {
        for r in &live_out[j] {
            *span.entry(*r).or_default() += 1;
        }
        let a = access(insn);
        let mut seen: Vec<Reg> = a.uses.clone();
        seen.extend(a.defs.iter().copied().filter(|d| !a.uses.contains(d)));
        for r in seen {
            *names.entry(RegKey(r)).or_default() += 1;
        }
    }
    let mut best: Option<(u64, RegKey, Form)> = None;
    for &r in &near {
        if !same_class(r.0, failed) || temps.contains(&r) {
            continue;
        }
        let Some(form) = spill_form(insns, r.0, &cc_in, &cc_out, &live_out, remat) else {
            continue;
        };
        // Live instructions per naming instruction, scaled to compare as
        // integers; ties go to the first in register order.
        let score = (span.get(&r).copied().unwrap_or(0) as u64 * 1024)
            / names.get(&r).copied().unwrap_or(1).max(1) as u64;
        if best.as_ref().is_none_or(|(b, _, _)| score > *b) {
            best = Some((score, r, form));
        }
    }
    best.map(|(_, r, form)| (r.0, form))
}

/// `code` with `v` spilled in `form` (to the frame slot `(off,A6)` for
/// [`Form::Slot`]): reloads before the instructions that read it, stores
/// after those that write it (see the module comment).  `None` when the
/// virtual register numbers run out.
fn spill(
    code: &[(Tag, Insn)],
    v: Reg,
    form: &Form,
    off: i32,
    next: &mut Fresh,
    temps: &mut BTreeSet<RegKey>,
) -> Option<Vec<(Tag, Insn)>> {
    let insns: Vec<Insn> = code.iter().map(|(_, i)| i.clone()).collect();
    let (_, cc_out) = cc_live(&insns);
    let slot = Ea::Disp(off, crate::frame::FP);
    // The data value an address value is parked in.
    let park = match form {
        Form::Cross => Some(next.take(Reg::D(0))?),
        _ => None,
    };
    let written = written_before(&insns, v);
    let mut out = Vec::with_capacity(code.len() + 8);
    for (j, (tag, insn)) in code.iter().enumerate() {
        let a = access(insn);
        let (u, d) = (a.uses.contains(&v), a.defs.contains(&v));
        if !u && !d {
            out.push((*tag, insn.clone()));
            continue;
        }
        let k = match *tag {
            Tag::Orig(k) | Tag::Spill(k) => k,
        };
        let t = next.take(v)?;
        temps.insert(RegKey(t));
        let to_t = |i: &Insn| {
            let mut i = i.clone();
            rename(&mut i, &|r| if r == v { t } else { r });
            i
        };
        if u || (written[j] && !full_def(insn, v)) {
            let reload = match (form, t) {
                (Form::Remat(load), _) => to_t(load),
                (Form::Cross, Reg::A(n)) => Insn::Movea {
                    size: Size::L,
                    src: Ea::Reg(park.expect("a parked value")),
                    dst: n,
                },
                (Form::Cross, Reg::D(_)) => unreachable!("only address values are parked"),
                (Form::Slot, Reg::D(_)) => Insn::Move {
                    size: Size::L,
                    src: slot.clone(),
                    dst: Ea::Reg(t),
                },
                (Form::Slot, Reg::A(n)) => Insn::Movea {
                    size: Size::L,
                    src: slot.clone(),
                    dst: n,
                },
            };
            out.push((Tag::Spill(k), reload));
        }
        out.push((*tag, to_t(insn)));
        if d && matches!(form, Form::Slot | Form::Cross) {
            out.push((
                Tag::Spill(k),
                Insn::Move {
                    size: Size::L,
                    src: Ea::Reg(t),
                    dst: park.map_or(slot.clone(), Ea::Reg),
                },
            ));
            if cc_out[j] {
                let size = retest(insn, v).expect("checked by `spill_form`");
                out.push((
                    Tag::Spill(k),
                    Insn::Tst {
                        size,
                        dst: Ea::Reg(t),
                    },
                ));
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::insn::Cond;
    use crate::regs::VBASE;

    fn vd(k: u8) -> Reg {
        Reg::D(VBASE + k)
    }
    fn va(k: u8) -> Reg {
        Reg::A(VBASE + k)
    }
    fn an(r: Reg) -> u8 {
        match r {
            Reg::A(n) => n,
            Reg::D(_) => unreachable!(),
        }
    }
    fn jsr() -> Insn {
        Insn::Jsr(Ea::AbsL("X".into(), 0))
    }
    fn lea(d: i32, r: Reg) -> Insn {
        Insn::Lea {
            src: Ea::Disp(d, 5),
            dst: an(r),
        }
    }
    fn ld(d: i32, r: Reg) -> Insn {
        Insn::Move {
            size: Size::L,
            src: Ea::Disp(d, 5),
            dst: Ea::Reg(r),
        }
    }
    /// `move.l rd,(ra)`.
    fn st(rd: Reg, ra: Reg) -> Insn {
        Insn::Move {
            size: Size::L,
            src: Ea::Reg(rd),
            dst: Ea::Ind(an(ra)),
        }
    }
    fn clr_through(ra: Reg) -> Insn {
        Insn::Clr {
            size: Size::L,
            dst: Ea::Ind(an(ra)),
        }
    }
    fn text(insns: &[Insn]) -> Vec<String> {
        insns.iter().map(|i| i.to_string()).collect()
    }
    /// The spliced body.
    fn spliced(s: &Spilled) -> Vec<String> {
        let mut out = Vec::new();
        for (k, i) in s.insns.iter().enumerate() {
            out.extend(text(&s.before[k]));
            out.push(i.to_string());
            out.extend(text(&s.after[k]));
        }
        out
    }
    fn physical(s: &Spilled) -> bool {
        s.insns
            .iter()
            .chain(s.before.iter().flatten())
            .chain(s.after.iter().flatten())
            .all(|i| {
                let a = access(i);
                a.uses.iter().chain(a.defs.iter()).all(|r| !is_virtual(*r))
            })
    }

    /// Seven data values and six address values live across a call: the
    /// address values do not all fit A2-A4 and the data registers are
    /// full, so some go to frame slots (`movea.l` reloads, `move.l`
    /// stores), and data values too.
    #[test]
    fn address_and_data_values_to_slots() {
        let mut body = Vec::new();
        for k in 0..7 {
            body.push(ld(4 * k as i32, vd(k)));
        }
        for k in 0..6 {
            body.push(lea(100 + 4 * k as i32, va(k)));
        }
        body.push(jsr());
        for k in 0..6 {
            body.push(st(vd(k), va(k)));
        }
        body.push(st(vd(6), va(0)));
        let s = allocate_spilling(&body, &[], &[], 8, &[]).unwrap();
        assert!(physical(&s));
        assert!(s.bytes >= 8 && s.bytes % 4 == 0);
        let t = spliced(&s);
        assert!(
            t.iter().any(|l| l.starts_with("movea.l\t(-") && l.contains(",%fp),%a")),
            "{t:?}"
        );
        assert!(
            t.iter().any(|l| l.starts_with("move.l\t%a") && l.ends_with(",%fp)")),
            "{t:?}"
        );
        // The first slot is just below the 8 bytes given.
        assert!(t.iter().any(|l| l.contains("(-0xc,%fp)")), "{t:?}");
        assert!(t.iter().all(|l| !l.contains("(-8,%fp)")), "{t:?}");
    }

    /// Four address values across a call with the data registers free:
    /// one is parked in a data register.
    #[test]
    fn address_value_parked_in_a_data_register() {
        let mut body = Vec::new();
        for k in 0..4 {
            body.push(lea(4 * k as i32, va(k)));
        }
        body.push(jsr());
        for k in 0..4 {
            body.push(clr_through(va(k)));
        }
        let s = allocate_spilling(&body, &[], &[], 0, &[]).unwrap();
        assert!(physical(&s));
        assert_eq!(s.bytes, 0);
        let t = spliced(&s);
        assert!(
            t.iter().any(|l| l.starts_with("move.l\t%a") && l.contains(",%d")),
            "{t:?}"
        );
        assert!(t.iter().any(|l| l.starts_with("movea.l\t%d")), "{t:?}");
    }

    /// Address parameters live across a call: loaded again from their
    /// slots, no spill slot; not where the body writes the slot.
    #[test]
    fn address_parameters_loaded_again() {
        let mut body = Vec::new();
        for k in 0..4u8 {
            body.push(Insn::Movea {
                size: Size::L,
                src: Ea::Disp(8 + 4 * k as i32, 6),
                dst: an(va(k)),
            });
        }
        body.push(jsr());
        for k in 0..4 {
            body.push(clr_through(va(k)));
        }
        let s = allocate_spilling(&body, &[], &[], 0, &[8, 12, 16, 20]).unwrap();
        assert!(physical(&s));
        assert_eq!(s.bytes, 0);
        let t = spliced(&s);
        let reloads = t
            .iter()
            .filter(|l| l.starts_with("movea.l\t(") && l.contains(",%fp),"))
            .count();
        assert!(reloads >= 5, "{t:?}");
        let mut body2 = body.clone();
        body2.push(Insn::Clr {
            size: Size::L,
            dst: Ea::Disp(8, 6),
        });
        let live = liveness(&body2, &[], &[]);
        let (ci, co) = cc_live(&body2);
        assert_eq!(
            spill_form(&body2, va(0), &ci, &co, &live, &[8]),
            Some(Form::Cross)
        );
        assert_eq!(
            spill_form(&body2, va(1), &ci, &co, &live, &[12]),
            Some(Form::Remat(body[1].clone()))
        );
    }

    /// A word write that an earlier write of the value reaches (around a
    /// loop) is preceded by a reload of the slot.
    #[test]
    fn partial_write_after_a_write_reloads() {
        let w = |src: Ea| Insn::Move {
            size: Size::W,
            src,
            dst: Ea::Reg(vd(0)),
        };
        let body = vec![
            Insn::Label(1),
            w(Ea::Disp(8, 6)),
            jsr(),
            Insn::Move {
                size: Size::L,
                src: Ea::Reg(vd(0)),
                dst: Ea::Disp(-2, 5),
            },
            Insn::Branch {
                cond: Cond::Ne,
                label: 1,
                short: true,
                back: true,
            },
        ];
        let code: Vec<(Tag, Insn)> = body
            .iter()
            .cloned()
            .enumerate()
            .map(|(k, i)| (Tag::Orig(k), i))
            .collect();
        let mut next = Fresh::new(&body);
        let mut temps = BTreeSet::new();
        let out = spill(&code, vd(0), &Form::Slot, -4, &mut next, &mut temps).unwrap();
        let t: Vec<String> = out.iter().map(|(_, i)| i.to_string()).collect();
        let k = t
            .iter()
            .position(|l| l.starts_with("move.w\t(8,%fp)"))
            .unwrap();
        assert!(t[k - 1].starts_with("move.l\t(-4,%fp),%d"), "{t:?}");
    }

    /// The condition codes: a store after a `move` whose flags a branch
    /// reads is followed by a `tst`; an address value written where the
    /// flags are live, and a `dbf` counter, are not spilled.
    #[test]
    fn condition_codes_kept() {
        let body = vec![
            Insn::Moveq {
                value: 3,
                dst: VBASE + 1,
            },
            Insn::Move {
                size: Size::W,
                src: Ea::Disp(8, 6),
                dst: Ea::Reg(vd(0)),
            },
            Insn::Movea {
                size: Size::L,
                src: Ea::Disp(10, 6),
                dst: an(va(0)),
            },
            Insn::Branch {
                cond: Cond::Mi,
                label: 1,
                short: true,
                back: false,
            },
            st(vd(0), va(0)),
            Insn::Label(1),
            Insn::Dbf {
                reg: VBASE + 1,
                label: 1,
                until: None,
            },
        ];
        let live = liveness(&body, &[], &[]);
        let (ci, co) = cc_live(&body);
        assert_eq!(spill_form(&body, va(0), &ci, &co, &live, &[]), None);
        assert_eq!(spill_form(&body, vd(1), &ci, &co, &live, &[]), None);
        assert_eq!(
            spill_form(&body, vd(0), &ci, &co, &live, &[]),
            Some(Form::Slot)
        );
        let code: Vec<(Tag, Insn)> = body
            .iter()
            .cloned()
            .enumerate()
            .map(|(k, i)| (Tag::Orig(k), i))
            .collect();
        let mut next = Fresh::new(&body);
        let mut temps = BTreeSet::new();
        let out = spill(&code, vd(0), &Form::Slot, -4, &mut next, &mut temps).unwrap();
        let t: Vec<String> = out.iter().map(|(_, i)| i.to_string()).collect();
        let k = t
            .iter()
            .position(|l| l.starts_with("move.w\t(8,%fp)"))
            .unwrap();
        // A word write that no earlier write reaches: no reload first;
        // stored whole, the flags set again.
        assert!(!t[k - 1].contains("(-4,%fp)"), "{t:?}");
        assert!(
            t[k + 1].starts_with("move.l\t%d") && t[k + 1].ends_with(",(-4,%fp)"),
            "{t:?}"
        );
        assert!(t[k + 2].starts_with("tst.w\t%d"), "{t:?}");
    }
}
