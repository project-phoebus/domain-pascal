//! Constants substituted for reaching definitions (ref 6.4.24, p. 6-28,
//! `-opt 2`: "If the definition is an assignment of a constant to the
//! variable, then the compiler can replace uses of the variable that the
//! definition reaches with the constant"), the parts of an `if` a constant
//! condition never runs removed (`-opt 1`, p. 6-27: "Eliminates dead
//! code"), and the assignments whose value nothing reads any more removed
//! (`-opt 3`, p. 6-28: "Redundant assignment statement elimination").
//!
//! Evidence (codegen-patterns 7.1, "Constants for reaching definitions";
//! bead domain-pascal-pvy8; ../source pascal-translation-notes finding
//! 607): SAU3 PBU_$SETUP, the SAU5's text with its VME arm configured out,
//! so that `bus := io_$bus_mbus` and `first := 0; last := 7` are the only
//! definitions reaching the rest:
//! - `if bus = io_$bus_vme` and `if bus = io_$bus_mbus` are gone, the
//!   Multibus checks unconditional (0xE595AA..0xE595D2), as is the VME
//!   block after the loop (before 0xE598CC);
//! - `req.tag := 3; req.bus := bus` is one `move.l #0x30002,(-8,A6)`
//!   (0xE596E4: the substituted constant merged with its neighbour);
//! - `case bus of` selects on `moveq #2,D0` (0xE596F0), its table kept;
//! - `for sub := first to last` counts `move.w #7,(-0x6a,A6)` from
//!   `clr.w D4` (0xE595D8, 0xE595E0), and no store of `first` or `last`
//!   is left;
//! - the packed field store `dcte^.unit_info.bus := bus` still reads the
//!   slot (`move.b (-0x55,A6),D1`, 0xE597D4), so `bus := 2` stays
//!   (`move.w #2,(-0x56,A6)`, 0xE5957C).
//!
//! PBU_$SETDOWN likewise (`clr.w D3; moveq #7,D2`, 0xE59382).
//!
//! A per-unit choice, as store merging (an `-opt 1` feature) is: the
//! kernel's units were not all compiled alike (`dpc`
//! `--substitute-constants`).
//!
//! What is substituted: the routine's own scalar locals of 1, 2 or 4 bytes
//! ([`Function::scalar_locals`]: not those a nested routine reaches),
//! neither pointers nor `for` indices, read and written only as wholes
//! (never by address, `addr`, or as part of a larger object).  A use is
//! substituted when every definition reaching it stores the same
//! constant.  Not substituted: the value of a packed field store (the
//! PBU_$SETUP site above, Uncertain, one site) and a read of part of the
//! local.  A constant condition of an `if` keeps the part it runs; a
//! `while` whose condition is false at entry goes; `case` keeps its arms
//! (the table above).  A local no read is left of loses its stores whose
//! value calls nothing.  Not modelled (no site): a store dead on some
//! paths only; the warning the reference shows (p. 6-28, "Warning 279").

use std::collections::{HashMap, HashSet};

use crate::{
    Addr, Arg, BinOp, Call, Callee, Cond, Expr, Function, Rel, Scalar, Sign, Stmt, StmtKind, UnOp,
};

/// What is known of each candidate local: absent = not a constant.
type State = HashMap<i32, i64>;

/// Substitute constants in `f` (see the module documentation).
/// `abnormal`: the symbols of the routines declared `abnormal` (ref
/// 5.5.6, p. 5-19: the option "causes the compiler to be careful about
/// optimizing around any cleanup handler"): a routine calling one is left
/// alone.  SAU2 PKT_$COPY_TO_PA 0xE1251C reads `buf_count` from its slot
/// in the handler after FIM_$CLEANUP's second return (0xE1260C), where
/// the only definition the code before the call shows is `clr.w
/// (-40,A6)` (0xE12534).
pub fn substitute_constants(f: &mut Function, abnormal: &HashSet<String>) {
    if calls_any(&f.body, abnormal) {
        return;
    }
    let locals = candidates(f);
    if locals.is_empty() {
        return;
    }
    let mut cx = Cx {
        locals,
        labels: HashMap::new(),
        loops: Vec::new(),
    };
    // The labels' states: meet of every `goto` reaching them, accumulated
    // over passes on copies until nothing changes.
    loop {
        let before = cx.labels.clone();
        let mut body = f.body.clone();
        cx.flow(&mut body, Some(State::new()));
        if cx.labels == before {
            break;
        }
    }
    cx.flow(&mut f.body, Some(State::new()));
    drop_dead_stores(&mut f.body, &cx.locals);
}

