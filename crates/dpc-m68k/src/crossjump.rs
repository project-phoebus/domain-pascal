//! Cross-jumping: the identical instruction tails of the arms that reach
//! one join are merged, every arm but one ending in a `bra` into the
//! kept copy (bead domain-pascal-k1da; codegen-patterns 7.1, "Identical
//! ends of the two parts", 1.1, "Shared pop at a join").
//!
//! Ref 6.4.24, p. 6-27 lists "cross-jumping" among the `-opt` levels'
//! optimizations without saying more.  The image: SAU2 0xE06156 `bsr.w;
//! bra.s 0xE06194` where 0xE06194 is the other part's `addq.w #4,SP` (the
//! 56 shared pops of section 1.1; the 45 pairs whose pops differ in size
//! pop apart); RIP_$SEND_UPDATES 0xE688B8 `st -(SP); bra.s 0xE688BE` into
//! the other part's `jsr RIP_$BROADCAST`; LOG_$ADD 0xE176C8 `beq.s
//! 0xE176D2`, the `then` part gone, its one store the inner `if`'s copy;
//! PARITY_$CHK_IO 0xE0B1A4 `bne.s 0xE0B1AA` to the outer `else` part's
//! `clr.w D0`; NETWORK_$REPORT_FAILURE 0xE10428 `moveq #1,D0; bra.s` to
//! the other arm's `move.l D0,(772,A5)`; SAU9 SCSI_$C90_INT 0x3C4A5B36
//! `bra.s 0x3C4A5BA0` into the last `case` arm's `move.l D4,-(SP); pea
//! (A3); pea (A2); bsr.w; lea (16,SP),SP` from six arms, and 0x3C4A57E6
//! `pea (cell,PC); bra.s 0x3C4A580C` into the last `crash_system` call's
//! `jsr; addq.w #4,SP` (../source `evidence/domain_os/scsi/`, 2026-10-29).
//!
//! The rule (Verified at the sites above; which arms are merged when no
//! arm falls through to the join Uncertain, one site each): the arm that
//! falls through to the join keeps the tail and every other arm's
//! identical suffix becomes a `bra` to it (forward from the arms before
//! it, backward from one after it: TERM_$CONTROL 0xE66C24 `moveq #2,D0;
//! bra.s 0xE66C10` back to the first arm's `move.l D0,(-28,A6); bra.w`
//! where neither arm falls through, the first kept; SAU9 SCSI_$C90_INT's
//! seven arms all `bra` to the join and the last is kept).  So without a
//! falling-through arm the MC68010 code keeps the first arm's copy and
//! the MC68020 code the last's.  An arm that is nothing but the shared
//! tail is dropped and the branches to it retargeted (LOG_$ADD's `beq.s
//! 0xE176D2`).  Not merged in the image (Uncertain): 0xE381A8 and
//! 0xE381D2, two `jsr 0xE825E4` ending the two parts of one `if`, and
//! 0xE1A9F0/0xE1AA00, two identical `move.w 0xE2E00E,D0` (4 of the 3324
//! forward `bra`s to a join reached by falling through have the same
//! instruction before the `bra` and before the join).
//!
//! The pass runs on the final instruction list of a routine, after
//! allocation and scheduling and before the branches are sized: a tail
//! is a run of straight-line instructions (no label, branch, loop
//! instruction, jump, table or data cell) equal in every operand.

use std::collections::HashMap;

use crate::insn::{Cond, Insn};
use crate::Cpu;

/// Merge the identical tails of `out`'s joins (the module doc).
pub fn merge_tails(out: &mut Vec<Insn>, cpu: Cpu) {
    let mut emptied = Vec::new();
    while merge_one(out, cpu, &mut emptied) {}
    invert_over_bra(out, &emptied);
    fix_directions(out);
}

