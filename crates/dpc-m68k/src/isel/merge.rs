//! Consecutive stores to adjacent memory: kept apart, merged, or refused
//! (ref 3.15.3 p. 3-57; codegen-patterns 13, "Store merging"; beads
//! domain-pascal-2m0, domain-pascal-qb9q).
//!
//! Ref 3.15.3, p. 3-57 ("Device"): "By default, the compiler optimizes
//! certain adjacent references by merging them into one large reference"
//! (`a := 0; b := 0` on two `integer16`s becomes one 32-bit store); ref
//! 6.4.24, p. 6-28 lists "Merges assignment statements where possible"
//! under `-opt 1`.  The image merges constant pairs in some units and keeps
//! them apart in others (`tools/storesurvey.py units`: of the map segments
//! that show either, 21 merge, 9 keep apart a pair of a shape merged
//! elsewhere, and 1 shows both, PROC2 0xE303D8, whose pair apart is two
//! word locals, which the in-unit rule keeps apart), and the `-opt`
//! level does not explain it (docs/opt-levels.md, bead domain-pascal-g728).  So whether a unit merges is a per-unit
//! setting, `dpc --merge-constant-stores` (owner decision 2026-10-16,
//! bead domain-pascal-qb9q); what sets it in the original is not known.
//!
//! **A pair** is two consecutive statements of one statement sequence (a
//! body, an `if` arm, a loop body, a `case` arm) that store to adjacent
//! objects of one base (the frame, the unit's data, another unit's object,
//! one pointer value, the block of another data section, the enclosing
//! frame) and fit together in a word or a longword: the candidates for one
//! reference.  Not candidates, and so always two stores:
//! - a byte and a word (3 bytes) cannot be one 68000 reference, as at
//!   module-local 0xE32B76, 0xE32BA6/0xE32BAA (`clr.b (0x36,A2); move.w
//!   D2,(0x34,A2)`).  TIME_$Q_INIT_QUEUE's stores (0..4, 4..8, 8, 10..12)
//!   never fit: 0xE16C6E..0xE16C78;
//! - a pair whose values are not both constants and not both loads of
//!   adjacent memory in the same arrangement (a computed value, a
//!   register, a load from elsewhere): no one reference stores both (a
//!   merge would need the two put together in a register, which no store
//!   in the compiled code shows), and the image keeps 270 such adjacent
//!   pairs apart (TPAD_$SET_UNIT_MODE 0xE69830..0xE69840; 0xE02D40
//!   `move.w D2,(-0x4e,A6); move.w D3,(-0x50,A6)`; 0xE07526);
//! - two bytes whose lower one is at an odd address in the frame or the
//!   module's static data (an even base): a word there is an odd-address
//!   reference, which the 68010 traps.  The image's 13 such pairs are all
//!   apart (MST_$DISKLESS_INIT 0xE30DD2, 0xE62A42, 0xE04CAC, 0xE19F1A);
//! - a longword and, above it, a word: merged and split again at the
//!   longword (FIM_$BUILD_DF 0xE0A896 `clr.l (0x54,A2); clr.w (0x58,A2)`)
//!   or not, they are the same two stores, made in source order (0xE70B5E
//!   `clr.w (0x1a0,A5); clr.l (0x19c,A5)`);
//! - `device` objects, which the reference exempts from merging (ref
//!   3.15.3).
//!
//! **Copies** of adjacent memory to adjacent memory are refused in every
//! unit: the image merges some (ROUTE_$SHORT_PORT 0xE69C16 copies
//! route_$port_t's 0x2C..0x2F as one longword, RIP_$TABLE 0xE6905E its
//! 0x2E..0x31) and keeps others apart in the same units (RIP_$TABLE_D
//! 0xE68ECA, the five of ASKNODE_$INTERNET_INFO 0xE647BE..): copies do not
//! follow the per-unit setting.
//!
//! **Constants without the knob** (a unit that does not merge): the two
//! stores are made apart, as the image's units without a merge do, for
//! the shapes those units show apart ([`apart_without_merging`]); a shape
//! they do not show is refused.
//!
//! **Constants with the knob** ([`Ctx::decide`]): two words stored in
//! ascending order are one longword, `clr.l` for two zeros, else the two
//! words' bits as one `move.l #imm` (TTY_$K_RESET 0xE67322 `move.l
//! #0x10001,(0x2ca,A2)`; LOG_$INIT 0xE3017C, the one merged value in
//! `moveq`'s range, `moveq #1,D0; move.l D0,(A0)`, which is how any
//! longword constant in that range is stored: [`super::ConstForm`]); a run
//! of three words
//! pairs the first two (5 runs: KBD_$INIT 0xE333B6, 0xE333C4,
//! TTY_$K_RESET 0xE67332, ACL_ 0xE4521E, 0xE45C1C); the shapes the
//! merging units keep apart (bytes, words descending or off A5, two word
//! locals) are two stores ([`apart_when_merging`], [`Ctx::decide`]);
//! everything else the merging units do not settle is refused.