/// Whether `body` calls one of `names`.
fn calls_any(body: &[Stmt], names: &HashSet<String>) -> bool {
    fn callee(c: &Call, names: &HashSet<String>) -> bool {
        matches!(&c.callee, Callee::External(n) | Callee::Local(n) | Callee::Section(n)
            if names.contains(n))
            || c.args.iter().any(|a| match a {
                Arg::Call { call, .. } => callee(call, names),
                Arg::Value { value, .. } | Arg::Temp { value, .. } => expr(value, names),
                Arg::Address { .. } => false,
            })
    }
    fn expr(e: &Expr, names: &HashSet<String>) -> bool {
        let mut found = false;
        walk_expr(e, &mut |x| {
            if let Expr::Call { call, .. } = x {
                found |= callee(call, names);
            }
        });
        found
    }
    if names.is_empty() {
        return false;
    }
    let mut found = false;
    crate::visit_stmts(body, &mut |s| {
        if let StmtKind::Call(c) = &s.kind {
            found |= callee(c, names);
        }
    });
    // Function calls in expressions: those of the statements' values and
    // conditions.
    let mut values = Vec::new();
    crate::visit_stmts(body, &mut |s| match &s.kind {
        StmtKind::SetResult(e) | StmtKind::Discard(e) => values.push(e.clone()),
        StmtKind::Store { value, .. } | StmtKind::StoreBits { value, .. } => {
            values.push(value.clone())
        }
        StmtKind::If { cond, .. } | StmtKind::While { cond, .. } => {
            values.push(Expr::Test(Box::new(cond.clone())))
        }
        StmtKind::Repeat { until: Some(c), .. } => values.push(Expr::Test(Box::new(c.clone()))),
        StmtKind::Case { selector, .. } => values.push(selector.clone()),
        StmtKind::For(l) => {
            values.push(l.from.clone());
            values.push(l.to.clone());
        }
        _ => {}
    });
    found
        || values
            .iter()
            .any(|e| expr(e, names) || cond_calls(e, names))
}

/// Calls inside the conditions of an `Expr::Test` (which `walk_expr`
/// does not enter).
fn cond_calls(e: &Expr, names: &HashSet<String>) -> bool {
    fn c(cond: &Cond, names: &HashSet<String>) -> bool {
        let e = |x: &Expr| {
            let mut found = false;
            walk_expr(x, &mut |y| match y {
                Expr::Call { call, .. } => {
                    found |= matches!(&call.callee,
                        Callee::External(n) | Callee::Local(n) | Callee::Section(n)
                        if names.contains(n))
                }
                Expr::Test(t) => found |= c(t, names),
                _ => {}
            });
            found
        };
        match cond {
            Cond::Compare { lhs, rhs, .. } => e(lhs) || e(rhs),
            Cond::Test(x) | Cond::Odd(x) => e(x),
            Cond::Not(a) => c(a, names),
            Cond::AndThen(a, b) | Cond::OrElse(a, b) | Cond::And(a, b) | Cond::Or(a, b) => {
                c(a, names) || c(b, names)
            }
            Cond::Block { .. } | Cond::SetEmpty { .. } => false,
        }
    }
    match e {
        Expr::Test(t) => c(t, names),
        _ => false,
    }
}

/// The candidate locals: offset and width in bytes.
fn candidates(f: &Function) -> HashMap<i32, u32> {
    let mut locals: HashMap<i32, u32> = f
        .scalar_locals
        .iter()
        .filter(|l| !l.ty.pointer)
        .map(|l| (l.offset, l.ty.width.bytes()))
        .collect();
    let mut bad = HashSet::new();
    let mut scan = Scan {
        locals: &locals,
        bad: &mut bad,
    };
    scan.stmts(&f.body);
    for o in bad {
        locals.remove(&o);
    }
    locals
}

/// Finds the candidates referred to other than as a whole read or a
/// whole store.
struct Scan<'a> {
    locals: &'a HashMap<i32, u32>,
    bad: &'a mut HashSet<i32>,
}