/// `b<cc> L1; bra L2; L1:` after a merge (an arm that was nothing but the
/// shared tail, entered by falling through its condition's branch) is
/// `b<!cc> L2` (SIO_$K_SET_PARAM 0xE68258 `cmpi.w #3,(20,A2); bhi.s
/// 0xE682D4`, the three `in_range` checks sharing one status store;
/// LOG_$ADD 0xE176C8 `beq.s 0xE176D2`).
fn invert_over_bra(out: &mut Vec<Insn>, emptied: &[u32]) {
    let mut k = 0;
    while k + 2 < out.len() {
        if let (
            Insn::Branch {
                cond, label: l1, ..
            },
            Insn::Branch {
                cond: Cond::T,
                label: l2,
                ..
            },
            Insn::Label(l),
        ) = (&out[k], &out[k + 1], &out[k + 2])
        {
            if *cond != Cond::T && l1 == l && emptied.contains(l) {
                let (c, l2) = (cond.negate(), *l2);
                out[k] = Insn::Branch {
                    cond: c,
                    label: l2,
                    short: true,
                    back: false,
                };
                out.remove(k + 1);
                continue;
            }
        }
        k += 1;
    }
}

/// Whether `i` may be part of a shared tail.
fn straight(i: &Insn) -> bool {
    !matches!(
        i,
        Insn::Label(_)
            | Insn::Branch { .. }
            | Insn::Dbf { .. }
            | Insn::DbfFwd { .. }
            | Insn::Jmp(_)
            | Insn::Rts
            | Insn::Unlk
            | Insn::Link { .. }
            | Insn::Align4
            | Insn::PoolLabel(_)
            | Insn::Words(_)
            | Insn::TableEntry { .. }
            | Insn::TableJump { .. }
            | Insn::TableIndex { .. }
    )
}

/// Whether nothing falls through `i` to the instruction after it.
fn terminator(i: &Insn) -> bool {
    matches!(
        i,
        Insn::Branch { cond: Cond::T, .. } | Insn::Jmp(_) | Insn::Rts | Insn::TableJump { .. }
    )
}

/// The length of the identical straight-line suffix before `a` and `b`.
fn common_suffix(out: &[Insn], a: usize, b: usize) -> usize {
    let mut k = 0;
    while k < a && k < b {
        let (x, y) = (&out[a - 1 - k], &out[b - 1 - k]);
        if x != y || !straight(x) {
            break;
        }
        k += 1;
    }
    k
}

/// One merge, if any join has one: `true` when `out` changed.
///
/// A join's predecessors are the forward `bra`s to its label and the
/// arm falling through to it.  An arm merges into the falling-through
/// arm when their tails agree; otherwise, in MC68010 code, into the
/// earliest arm before it whose tail agrees (TERM_$CONTROL 0xE66C24), in
/// MC68020 code into the latest arm after it (SCSI_$C90_INT 0x3C4A5B36).
fn merge_one(out: &mut Vec<Insn>, cpu: Cpu, emptied: &mut Vec<u32>) -> bool {
    let at: HashMap<u32, usize> = out
        .iter()
        .enumerate()
        .filter_map(|(k, i)| match i {
            Insn::Label(l) => Some((*l, k)),
            _ => None,
        })
        .collect();
    // Each join label's predecessors, in textual order: the index of a
    // forward `bra` to it, or of the label itself for the arm falling
    // through (`true`).
    let mut preds: HashMap<u32, Vec<(usize, bool)>> = HashMap::new();
    let mut order: Vec<u32> = Vec::new();
    for (k, i) in out.iter().enumerate() {
        let (l, ft) = match i {
            Insn::Branch {
                cond: Cond::T,
                label,
                ..
            } if at.get(label).is_some_and(|&p| p > k) => (*label, false),
            Insn::Label(l) if k > 0 && !terminator(&out[k - 1]) => (*l, true),
            _ => continue,
        };
        if !preds.contains_key(&l) {
            order.push(l);
        }
        preds.entry(l).or_default().push((k, ft));
    }
    for l in order {
        let ps = &preds[&l];
        if ps.len() < 2 {
            continue;
        }
        let fall = ps.iter().position(|&(_, ft)| ft);
        for (pi, &(pend, ft)) in ps.iter().enumerate() {
            if ft {
                continue;
            }
            let mut partner = None;
            if let Some(f) = fall {
                let k = common_suffix(out, pend, ps[f].0);
                if k > 0 {
                    partner = Some((ps[f].0, k));
                }
            }
            if partner.is_none() {
                let others: Vec<usize> = if cpu.is_68020() {
                    (pi + 1..ps.len()).rev().collect()
                } else {
                    (0..pi).collect()
                };
                for q in others {
                    if ps[q].1 {
                        continue;
                    }
                    let k = common_suffix(out, pend, ps[q].0);
                    if k > 0 {
                        partner = Some((ps[q].0, k));
                        break;
                    }
                }
            }
            if let Some((rend, k)) = partner {
                apply(out, pend, rend, k, emptied);
                return true;
            }
        }
    }
    false
}