use std::borrow::Cow;

use dpc_ir::{Addr, Expr, Function, Scalar, Stmt, StmtKind, Width};
use dpc_syntax::Span;

use super::{expr_has_call, register_locals, split_addr, unsupported, Res};

/// The stores that stand for two statements' word stores merged (the
/// first statement's span and the longword's address).
type Merged = Vec<(Span, Addr)>;

/// Check every pair of consecutive stores to adjacent memory of `f`'s
/// body (see the module doc) and, with `merge` (`dpc
/// --merge-constant-stores`), merge the pairs the image merges.  -> the
/// function to select (`f` itself when nothing merged).  A merged
/// longword is an ordinary constant store: in `moveq`'s range it is
/// stored from a register (LOG_$INIT 0xE3017C), as [`super::ConstForm`]
/// says for every longword constant.
pub(super) fn adjacent_stores(f: &Function, merge: bool) -> Res<Cow<'_, Function>> {
    let ctx = Ctx::new(f, merge);
    let mut merged = Merged::new();
    let body = ctx.seq(&f.body, &mut merged)?;
    if merged.is_empty() {
        return Ok(Cow::Borrowed(f));
    }
    let mut g = f.clone();
    g.body = body;
    Ok(Cow::Owned(g))
}

/// The kind of a pair's common base ([`split_addr`]), as the image's
/// survey classes bases (`tools/storesurvey.py merge`): `a6` the frame,
/// `a5` the unit's data, `abs` an absolute address, `reg` an address
/// register (a pointer, another data section's block loaded with `movea.l
/// #`, the enclosing frame through the static link).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Base {
    Frame,
    Static,
    Symbol,
    Pointer,
    OtherStatic,
    Enclosing,
}

impl Base {
    fn of(base: &Addr) -> Option<Base> {
        Some(match base {
            Addr::Frame(_) => Base::Frame,
            Addr::Static(_) => Base::Static,
            Addr::Symbol { .. } => Base::Symbol,
            Addr::Indirect { .. } => Base::Pointer,
            Addr::OtherStatic { .. } => Base::OtherStatic,
            Addr::Enclosing(_) => Base::Enclosing,
            _ => return None,
        })
    }

    /// The survey's base kind.
    fn image_kind(self) -> &'static str {
        match self {
            Base::Frame => "a6",
            Base::Static => "a5",
            Base::Symbol => "abs",
            Base::Pointer | Base::OtherStatic | Base::Enclosing => "reg",
        }
    }
}

/// Whether the image's units without a merge keep a pair of constants of
/// this shape apart (`tools/storesurvey.py units`, "pairs apart in units
/// without a merge"): `first` and `second` the stores' sizes in statement
/// order, `ascending` the second above the first.
///
/// - two bytes ascending in the frame (33, 0xE6173C), absolute (1,
///   0xE30DDA) and through a register (18, 0xE1217E); never off A5 (the
///   two such pairs are in SMD 0xE6D700, which merges); descending only
///   in the frame (1, 0xE61788);
/// - two words ascending off A5 (3: NETLOG_$CNTL 0xE719A4, 0xE71AB2), in
///   the frame (7: DISK 0xE6C9FA, RING 0xE775AE; among them
///   NETWORK_$PAGE_SERVER 0xE117C6 `move.w #-1,(-0x2f8,A6); move.w
///   #-1,(-0x2f6,A6)`, overflow_rqst.words[0..1] of an array local at 0
///   mod 4, network/page_server.pas:160: the shape a merging unit merges,
///   kept apart, so a frame record's words too are two stores without
///   the knob) and through a register (14: TTY_$I_INIT 0xE3332E, XNS_IDP
///   0xE18F8C, TPAD_$INIT 0xE3363E); never absolute;
/// - two words descending, every base: 26 (A5 1, TPAD_$SET_UNIT_CURSOR
///   0xE69902; frame 4; absolute 13, MST_UNWIRED 0xE30A04..; register 8,
///   AREA_$INIT 0xE2F3C0);
/// - a word and a longword above it, ascending off A5 (NETLOG 0xE71AC6),
///   absolute (0xE2FF2C) and through a register (4: PMAP 0xE1465E,
///   0xE2F8BA); descending (the longword first) in the frame (0xE609E4)
///   and through a register (2: 0xE0C958, 0xE1124C).
fn apart_without_merging(first: u32, second: u32, ascending: bool, base: Base) -> bool {
    let k = base.image_kind();
    match (first, second, ascending) {
        (1, 1, true) => k != "a5",
        (1, 1, false) => k == "a6",
        (2, 2, true) => k != "abs",
        (2, 2, false) => true,
        (2, 4, true) => k != "a6",
        (4, 2, false) => matches!(k, "a6" | "reg"),
        _ => false,
    }
}