impl Scan<'_> {
    /// The candidate whose bytes hold frame offset `x`.
    fn owner(&self, x: i32) -> Option<i32> {
        self.locals
            .iter()
            .find(|(&o, &w)| x >= o && x < o + w as i32)
            .map(|(&o, _)| o)
    }

    fn mark(&mut self, a: &Addr) {
        match a {
            Addr::Frame(x) => {
                if let Some(o) = self.owner(*x) {
                    self.bad.insert(o);
                }
            }
            Addr::Index { base, index, .. } => {
                self.mark(base);
                self.expr(index);
            }
            Addr::Volatile { addr, .. } => self.mark(addr),
            Addr::Indirect { base, .. } => self.expr(base),
            _ => {}
        }
    }

    /// An address only read through (a load's): a part of a candidate is
    /// a use, not a disqualification.
    fn read(&mut self, a: &Addr) {
        match a {
            Addr::Frame(_) => {}
            _ => self.mark(a),
        }
    }

    fn expr(&mut self, e: &Expr) {
        match e {
            Expr::Const { .. } => {}
            Expr::Load { addr, .. } => self.read(addr),
            Expr::Bits { addr, .. } => self.mark(addr),
            Expr::BitElem { array, index, .. } => {
                self.mark(array);
                self.expr(index);
            }
            Expr::SetMember { elem, set, .. } => {
                self.expr(elem);
                self.expr(set);
            }
            Expr::SetMemberAt { elem, set, .. } => {
                self.expr(elem);
                self.mark(set);
            }
            Expr::SetOp { lhs, rhs, .. } | Expr::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            Expr::SetOf { elem, .. } => self.expr(elem),
            Expr::AddrOf(a) => self.mark(a),
            Expr::Unary { operand: x, .. } | Expr::Convert { value: x, .. } | Expr::SetSr(x) => {
                self.expr(x)
            }
            Expr::Call { call, .. } => self.call(call),
            Expr::Test(c) => self.cond(c),
        }
    }

    fn cond(&mut self, c: &Cond) {
        match c {
            Cond::Compare { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            Cond::Test(e) | Cond::Odd(e) => self.expr(e),
            Cond::Not(c) => self.cond(c),
            Cond::AndThen(a, b) | Cond::OrElse(a, b) | Cond::And(a, b) | Cond::Or(a, b) => {
                self.cond(a);
                self.cond(b);
            }
            Cond::Block { lhs, rhs, .. } => {
                self.mark(lhs);
                self.mark(rhs);
            }
            Cond::SetEmpty { set, .. } => self.mark(set),
        }
    }

    fn call(&mut self, c: &Call) {
        if let Callee::Pointer(x) = &c.callee {
            self.expr(x);
        }
        for a in &c.args {
            match a {
                Arg::Value { value, .. } | Arg::Temp { value, .. } => self.expr(value),
                Arg::Address { addr, .. } => self.mark(addr),
                Arg::Call { call, .. } => self.call(call),
            }
        }
    }

    fn stmts(&mut self, body: &[Stmt]) {
        for s in body {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::SetResult(e) | StmtKind::Discard(e) => self.expr(e),
            StmtKind::Store { addr, value } => {
                match addr {
                    // A whole store: the same width as the candidate.
                    Addr::Frame(x) => {
                        if let Some(o) = self.owner(*x) {
                            if *x != o || value.ty().width.bytes() != self.locals[&o] {
                                self.bad.insert(o);
                            }
                        }
                    }
                    a => self.mark(a),
                }
                self.expr(value);
            }
            StmtKind::StoreBits { addr, value, .. } => {
                self.mark(addr);
                self.expr(value);
            }
            StmtKind::StoreBitElem { array, index, .. } => {
                self.mark(array);
                self.expr(index);
            }
            StmtKind::StoreSetMember { set, elem, .. } => {
                self.mark(set);
                self.expr(elem);
            }
            StmtKind::StoreSetConst { set, .. } => self.mark(set),
            StmtKind::StoreSetRange { set, lo, hi, .. } => {
                self.mark(set);
                self.expr(hi);
                self.expr(lo);
            }
            StmtKind::Copy { dst, src, .. } => {
                self.mark(dst);
                self.mark(src);
            }
            StmtKind::Call(c) => self.call(c),
            StmtKind::Fill { start, value, .. } => {
                self.mark(start);
                self.expr(value);
            }
            StmtKind::If { cond, then_, else_ } => {
                self.cond(cond);
                self.stmts(then_);
                self.stmts(else_);
            }
            StmtKind::Repeat { body, until } => {
                self.stmts(body);
                if let Some(c) = until {
                    self.cond(c);
                }
            }
            StmtKind::While { cond, body } => {
                self.cond(cond);
                self.stmts(body);
            }
            StmtKind::For(l) => {
                if let Some(o) = self.owner(l.index.offset) {
                    self.bad.insert(o);
                }
                if let Some(h) = &l.home {
                    self.mark(h);
                }
                self.expr(&l.from);
                self.expr(&l.to);
                self.stmts(&l.body);
            }
            StmtKind::Case {
                selector,
                arms,
                otherwise,
            } => {
                self.expr(selector);
                for a in arms {
                    self.stmts(&a.body);
                }
                if let Some(o) = otherwise {
                    self.stmts(o);
                }
            }
            StmtKind::Exit
            | StmtKind::Next
            | StmtKind::Return
            | StmtKind::Disable
            | StmtKind::Enable
            | StmtKind::Label(_)
            | StmtKind::Goto(_) => {}
        }
    }
}

/// The meet of two states (`None`: not reached).
fn meet(a: Option<State>, b: Option<State>) -> Option<State> {
    match (a, b) {
        (None, x) | (x, None) => x,
        (Some(a), Some(b)) => Some(a.into_iter().filter(|(k, v)| b.get(k) == Some(v)).collect()),
    }
}

/// Where `exit` and `next` of the innermost loop go.
#[derive(Default)]
struct LoopExits {
    exits: Option<State>,
    nexts: Option<State>,
}

struct Cx {
    locals: HashMap<i32, u32>,
    labels: HashMap<u32, Option<State>>,
    loops: Vec<LoopExits>,
}