/// Replace the `k` instructions before `pend` and the `bra` at `pend`
/// by a `bra` to the `k` instructions before `rend`.  `emptied`: the
/// labels of arms left as a lone `bra` ([`invert_over_bra`]).
fn apply(out: &mut Vec<Insn>, pend: usize, rend: usize, k: usize, emptied: &mut Vec<u32>) {
    let mut target = rend - k;
    let mut inserted = false;
    let label = match target.checked_sub(1).map(|t| &out[t]) {
        Some(Insn::Label(l)) => *l,
        _ => {
            let l = fresh_label(out);
            out.insert(target, Insn::Label(l));
            target += 1;
            inserted = true;
            l
        }
    };
    // The arm's indices shift by one when a label went in before it.
    let shift = usize::from(inserted && pend > target);
    let (start, end) = (pend + shift - k, pend + shift);
    let bra = Insn::Branch {
        cond: Cond::T,
        label,
        short: true,
        back: end > target,
    };
    out.splice(start..=end, [bra]);
    // An arm that was nothing but the tail, entered by falling through
    // its condition's branch over it: that branch inverts
    // ([`invert_over_bra`]).
    if let (Some(Insn::Branch { cond, label: x, .. }), Some(Insn::Label(y))) =
        (start.checked_sub(1).map(|t| &out[t]), out.get(start + 1))
    {
        if *cond != Cond::T && x == y {
            emptied.push(*x);
        }
    }
    // An arm that was nothing but the tail: its label's branches go
    // to the kept copy; entered by a branch only, the arm goes too.
    if let Some(Insn::Label(l)) = start.checked_sub(1).map(|t| out[t].clone()) {
        if l != label {
            let entered = start < 2 || !terminator(&out[start - 2]);
            retarget(out, l, label);
            if entered {
                emptied.push(l);
            } else {
                out.drain(start - 1..=start);
            }
        }
    }
}

/// Every branch and table entry to `from` goes to `to`.
fn retarget(out: &mut [Insn], from: u32, to: u32) {
    for i in out.iter_mut() {
        match i {
            Insn::Branch { label, .. }
            | Insn::Dbf { label, .. }
            | Insn::DbfFwd { label, .. }
            | Insn::TableEntry { target: label, .. }
                if *label == from =>
            {
                *label = to;
            }
            _ => {}
        }
    }
}

/// A label number no instruction of `out` uses.
fn fresh_label(out: &[Insn]) -> u32 {
    out.iter()
        .filter_map(|i| match i {
            Insn::Label(l)
            | Insn::Branch { label: l, .. }
            | Insn::Dbf { label: l, .. }
            | Insn::DbfFwd { label: l, .. }
            | Insn::TableEntry { target: l, .. } => Some(*l),
            _ => None,
        })
        .max()
        .map_or(0, |m| m + 1)
}

/// Set every branch's and table entry's `back` flag from the positions.
fn fix_directions(out: &mut [Insn]) {
    let at: HashMap<u32, usize> = out
        .iter()
        .enumerate()
        .filter_map(|(k, i)| match i {
            Insn::Label(l) => Some((*l, k)),
            _ => None,
        })
        .collect();
    for k in 0..out.len() {
        match &mut out[k] {
            Insn::Branch { label, back, .. }
            | Insn::TableEntry {
                target: label,
                back,
                ..
            } => {
                if let Some(&p) = at.get(label) {
                    *back = p < k;
                }
            }
            _ => {}
        }
    }
}