/// Whether the image's merging units keep a pair of constants of this
/// shape apart, other than two words ascending ([`Ctx::decide`]): the
/// shapes of `tools/storesurvey.py units`, "in merging units", pairs
/// apart, none of them merged anywhere in the image:
///
/// - two bytes ascending off A5 (SMD 0xE6D838, 0xE6D844), in the frame
///   (ACL_ 0xE4941E) and through a register (5: XNS_ERROR_$SEND
///   0xE17B48, ACL_ 0xE44EC6, 0xE47916, 0xE47922, 0xE47DD8); descending
///   through a register (ACL_ 0xE46074);
/// - two words descending off A5 (FLP_DO_IO 0xE3DE4A, WIN_ 0xE1955C).
///
/// Other shapes (bytes descending in the frame, off A5 or absolute; bytes
/// ascending absolute; words descending elsewhere; a word below a
/// longword) have no site in a merging unit.
fn apart_when_merging(first: u32, second: u32, ascending: bool, base: Base) -> bool {
    let k = base.image_kind();
    match (first, second, ascending) {
        (1, 1, true) => k != "abs",
        (1, 1, false) => k == "reg",
        (2, 2, false) => k == "a5",
        _ => false,
    }
}

/// A pair of consecutive stores ([`Ctx::pair`]).
enum Pair {
    /// Not a candidate for one reference.
    No,
    /// Adjacent memory copied to adjacent memory.
    Copy,
    /// Two constants: the sizes in statement order, whether the second
    /// is above the first, the base's kind.
    Consts {
        first: u32,
        second: u32,
        ascending: bool,
        base: Base,
    },
}

/// What the knob does with the first two words of a run.
enum Decision {
    Merge,
    Apart,
}

struct Ctx<'f> {
    f: &'f Function,
    merge: bool,
    /// Register-held locals ([`register_locals`]): a store to one writes
    /// no memory.
    held: Vec<i32>,
    /// The frame objects known to be scalars, as (offset, bytes): the
    /// scalar locals, the function result variable, the copies of value
    /// parameters and the objects nested routines reach of 4 bytes or
    /// less.
    scalars: Vec<(i32, u32)>,
}