impl Cx {
    /// Rewrite `body` from state `cur`; the state after it.
    fn flow(&mut self, body: &mut Vec<Stmt>, mut cur: Option<State>) -> Option<State> {
        let old = std::mem::take(body);
        for mut s in old {
            let Some(st) = cur.clone() else {
                // Not reached but by a label: a label joins its gotos.
                if let StmtKind::Label(l) = s.kind {
                    cur = self.labels.get(&l).cloned().flatten();
                }
                body.push(s);
                continue;
            };
            match &mut s.kind {
                StmtKind::If { cond, then_, else_ } => {
                    subst_cond(cond, &st, &self.locals);
                    match known(cond) {
                        Some(taken) if !has_label(then_) && !has_label(else_) => {
                            let mut part = std::mem::take(if taken { then_ } else { else_ });
                            cur = self.flow(&mut part, cur);
                            body.extend(part);
                            continue;
                        }
                        _ => {
                            let a = self.flow(then_, Some(st.clone()));
                            let b = self.flow(else_, Some(st));
                            cur = meet(a, b);
                        }
                    }
                }
                StmtKind::While { cond, body: inner } => {
                    let mut c = cond.clone();
                    subst_cond(&mut c, &st, &self.locals);
                    if known(&c) == Some(false) && !has_label(inner) {
                        continue;
                    }
                    let head = self.loop_head(Some(st), |cx, h| {
                        let mut c = cond.clone();
                        let mut b = inner.clone();
                        cx.while_pass(&mut c, &mut b, h)
                    });
                    let mut le = self.while_pass(cond, inner, head.clone());
                    let exit = if known(cond) == Some(true) {
                        None
                    } else {
                        head
                    };
                    cur = meet(exit, le.exits.take());
                }
                StmtKind::Repeat { body: inner, until } => {
                    let head = self.loop_head(Some(st), |cx, h| {
                        let mut b = inner.clone();
                        let mut u = until.clone();
                        cx.repeat_pass(&mut b, &mut u, h)
                    });
                    let (mut le, end) = self.repeat_pass_end(inner, until, head);
                    let exit = match until.as_ref().map(known) {
                        Some(Some(false)) | None => None,
                        _ => end,
                    };
                    cur = meet(exit, le.exits.take());
                }
                StmtKind::For(l) => {
                    subst_expr(&mut l.from, &st, &self.locals);
                    subst_expr(&mut l.to, &st, &self.locals);
                    if let (Expr::Const { value: a, .. }, Expr::Const { value: b, .. }) =
                        (&l.from, &l.to)
                    {
                        // A loop that never runs: dead code (`-opt 1`).
                        if (if l.down { a < b } else { a > b }) && !has_label(&l.body) {
                            continue;
                        }
                    }
                    let entry = Some(st);
                    let head = self.loop_head(entry.clone(), |cx, h| {
                        let mut b = l.body.clone();
                        cx.body_pass(&mut b, h)
                    });
                    let mut le = self.body_pass(&mut l.body, head.clone());
                    cur = meet(head, le.exits.take());
                }
                StmtKind::Case {
                    selector,
                    arms,
                    otherwise,
                } => {
                    subst_expr(selector, &st, &self.locals);
                    let mut out = None;
                    for a in arms.iter_mut() {
                        let o = self.flow(&mut a.body, Some(st.clone()));
                        out = meet(out, o);
                    }
                    match otherwise {
                        Some(o) => {
                            let o = self.flow(o, Some(st));
                            out = meet(out, o);
                        }
                        None => out = meet(out, Some(st)),
                    }
                    cur = out;
                }
                StmtKind::Exit => {
                    if let Some(le) = self.loops.last_mut() {
                        le.exits = meet(le.exits.take(), Some(st));
                    }
                    cur = None;
                }
                StmtKind::Next => {
                    if let Some(le) = self.loops.last_mut() {
                        le.nexts = meet(le.nexts.take(), Some(st));
                    }
                    cur = None;
                }
                StmtKind::Return => cur = None,
                StmtKind::Goto(l) => {
                    let e = self.labels.entry(*l).or_insert(None);
                    *e = meet(e.take(), Some(st));
                    cur = None;
                }
                StmtKind::Label(l) => {
                    cur = meet(Some(st), self.labels.get(l).cloned().flatten());
                }
                StmtKind::Store { addr, value } => {
                    subst_expr(value, &st, &self.locals);
                    subst_addr(addr, &st, &self.locals);
                    let mut st = st;
                    if let Addr::Frame(o) = addr {
                        if self.locals.contains_key(o) {
                            match value {
                                Expr::Const { value: v, .. } => {
                                    st.insert(*o, *v);
                                }
                                _ => {
                                    st.remove(o);
                                }
                            }
                        }
                    }
                    cur = Some(st);
                }
                k => {
                    subst_simple(k, &st, &self.locals);
                }
            }
            body.push(s);
        }
        cur
    }

    /// The loop head's state: the meet of the entry and the back edges,
    /// `pass` giving the back edge from a head state.
    fn loop_head(
        &mut self,
        entry: Option<State>,
        mut pass: impl FnMut(&mut Cx, Option<State>) -> LoopExits,
    ) -> Option<State> {
        let mut head = entry.clone();
        loop {
            let le = pass(self, head.clone());
            let next = meet(entry.clone(), le.nexts);
            if next == head {
                return head;
            }
            head = next;
        }
    }

    /// One pass over a `while` loop from head state `h`: its exits, and in
    /// `nexts` its back edges.
    fn while_pass(&mut self, cond: &mut Cond, body: &mut Vec<Stmt>, h: Option<State>) -> LoopExits {
        let Some(st) = h else {
            return LoopExits::default();
        };
        subst_cond(cond, &st, &self.locals);
        if known(cond) == Some(false) {
            return LoopExits::default();
        }
        self.body_pass(body, Some(st))
    }

    /// One pass over a loop body from `h`: exits, and back edges (its end
    /// and its `next`s) in `nexts`.
    fn body_pass(&mut self, body: &mut Vec<Stmt>, h: Option<State>) -> LoopExits {
        self.loops.push(LoopExits::default());
        let end = self.flow(body, h);
        let mut le = self.loops.pop().unwrap_or_default();
        le.nexts = meet(le.nexts.take(), end);
        le
    }

    fn repeat_pass(
        &mut self,
        body: &mut Vec<Stmt>,
        until: &mut Option<Cond>,
        h: Option<State>,
    ) -> LoopExits {
        let (mut le, end) = self.repeat_pass_end(body, until, h);
        // The back edge: from the test, unless the condition is true.
        le.nexts = match until.as_ref().map(known) {
            Some(Some(true)) => None,
            _ => end,
        };
        le
    }

    /// A `repeat` body from `h`, then its condition: the exits, and the
    /// state at the test (the end and the `next`s, which go to the test).
    fn repeat_pass_end(
        &mut self,
        body: &mut Vec<Stmt>,
        until: &mut Option<Cond>,
        h: Option<State>,
    ) -> (LoopExits, Option<State>) {
        self.loops.push(LoopExits::default());
        let end = self.flow(body, h);
        let mut le = self.loops.pop().unwrap_or_default();
        let end = meet(end, le.nexts.take());
        if let (Some(c), Some(st)) = (until.as_mut(), end.as_ref()) {
            subst_cond(c, st, &self.locals);
        }
        (le, end)
    }
}

/// A label anywhere in `body` (a part holding one is kept: a `goto` may
/// reach it).
fn has_label(body: &[Stmt]) -> bool {
    let mut found = false;
    crate::visit_stmts(body, &mut |s| {
        if matches!(s.kind, StmtKind::Label(_)) {
            found = true;
        }
    });
    found
}

/// Substitute in the expressions of a statement that neither defines a
/// candidate nor branches.  A packed field store's value is left alone
/// (PBU_$SETUP 0xE597D4).
fn subst_simple(k: &mut StmtKind, st: &State, l: &HashMap<i32, u32>) {
    match k {
        StmtKind::SetResult(e) | StmtKind::Discard(e) => subst_expr(e, st, l),
        StmtKind::StoreBits { addr, .. } => subst_addr(addr, st, l),
        StmtKind::StoreBitElem { array, index, .. } => {
            subst_addr(array, st, l);
            subst_expr(index, st, l);
        }
        StmtKind::StoreSetMember { set, elem, .. } => {
            subst_addr(set, st, l);
            subst_expr(elem, st, l);
        }
        StmtKind::StoreSetConst { set, .. } => subst_addr(set, st, l),
        StmtKind::StoreSetRange { set, lo, hi, .. } => {
            subst_addr(set, st, l);
            subst_expr(hi, st, l);
            subst_expr(lo, st, l);
        }
        StmtKind::Copy { dst, src, .. } => {
            subst_addr(dst, st, l);
            subst_addr(src, st, l);
        }
        StmtKind::Call(c) => subst_call(c, st, l),
        StmtKind::Fill { start, value, .. } => {
            subst_addr(start, st, l);
            subst_expr(value, st, l);
        }
        _ => {}
    }
}

fn subst_call(c: &mut Call, st: &State, l: &HashMap<i32, u32>) {
    if let Callee::Pointer(x) = &mut c.callee {
        subst_expr(x, st, l);
    }
    for a in &mut c.args {
        match a {
            Arg::Value { value, .. } | Arg::Temp { value, .. } => subst_expr(value, st, l),
            Arg::Address { addr, .. } => subst_addr(addr, st, l),
            Arg::Call { call, .. } => subst_call(call, st, l),
        }
    }
}