impl<'f> Ctx<'f> {
    fn new(f: &'f Function, merge: bool) -> Ctx<'f> {
        let held = register_locals(f).iter().map(|&(o, _)| o).collect();
        let mut scalars: Vec<(i32, u32)> = f
            .scalar_locals
            .iter()
            .map(|l| (l.offset, l.ty.bytes()))
            .collect();
        if let (Some(o), Some(r)) = (f.result_slot, &f.result) {
            scalars.push((o, r.ty.bytes()));
        }
        scalars.extend(
            f.param_copies
                .iter()
                .filter(|c| c.bytes <= 4)
                .map(|c| (c.copy, c.bytes)),
        );
        scalars.extend(f.uplevel_refs.iter().filter(|&&(_, n)| n <= 4).copied());
        Ctx {
            f,
            merge,
            held,
            scalars,
        }
    }

    /// A statement with its nested sequences checked (and merged).
    fn nested(&self, st: &Stmt, merged: &mut Merged) -> Res<Stmt> {
        let mut st = st.clone();
        match &mut st.kind {
            StmtKind::If { then_, else_, .. } => {
                *then_ = self.seq(then_, merged)?;
                *else_ = self.seq(else_, merged)?;
            }
            StmtKind::Repeat { body, .. } | StmtKind::While { body, .. } => {
                *body = self.seq(body, merged)?
            }
            StmtKind::For(l) => l.body = self.seq(&l.body, merged)?,
            StmtKind::Case {
                arms, otherwise, ..
            } => {
                for a in arms.iter_mut() {
                    a.body = self.seq(&a.body, merged)?;
                }
                if let Some(o) = otherwise {
                    *o = self.seq(o, merged)?;
                }
            }
            _ => {}
        }
        Ok(st)
    }

    /// The address and value of `st` when it is a store to memory (not
    /// to a register-held local, which writes no memory).
    fn mem_store<'a>(&self, st: &'a Stmt) -> Option<(&'a Addr, &'a Expr)> {
        match &st.kind {
            StmtKind::Store { addr, value } => {
                if matches!(addr, Addr::Frame(o) if self.held.contains(o)) {
                    None
                } else {
                    Some((addr, value))
                }
            }
            _ => None,
        }
    }

    /// One statement sequence: its pairs checked, and with the knob its
    /// runs of ascending word constants paired from the first.
    fn seq(&self, body: &[Stmt], merged: &mut Merged) -> Res<Vec<Stmt>> {
        let stmts: Vec<Stmt> = body
            .iter()
            .map(|st| self.nested(st, merged))
            .collect::<Res<_>>()?;
        // With the knob: the first statement of each merged pair, and the
        // statements merged into the one before.
        let mut merge_at: Vec<Option<i64>> = vec![None; stmts.len()];
        let mut drop = vec![false; stmts.len()];
        let mut run: Vec<usize> = Vec::new();
        let mut prev: Option<usize> = None;
        for k in 0..stmts.len() {
            let Some((addr, value)) = self.mem_store(&stmts[k]) else {
                self.end_run(&stmts, &mut run, &mut merge_at, &mut drop)?;
                prev = None;
                continue;
            };
            let pair = match prev {
                Some(j) => {
                    let (pa, pv) = self.mem_store(&stmts[j]).unwrap();
                    self.pair(pa, pv, addr, value)
                }
                None => Pair::No,
            };
            let span = stmts[k].span;
            match pair {
                Pair::No => self.end_run(&stmts, &mut run, &mut merge_at, &mut drop)?,
                Pair::Copy => {
                    return Err(unsupported(
                        span,
                        "a store adjacent to the previous statement's store, both copying \
                         adjacent memory (one longword: merged at ROUTE_$SHORT_PORT 0xE69C16, \
                         apart at RIP_$TABLE_D 0xE68ECA in the same kind of unit; ref 3.15.3 \
                         p. 3-57, codegen-patterns 13), is",
                    ))
                }
                Pair::Consts {
                    first,
                    second,
                    ascending,
                    base,
                } if !self.merge => {
                    if !apart_without_merging(first, second, ascending, base) {
                        return Err(unsupported(
                            span,
                            format!(
                                "a store adjacent to the previous statement's store, of two \
                                 constants ({first} and {second} bytes, {}, {}) that no unit \
                                 without merging shows apart (`tools/storesurvey.py units`; \
                                 codegen-patterns 13), is",
                                if ascending { "ascending" } else { "descending" },
                                base_words(base)
                            ),
                        ));
                    }
                }
                Pair::Consts {
                    first: 2,
                    second: 2,
                    ascending: true,
                    ..
                } => {
                    let j = prev.unwrap();
                    if run.last() != Some(&j) {
                        self.end_run(&stmts, &mut run, &mut merge_at, &mut drop)?;
                        run.push(j);
                    }
                    run.push(k);
                }
                Pair::Consts {
                    first,
                    second,
                    ascending,
                    base,
                } => {
                    let j = prev.unwrap();
                    let run_end = run.last() == Some(&j);
                    self.end_run(&stmts, &mut run, &mut merge_at, &mut drop)?;
                    if run_end && drop[j] {
                        // The previous word is merged into the one below
                        // it: this store is next to a longword, which is
                        // no pair (XNS_ERROR 0xE178D2 `move.l
                        // #0x30008,(-40,A6); clr.l (-36,A6)`, two words
                        // and a longword above them).
                        prev = Some(k);
                        continue;
                    }
                    if apart_when_merging(first, second, ascending, base) {
                        prev = Some(k);
                        continue;
                    }
                    let what = match (first, second) {
                        (1, 1) => format!(
                            "two byte constants, {}, {} (no such pair in a merging unit; \
                             bead domain-pascal-qb9q)",
                            if ascending { "ascending" } else { "descending" },
                            base_words(base)
                        ),
                        (2, 2) => format!(
                            "a word constant stored just below the previous statement's, {} \
                             (no such pair in a merging unit; bead domain-pascal-qb9q)",
                            base_words(base)
                        ),
                        _ => "a word and a longword constant, the word the lower (merged \
                              and split again at FIM_$BUILD_DF 0xE0A896, apart at 0xE14CBC)"
                            .to_string(),
                    };
                    return Err(unsupported(
                        span,
                        format!("under --merge-constant-stores, {what} is"),
                    ));
                }
            }
            prev = Some(k);
        }
        self.end_run(&stmts, &mut run, &mut merge_at, &mut drop)?;
        let mut out = Vec::with_capacity(stmts.len());
        for (k, st) in stmts.into_iter().enumerate() {
            if drop[k] {
                continue;
            }
            match (merge_at[k], &st.kind) {
                (Some(v), StmtKind::Store { addr, .. }) => {
                    merged.push((st.span, addr.clone()));
                    out.push(Stmt {
                        kind: StmtKind::Store {
                            addr: addr.clone(),
                            value: Expr::Const {
                                value: v,
                                ty: Scalar::signed(Width::Long),
                            },
                        },
                        span: st.span,
                    });
                }
                _ => out.push(st),
            }
        }
        Ok(out)
    }