fn subst_addr(a: &mut Addr, st: &State, l: &HashMap<i32, u32>) {
    match a {
        Addr::Indirect { base, .. } => subst_expr(base, st, l),
        Addr::Index { base, index, .. } => {
            subst_addr(base, st, l);
            subst_expr(index, st, l);
        }
        Addr::Volatile { addr, .. } => subst_addr(addr, st, l),
        _ => {}
    }
}

fn subst_cond(c: &mut Cond, st: &State, l: &HashMap<i32, u32>) {
    match c {
        Cond::Compare { op, lhs, rhs } => {
            subst_expr(lhs, st, l);
            subst_expr(rhs, st, l);
            // A constant come to the left goes to the right, the
            // relation mirrored: `len := 0; if len < n` as `n > 0`.
            if matches!(*lhs, Expr::Const { .. }) && !matches!(*rhs, Expr::Const { .. }) {
                std::mem::swap(lhs, rhs);
                *op = match *op {
                    Rel::Lt => Rel::Gt,
                    Rel::Gt => Rel::Lt,
                    Rel::Le => Rel::Ge,
                    Rel::Ge => Rel::Le,
                    o => o,
                };
            }
        }
        Cond::Test(e) | Cond::Odd(e) => subst_expr(e, st, l),
        Cond::Not(c) => subst_cond(c, st, l),
        Cond::AndThen(a, b) | Cond::OrElse(a, b) | Cond::And(a, b) | Cond::Or(a, b) => {
            subst_cond(a, st, l);
            subst_cond(b, st, l);
        }
        Cond::Block { lhs, rhs, .. } => {
            subst_addr(lhs, st, l);
            subst_addr(rhs, st, l);
        }
        Cond::SetEmpty { set, .. } => subst_addr(set, st, l),
    }
}

/// Substitute the known candidates read whole in `e`, then fold.
fn subst_expr(e: &mut Expr, st: &State, l: &HashMap<i32, u32>) {
    match e {
        Expr::Load {
            addr: Addr::Frame(o),
            ty,
        } => {
            if let (Some(v), Some(&w)) = (st.get(o), l.get(o)) {
                if ty.width.bytes() == w {
                    *e = Expr::Const {
                        value: wrap(*v, *ty),
                        ty: *ty,
                    };
                }
            }
            return;
        }
        Expr::Load { addr, .. } | Expr::Bits { addr, .. } | Expr::AddrOf(addr) => {
            subst_addr(addr, st, l)
        }
        Expr::Const { .. } => {}
        Expr::BitElem { array, index, .. } => {
            subst_addr(array, st, l);
            subst_expr(index, st, l);
        }
        Expr::SetMember { elem, set, .. } => {
            subst_expr(elem, st, l);
            subst_expr(set, st, l);
        }
        Expr::SetMemberAt { elem, set, .. } => {
            subst_expr(elem, st, l);
            subst_addr(set, st, l);
        }
        Expr::SetOp { lhs, rhs, .. } | Expr::Binary { lhs, rhs, .. } => {
            subst_expr(lhs, st, l);
            subst_expr(rhs, st, l);
        }
        Expr::SetOf { elem, .. } => subst_expr(elem, st, l),
        Expr::Unary { operand: x, .. } | Expr::Convert { value: x, .. } | Expr::SetSr(x) => {
            subst_expr(x, st, l)
        }
        Expr::Call { call, .. } => subst_call(call, st, l),
        Expr::Test(c) => subst_cond(c, st, l),
    }
    fold(e);
}

/// `v` as a number of type `ty` (its bits at `ty`'s width).
fn wrap(v: i64, ty: Scalar) -> i64 {
    let bits = ty.width.bytes() * 8;
    let m = v & ((1i64 << bits) - 1);
    match ty.sign {
        Sign::Unsigned => m,
        Sign::Signed if m >= 1i64 << (bits - 1) => m - (1i64 << bits),
        Sign::Signed => m,
    }
}

/// Fold an operator whose operands are constants.  `div` and `mod` are
/// left alone (their signs follow M$ARITH, codegen-patterns 8).
fn fold(e: &mut Expr) {
    let c = |x: &Expr| match x {
        Expr::Const { value, .. } => Some(*value),
        _ => None,
    };
    let v = match e {
        Expr::Binary { op, lhs, rhs, ty } => {
            let (Some(a), Some(b)) = (c(lhs), c(rhs)) else {
                return;
            };
            let bits = ty.width.bytes() * 8;
            let ua = a & ((1i64 << bits) - 1);
            Some((
                match op {
                    BinOp::Add => a.wrapping_add(b),
                    BinOp::Sub => a.wrapping_sub(b),
                    BinOp::Mul => a.wrapping_mul(b),
                    BinOp::And => a & b,
                    BinOp::Or => a | b,
                    BinOp::Xor => a ^ b,
                    BinOp::Shl if (0..64).contains(&b) => a << b,
                    BinOp::Shr if (0..64).contains(&b) => ua >> b,
                    BinOp::Ashr if (0..64).contains(&b) => a >> b,
                    BinOp::Min => a.min(b),
                    BinOp::Max => a.max(b),
                    _ => return,
                },
                *ty,
            ))
        }
        Expr::Unary { op, operand, ty } => {
            let Some(a) = c(operand) else { return };
            Some((
                match op {
                    UnOp::Neg => a.wrapping_neg(),
                    UnOp::Not => !a,
                    UnOp::Abs => a.wrapping_abs(),
                },
                *ty,
            ))
        }
        Expr::Convert { value, ty, .. } => c(value).map(|a| (a, *ty)),
        Expr::Test(cond) => known(cond).map(|b| (if b { -1 } else { 0 }, Scalar::BOOLEAN)),
        _ => None,
    };
    if let Some((v, ty)) = v {
        *e = Expr::Const {
            value: wrap(v, ty),
            ty,
        };
    }
}

/// A condition's value when its operands are constants.
fn known(c: &Cond) -> Option<bool> {
    let k = |x: &Expr| match x {
        Expr::Const { value, .. } => Some(*value),
        _ => None,
    };
    match c {
        Cond::Compare { op, lhs, rhs } => {
            let (a, b) = (k(lhs)?, k(rhs)?);
            Some(match op {
                Rel::Eq => a == b,
                Rel::Ne => a != b,
                Rel::Lt => a < b,
                Rel::Le => a <= b,
                Rel::Gt => a > b,
                Rel::Ge => a >= b,
            })
        }
        Cond::Test(e) => k(e).map(|v| v != 0),
        Cond::Odd(e) => k(e).map(|v| v & 1 != 0),
        Cond::Not(c) => known(c).map(|b| !b),
        Cond::AndThen(a, b) => match known(a)? {
            false => Some(false),
            true => known(b),
        },
        Cond::OrElse(a, b) => match known(a)? {
            true => Some(true),
            false => known(b),
        },
        Cond::And(a, b) => Some(known(a)? && known(b)?),
        Cond::Or(a, b) => Some(known(a)? || known(b)?),
        Cond::Block { .. } | Cond::SetEmpty { .. } => None,
    }
}

/// Remove the stores of the candidates no read is left of, where the
/// value calls nothing.
fn drop_dead_stores(body: &mut Vec<Stmt>, locals: &HashMap<i32, u32>) {
    let mut read = HashSet::new();
    let mut scan = Reads {
        locals,
        read: &mut read,
    };
    scan.stmts(body);
    let dead: HashSet<i32> = locals
        .keys()
        .filter(|o| !read.contains(o))
        .copied()
        .collect();
    if !dead.is_empty() {
        drop_stores(body, &dead);
    }
}

fn drop_stores(body: &mut Vec<Stmt>, dead: &HashSet<i32>) {
    body.retain(|s| match &s.kind {
        StmtKind::Store {
            addr: Addr::Frame(o),
            value,
        } => !dead.contains(o) || has_call(value),
        _ => true,
    });
    for s in body.iter_mut() {
        match &mut s.kind {
            StmtKind::If { then_, else_, .. } => {
                drop_stores(then_, dead);
                drop_stores(else_, dead);
            }
            StmtKind::Repeat { body, .. } | StmtKind::While { body, .. } => drop_stores(body, dead),
            StmtKind::For(l) => drop_stores(&mut l.body, dead),
            StmtKind::Case {
                arms, otherwise, ..
            } => {
                for a in arms {
                    drop_stores(&mut a.body, dead);
                }
                if let Some(o) = otherwise {
                    drop_stores(o, dead);
                }
            }
            _ => {}
        }
    }
}

/// Whether `e` calls a routine.
fn has_call(e: &Expr) -> bool {
    let mut found = false;
    walk_expr(e, &mut |x| {
        if matches!(x, Expr::Call { .. }) {
            found = true;
        }
    });
    found
}

/// Every expression under `e`, `e` first (the operands of its addresses
/// too, and those of calls; not the conditions' expressions under
/// `Expr::Test`).
fn walk_expr(e: &Expr, f: &mut dyn FnMut(&Expr)) {
    f(e);
    let addr = |a: &Addr, f: &mut dyn FnMut(&Expr)| walk_addr(a, f);
    match e {
        Expr::Const { .. } => {}
        Expr::Load { addr: a, .. } | Expr::Bits { addr: a, .. } | Expr::AddrOf(a) => addr(a, f),
        Expr::BitElem { array, index, .. } => {
            addr(array, f);
            walk_expr(index, f);
        }
        Expr::SetMember { elem, set, .. } => {
            walk_expr(elem, f);
            walk_expr(set, f);
        }
        Expr::SetMemberAt { elem, set, .. } => {
            walk_expr(elem, f);
            addr(set, f);
        }
        Expr::SetOp { lhs, rhs, .. } | Expr::Binary { lhs, rhs, .. } => {
            walk_expr(lhs, f);
            walk_expr(rhs, f);
        }
        Expr::SetOf { elem, .. } => walk_expr(elem, f),
        Expr::Unary { operand: x, .. } | Expr::Convert { value: x, .. } | Expr::SetSr(x) => {
            walk_expr(x, f)
        }
        Expr::Call { call, .. } => {
            if let Callee::Pointer(x) = &call.callee {
                walk_expr(x, f);
            }
            for a in &call.args {
                match a {
                    Arg::Value { value, .. } | Arg::Temp { value, .. } => walk_expr(value, f),
                    Arg::Address { addr: a, .. } => addr(a, f),
                    Arg::Call { .. } => {}
                }
            }
        }
        Expr::Test(_) => {}
    }
}