    /// The pair the store of `value` at `addr` makes with the previous
    /// statement's store of `pv` at `pa`.
    fn pair(&self, pa: &Addr, pv: &Expr, addr: &Addr, value: &Expr) -> Pair {
        let (pb, bytes) = (pv.ty().bytes(), value.ty().bytes());
        let (Some((k1, o1)), Some((k2, o2))) = (split_addr(pa), split_addr(addr)) else {
            return Pair::No;
        };
        if k1 != k2 || !(o1 + pb as i32 == o2 || o2 + bytes as i32 == o1) {
            return Pair::No;
        }
        let consts = matches!((pv, value), (Expr::Const { .. }, Expr::Const { .. }));
        let one = matches!(pb + bytes, 2 | 4)
            && one_reference(&k1, o1.min(o2), pb + bytes)
            && mergeable_values(pv, value, o2 - o1, &self.held);
        // A word and, above it, a longword of constants: merged and split
        // again at the other boundary at FIM_$BUILD_DF 0xE0A896 (`clr.l
        // (0x54,A2); clr.w (0x58,A2)` for a word at 0x54 and a longword at
        // 0x56), apart at 0xE14CBC.  With the longword below, merged or
        // not they are the same two stores (apart in source order at
        // 0xE70B5E).
        let word_lower = if pb == 2 { o1 < o2 } else { o2 < o1 };
        let six = pb + bytes == 6 && word_lower && consts;
        if !(one || six) {
            return Pair::No;
        }
        if !consts {
            return Pair::Copy;
        }
        let Some(base) = Base::of(&k1) else {
            return Pair::No;
        };
        Pair::Consts {
            first: pb,
            second: bytes,
            ascending: o2 > o1,
            base,
        }
    }

    /// With the knob, the end of a run of ascending word constants
    /// (`run`, statement indices): the first two merged when the image
    /// settles it ([`Self::decide`]); a run of four or more is refused.
    fn end_run(
        &self,
        stmts: &[Stmt],
        run: &mut Vec<usize>,
        merge_at: &mut [Option<i64>],
        drop: &mut [bool],
    ) -> Res<()> {
        if run.is_empty() {
            return Ok(());
        }
        let r = std::mem::take(run);
        let (s1, s2) = (&stmts[r[0]], &stmts[r[1]]);
        let decision = self.decide(s1, s2)?;
        if r.len() >= 4 {
            // TTY_$K_RESET 0xE67322 stores 0x2CA..0x2D0, four words in
            // ../source's order, as two longwords (greedy from the first);
            // KBD_$INIT 0xE33378 stores 0x38..0x3E as `clr.w (0x38,A2);
            // clr.l (0x3a,A2); clr.w (0x3e,A2)` (the original's order is
            // not known).
            return Err(unsupported(
                stmts[r[3]].span,
                "under --merge-constant-stores, a fourth word constant stored above three \
                 (runs of four words pair from the first at TTY_$K_RESET 0xE67322, not at \
                 KBD_$INIT 0xE33378; not settled, bead domain-pascal-qb9q) is",
            ));
        }
        match decision {
            Decision::Merge => {
                let (Some(v1), Some(v2)) = (const_of(s1), const_of(s2)) else {
                    unreachable!("a run of constants")
                };
                let bits = ((v1 as u16 as u32) << 16) | v2 as u16 as u32;
                merge_at[r[0]] = Some(bits as i32 as i64);
                drop[r[1]] = true;
            }
            Decision::Apart if r.len() == 3 => {
                return Err(unsupported(
                    stmts[r[2]].span,
                    "under --merge-constant-stores, a third word constant after two kept \
                     apart (not seen) is",
                ));
            }
            Decision::Apart => {}
        }
        Ok(())
    }

    /// Two words of constants, the second above the first, in a unit
    /// that merges (`tools/storesurvey.py units`, "in merging units"):
    ///
    /// - through a pointer: merged, at either alignment (25 merged
    ///   constant sites through an address register, the low address 0
    ///   mod 4 at 10, 2 mod 4 at 15: TTY_$K_RESET 0xE67322 (A2, the
    ///   pointer TTY_$I_GET_DESC returns), the module-local 0xE1BF70 at
    ///   0xE1BFE4 (A2 = (8,A6)), LOG_$ADD 0xE176B0, PROC1_$BIND 0xE14DA6);
    ///   no pair of two words through a register is apart in a merging
    ///   unit.  A base with a call in it is refused (not seen);
    /// - in another data section's block (`movea.l #`): merged, KBD_$INIT
    ///   0xE33374 `clr.l (0x58,A0)`;
    /// - in the frame (Uncertain): two word locals apart, at 0 mod 4 (PROC2_$INIT
    ///   0xE30412 `move.w #16,(-56,A6); move.w #16,(-54,A6)`, min_pri and
    ///   max_pri in the C, passed apart by address) and at 2 mod 4
    ///   (ASKNODE_$SERVER 0xE65A3E `clr.w (-0x296,A6); move.w
    ///   #0x200,(-0x294,A6)`, two locals in the C); two words of a record
    ///   or array local merged at 0 mod 4 (APP 0xE00BA8, RIP_ 0xE15ABA,
    ///   XNS_ERROR 0xE178D2 `move.l #..,(-40,A6)`, opt.socket and
    ///   opt.flags; ASKNODE_$PROPAGATE_WHO 0xE65EBC, 0xE65ED0).  The five
    ///   merges are two idioms (xns_$os_open_opt_t three times,
    ///   ASKNODE_$PROPAGATE_WHO twice); the word locals apart include, in
    ///   units not known to merge, PROC2_$LIST2 0xE4042A, PROC2_$ZOMBIE_LIST
    ///   0xE40570, DISK_$PV_MOUNT 0xE6C9FA, VFMT 0xE6AB44, RING 0xE7717A
    ///   and 0xE775AE; "scalar" is ../source's reconstruction, and another
    ///   reading fits as well: words whose addresses are taken one by one
    ///   (PROC2_$INIT pushes &min_pri, &max_pri).  A record's words at 2
    ///   mod 4, and a local next to another kind of frame object, are not
    ///   seen: refused;
    /// - off A5: apart (SMD 0xE6D824 `clr.w (204,A5); clr.w (206,A5)`,
    ///   0xE6D830, in the routine that merges through a pointer at
    ///   0xE6D80E; no merge off A5 anywhere);
    /// - absolute, in the enclosing frame, or volatile: no pair in a
    ///   merging unit: refused.
    ///
    /// A record or array local next to another is not told apart from one
    /// local here (the IR does not give the frame's objects): two such
    /// locals' adjacent words at 0 mod 4 are merged as one local's are.
    fn decide(&self, s1: &Stmt, s2: &Stmt) -> Res<Decision> {
        let (a1, _) = self.mem_store(s1).unwrap();
        let (a2, _) = self.mem_store(s2).unwrap();
        let refuse = |what: &str| {
            Err(unsupported(
                s2.span,
                format!(
                    "under --merge-constant-stores, two word constants stored to adjacent \
                     words {what} (no such pair in a merging unit, codegen-patterns 13; bead \
                     domain-pascal-qb9q) is"
                ),
            ))
        };
        if a1.unwrap_volatile().1.is_some() || a2.unwrap_volatile().1.is_some() {
            return refuse("of a volatile object");
        }
        let (Some((base, lo)), Some((_, hi))) = (split_addr(a1), split_addr(a2)) else {
            unreachable!("a pair has a base")
        };
        match Base::of(&base) {
            Some(Base::Pointer) => match &base {
                Addr::Indirect { base: p, .. } if expr_has_call(p) => {
                    refuse("through a pointer a call computes")
                }
                _ => Ok(Decision::Merge),
            },
            Some(Base::OtherStatic) => Ok(Decision::Merge),
            Some(Base::Frame) => {
                let scalar = |o: i32| {
                    self.scalars
                        .iter()
                        .any(|&(s, n)| s <= o && o < s + n as i32)
                };
                let local = |o: i32| self.f.scalar_locals.iter().any(|l| l.offset == o);
                if local(lo) && local(hi) {
                    Ok(Decision::Apart)
                } else if scalar(lo) || scalar(hi) {
                    refuse("in the frame, a word local next to another kind of frame object")
                } else if lo.rem_euclid(4) == 0 {
                    Ok(Decision::Merge)
                } else {
                    refuse("of a record or array local in the frame at 2 mod 4")
                }
            }
            Some(Base::Static) => Ok(Decision::Apart),
            Some(Base::Symbol) => refuse("of another unit's object, ascending"),
            Some(Base::Enclosing) => refuse("of the enclosing routine's frame"),
            None => unreachable!("a pair has a base"),
        }
    }
}