fn walk_addr(a: &Addr, f: &mut dyn FnMut(&Expr)) {
    match a {
        Addr::Indirect { base, .. } => walk_expr(base, f),
        Addr::Index { base, index, .. } => {
            walk_addr(base, f);
            walk_expr(index, f);
        }
        Addr::Volatile { addr, .. } => walk_addr(addr, f),
        _ => {}
    }
}

/// Collects the candidates read (whole or in part) anywhere.
struct Reads<'a> {
    locals: &'a HashMap<i32, u32>,
    read: &'a mut HashSet<i32>,
}

impl Reads<'_> {
    fn note(&mut self, x: i32) {
        for (&o, &w) in self.locals {
            if x >= o && x < o + w as i32 {
                self.read.insert(o);
            }
        }
    }

    fn expr(&mut self, e: &Expr) {
        let mut loads = Vec::new();
        let mut conds = Vec::new();
        walk_expr(e, &mut |x| match x {
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } => loads.push(*o),
            Expr::Test(c) => conds.push((**c).clone()),
            _ => {}
        });
        for o in loads {
            self.note(o);
        }
        for c in conds {
            self.cond(&c);
        }
    }

    fn cond(&mut self, c: &Cond) {
        match c {
            Cond::Compare { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            Cond::Test(e) | Cond::Odd(e) => self.expr(e),
            Cond::Not(c) => self.cond(c),
            Cond::AndThen(a, b) | Cond::OrElse(a, b) | Cond::And(a, b) | Cond::Or(a, b) => {
                self.cond(a);
                self.cond(b);
            }
            Cond::Block { .. } | Cond::SetEmpty { .. } => {}
        }
    }

    fn call(&mut self, c: &Call) {
        let e = Expr::Call {
            call: c.clone(),
            ty: Scalar::BOOLEAN,
            reg: crate::ResultClass::Data,
        };
        self.expr(&e);
        for a in &c.args {
            if let Arg::Call { call, .. } = a {
                self.call(call);
            }
        }
    }

    fn addr(&mut self, a: &Addr) {
        let mut loads = Vec::new();
        walk_addr(a, &mut |x| {
            if let Expr::Load {
                addr: Addr::Frame(o),
                ..
            } = x
            {
                loads.push(*o);
            }
        });
        for o in loads {
            self.note(o);
        }
    }

    fn stmts(&mut self, body: &[Stmt]) {
        for s in body {
            match &s.kind {
                StmtKind::SetResult(e) | StmtKind::Discard(e) => self.expr(e),
                StmtKind::Store { addr, value } => {
                    self.addr(addr);
                    self.expr(value);
                }
                StmtKind::StoreBits { addr, value, .. } => {
                    self.addr(addr);
                    self.expr(value);
                }
                StmtKind::StoreBitElem { array, index, .. } => {
                    self.addr(array);
                    self.expr(index);
                }
                StmtKind::StoreSetMember { set, elem, .. } => {
                    self.addr(set);
                    self.expr(elem);
                }
                StmtKind::StoreSetConst { set, .. } => self.addr(set),
                StmtKind::StoreSetRange { set, lo, hi, .. } => {
                    self.addr(set);
                    self.expr(hi);
                    self.expr(lo);
                }
                StmtKind::Copy { dst, src, .. } => {
                    self.addr(dst);
                    self.addr(src);
                }
                StmtKind::Call(c) => self.call(c),
                StmtKind::Fill { start, value, .. } => {
                    self.addr(start);
                    self.expr(value);
                }
                StmtKind::If { cond, then_, else_ } => {
                    self.cond(cond);
                    self.stmts(then_);
                    self.stmts(else_);
                }
                StmtKind::Repeat { body, until } => {
                    self.stmts(body);
                    if let Some(c) = until {
                        self.cond(c);
                    }
                }
                StmtKind::While { cond, body } => {
                    self.cond(cond);
                    self.stmts(body);
                }
                StmtKind::For(l) => {
                    self.expr(&l.from);
                    self.expr(&l.to);
                    self.stmts(&l.body);
                }
                StmtKind::Case {
                    selector,
                    arms,
                    otherwise,
                } => {
                    self.expr(selector);
                    for a in arms {
                        self.stmts(&a.body);
                    }
                    if let Some(o) = otherwise {
                        self.stmts(o);
                    }
                }
                _ => {}
            }
        }
    }
}