/// The constant a store statement stores.
fn const_of(st: &Stmt) -> Option<i64> {
    match &st.kind {
        StmtKind::Store {
            value: Expr::Const { value, .. },
            ..
        } => Some(*value),
        _ => None,
    }
}

/// The base kind in words, for a diagnostic.
fn base_words(base: Base) -> &'static str {
    match base {
        Base::Frame => "in the frame",
        Base::Static => "of the unit's data",
        Base::Symbol => "of another unit's object",
        Base::Pointer => "through a pointer",
        Base::OtherStatic => "of another data section's block",
        Base::Enclosing => "of the enclosing routine's frame",
    }
}

/// Whether `bytes` bytes at offset `lo` from the base `base` (a base from
/// [`split_addr`]) can be one 68000 reference: not a word at an odd
/// address.  The frame and the module's static data are even (A6 and A5
/// hold even addresses); the parity of a pointer's or a symbol's target
/// is not known here, so a pair through one stays a candidate.
fn one_reference(base: &Addr, lo: i32, bytes: u32) -> bool {
    let even_base = matches!(base, Addr::Frame(_) | Addr::Enclosing(_) | Addr::Static(_));
    !(bytes > 1 && even_base && lo % 2 != 0)
}

/// Whether the values of two stores to adjacent objects could be stored
/// by one reference (ref 3.15.3 p. 3-57): both constants, or both loads,
/// at the stores' widths, of memory of one base whose offsets differ by
/// `delta`, the difference of the destinations' offsets (a copy of
/// adjacent memory).  A computed value, a register-held local or a load
/// from elsewhere is not (codegen-patterns 13, "Store merging": 270 such
/// pairs in the image, none merged).
fn mergeable_values(first: &Expr, second: &Expr, delta: i32, held: &[i32]) -> bool {
    match (first, second) {
        (Expr::Const { .. }, Expr::Const { .. }) => true,
        (Expr::Load { addr: a1, .. }, Expr::Load { addr: a2, .. }) => {
            let held_local = |a: &Addr| matches!(a, Addr::Frame(o) if held.contains(o));
            if held_local(a1) || held_local(a2) {
                return false;
            }
            match (split_addr(a1), split_addr(a2)) {
                (Some((b1, s1)), Some((b2, s2))) => b1 == b2 && s2 - s1 == delta,
                // A load whose address is not a base and an offset (an
                // element at a run-time index): adjacency is not known.
                _ => true,
            }
        }
        _ => false,
    }
}
