//! How a routine body uses its scalar locals (codegen-patterns 12.5).
//!
//! The selector decides from these facts which locals it holds in a
//! register rather than in their frame slot (`isel.rs`,
//! `register_locals`); `dpc --dump-locals` prints them for
//! `tools/localsurvey.py`, which sets them against the image.
//!
//! Statements are numbered in source order (pre-order: a structured
//! statement before the statements inside it).  A reference records the
//! number of its statement (`seq`), of the routine-level statement that
//! contains it (`top`), whether it lies inside an `if`, loop or `case`
//! (`nested`), and the innermost loop around it.

use dpc_ir::{Addr, Arg, Call, Cond, Expr, Function, Stmt, StmtKind, Width};

/// Where a reference is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Site {
    /// The index of the routine-level statement that contains it.
    pub top: usize,
    /// The number of its statement (source order, pre-order).
    pub seq: usize,
    /// Inside an `if`, loop or `case`.
    pub nested: bool,
    /// The innermost loop around it: the numbers of its first and last
    /// statements (the loop statement itself and the last one inside).
    pub lp: Option<(usize, usize)>,
}

/// An assignment of the whole local.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoreUse {
    pub at: Site,
    /// The value is a constant.
    pub konst: bool,
    /// The value is a function's result, as it is (`x := f(...)`).
    pub call: bool,
}

/// A read of the whole local.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadUse {
    pub at: Site,
    /// Inside an argument of a call (passed by value, or inside an
    /// expression passed).
    pub arg: bool,
    /// The argument is the local itself (widened or not): the local's
    /// value is pushed.
    pub whole: bool,
    /// Inside the condition of an `if` or a loop.
    pub cond: bool,
}

/// The uses of one scalar local ([`Function::scalar_locals`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalUse {
    pub offset: i32,
    pub width: Width,
    pub stores: Vec<StoreUse>,
    pub loads: Vec<LoadUse>,
    /// Dereferenced as a pointer (`p^`, `p^.f`).
    pub deref: bool,
    /// Any other reference: its address passed or taken, part of it read
    /// or written, a block copy or fill over it.
    pub other: bool,
    /// The index of a `for` loop (held in the loop's own register,
    /// codegen-patterns 7.4).
    pub for_index: bool,
}

/// The uses of every scalar local of a body, and its calls.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BodyUses {
    pub locals: Vec<LocalUse>,
    /// Every call (routine calls; not M$ARITH, which the IR does not
    /// call), where it is.
    pub calls: Vec<Site>,
}

impl BodyUses {
    /// Whether some call lies between an assignment of `l` and a later
    /// read (statement numbers strictly between), or in a loop around a
    /// read (the next iteration reads the value again).
    pub fn across_call(&self, l: &LocalUse) -> bool {
        l.loads.iter().any(|r| {
            let between = l.stores.iter().any(|s| {
                self.calls
                    .iter()
                    .any(|c| s.at.seq < c.seq && c.seq < r.at.seq)
            });
            let in_loop =
                r.at.lp
                    .is_some_and(|(lo, hi)| self.calls.iter().any(|c| lo <= c.seq && c.seq <= hi));
            between || in_loop
        })
    }
}

/// The uses of `f`'s scalar locals.
pub fn body_uses(f: &Function) -> BodyUses {
    let mut w = Walk {
        f,
        out: BodyUses {
            locals: f
                .scalar_locals
                .iter()
                .map(|l| LocalUse {
                    offset: l.offset,
                    width: l.ty.width,
                    stores: Vec::new(),
                    loads: Vec::new(),
                    deref: false,
                    other: false,
                    for_index: false,
                })
                .collect(),
            calls: Vec::new(),
        },
        site: Site {
            top: 0,
            seq: 0,
            nested: false,
            lp: None,
        },
        next: 0,
        in_arg: false,
        whole: false,
        in_cond: false,
    };
    w.stmts(&f.body, false);
    w.out
}

struct Walk<'a> {
    f: &'a Function,
    out: BodyUses,
    site: Site,
    /// The next statement number.
    next: usize,
    in_arg: bool,
    /// Walking an argument that is a frame object's value as it is.
    whole: bool,
    in_cond: bool,
}

impl Walk<'_> {
    /// The local whose object covers frame offset `o`.
    fn local(&self, o: i32) -> Option<usize> {
        self.f
            .scalar_locals
            .iter()
            .position(|l| o >= l.offset && o < l.offset + l.ty.bytes() as i32)
    }

    fn exact(&self, o: i32, w: Width) -> Option<usize> {
        self.f
            .scalar_locals
            .iter()
            .position(|l| l.offset == o && l.ty.width == w)
    }

    fn other(&mut self, a: &Addr) {
        match a {
            Addr::Frame(o) => {
                if let Some(k) = self.local(*o) {
                    self.out.locals[k].other = true;
                }
            }
            Addr::Enclosing(_)
            | Addr::Static(_)
            | Addr::OtherStatic { .. }
            | Addr::Symbol { .. }
            | Addr::Const(_) => {}
            Addr::Volatile { addr, .. } => self.other(addr),
            Addr::Indirect { base, .. } => match &**base {
                // A local dereferenced as a pointer.
                Expr::Load {
                    addr: Addr::Frame(o),
                    ty,
                } if self.exact(*o, ty.width).is_some() => {
                    let k = self.exact(*o, ty.width).unwrap();
                    self.out.locals[k].deref = true;
                }
                Expr::Load { addr, .. } => self.other(addr),
                b => self.expr(b),
            },
            Addr::Index { base, index, .. } => {
                self.other(base);
                self.expr(index);
            }
        }
    }

    fn expr(&mut self, e: &Expr) {
        match e {
            Expr::Load {
                addr: Addr::Frame(o),
                ty,
            } if self.local(*o).is_some() => match self.exact(*o, ty.width) {
                Some(k) => {
                    let u = LoadUse {
                        at: self.site,
                        arg: self.in_arg,
                        whole: self.whole,
                        cond: self.in_cond,
                    };
                    self.out.locals[k].loads.push(u);
                }
                None => self.other(&Addr::Frame(*o)),
            },
            // A bit field is read in memory, never in a register.
            Expr::Load { addr, .. } | Expr::Bits { addr, .. } | Expr::AddrOf(addr) => {
                self.other(addr)
            }
            Expr::Const { .. } => {}
            Expr::SetMember { elem, set, .. } => {
                self.expr(elem);
                self.expr(set);
            }
            Expr::SetMemberAt { elem, set, .. }
            | Expr::BitElem {
                index: elem,
                array: set,
                ..
            } => {
                self.expr(elem);
                self.other(set);
            }
            Expr::Binary { lhs, rhs, .. } | Expr::SetOp { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            Expr::Unary { operand: x, .. }
            | Expr::Convert { value: x, .. }
            | Expr::SetSr(x)
            | Expr::SetOf { elem: x, .. } => self.expr(x),
            Expr::Call { call, .. } => self.call(call),
            Expr::Test(c) => self.cond(c),
        }
    }

    fn call(&mut self, c: &Call) {
        let outer = self.in_arg;
        self.in_arg = true;
        for a in &c.args {
            match a {
                Arg::Value { value, .. } | Arg::Temp { value, .. } => {
                    let mut v = value;
                    while let Expr::Convert { value: x, .. } = v {
                        v = x;
                    }
                    self.whole = matches!(
                        v,
                        Expr::Load {
                            addr: Addr::Frame(_),
                            ..
                        }
                    );
                    self.expr(v);
                    self.whole = false;
                }
                Arg::Address { addr, .. } => self.other(addr),
                Arg::Call { call, .. } => self.call(call),
            }
        }
        self.in_arg = outer;
        // The routine pointer is read by the call, not as an argument.
        if let Some(x) = c.pointer() {
            self.expr(x);
        }
        self.out.calls.push(self.site);
    }

    fn cond(&mut self, c: &Cond) {
        let outer = self.in_cond;
        self.in_cond = true;
        match c {
            Cond::Compare { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            Cond::Test(e) | Cond::Odd(e) => self.expr(e),
            Cond::Not(x) => self.cond(x),
            Cond::AndThen(a, b) | Cond::OrElse(a, b) | Cond::And(a, b) | Cond::Or(a, b) => {
                self.cond(a);
                self.cond(b);
            }
            Cond::Block { lhs, rhs, .. } => {
                self.other(lhs);
                self.other(rhs);
            }
            Cond::SetEmpty { set, .. } => self.other(set),
        }
        self.in_cond = outer;
    }

    /// The statements of a loop body, with the loop's range in their
    /// sites; `head` is the loop statement's number.
    fn loop_body(&mut self, head: usize, body: &[Stmt], after: impl FnOnce(&mut Self)) {
        let mut last = head;
        dpc_ir::visit_stmts(body, &mut |_| last += 1);
        let outer = self.site;
        self.site.lp = Some((head, last));
        self.site.nested = true;
        self.stmts(body, true);
        self.site.seq = head;
        after(self);
        self.site = outer;
    }

    fn stmts(&mut self, body: &[Stmt], nested: bool) {
        for (i, st) in body.iter().enumerate() {
            let outer = self.site;
            if !nested {
                self.site.top = i;
            }
            self.site.seq = self.next;
            self.next += 1;
            self.site.nested = nested;
            let seq = self.site.seq;
            match &st.kind {
                StmtKind::Store {
                    addr: Addr::Frame(o),
                    value,
                } if self.exact(*o, value.ty().width).is_some() => {
                    self.expr(value);
                    let k = self.exact(*o, value.ty().width).unwrap();
                    let u = StoreUse {
                        at: self.site,
                        konst: matches!(value, Expr::Const { .. }),
                        call: matches!(value, Expr::Call { .. }),
                    };
                    self.out.locals[k].stores.push(u);
                }
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
                } => {
                    self.other(addr);
                    self.expr(value);
                }
                StmtKind::StoreSetConst { set, .. } => self.other(set),
                StmtKind::SetResult(e) | StmtKind::Discard(e) => self.expr(e),
                StmtKind::Copy { dst, src, .. } => {
                    self.other(src);
                    self.other(dst);
                }
                StmtKind::Call(c) => self.call(c),
                StmtKind::Fill { start, value, .. } => {
                    self.other(start);
                    self.expr(value);
                }
                StmtKind::If { cond, then_, else_ } => {
                    self.cond(cond);
                    self.stmts(then_, true);
                    self.stmts(else_, true);
                }
                StmtKind::Repeat { body, until } => {
                    self.loop_body(seq, body, |w| {
                        if let Some(u) = until {
                            w.cond(u)
                        }
                    });
                }
                StmtKind::While { cond, body } => {
                    self.loop_body(seq, body, |w| w.cond(cond));
                }
                // The index is the loop's own register (`Selector::for_`);
                // its reads count as reads of a local with no store.
                StmtKind::For(l) => {
                    if let Some(h) = &l.home {
                        // Kept in memory (`Selector::for_wide`).
                        self.other(h);
                    }
                    if l.home.is_none() {
                        if let Some(k) = self.exact(l.index.offset, l.index.ty.width) {
                            self.out.locals[k].for_index = true;
                        }
                    }
                    self.expr(&l.from);
                    self.expr(&l.to);
                    self.loop_body(seq, &l.body, |_| {});
                }
                StmtKind::Case {
                    selector,
                    arms,
                    otherwise,
                } => {
                    self.expr(selector);
                    for a in arms {
                        self.stmts(&a.body, true);
                    }
                    if let Some(o) = otherwise {
                        self.stmts(o, true);
                    }
                }
                StmtKind::Return
                | StmtKind::Disable
                | StmtKind::Enable
                | StmtKind::Exit
                | StmtKind::Next
                | StmtKind::Label(_)
                | StmtKind::Goto(_) => {}
            }
            let top = self.site.top;
            self.site = outer;
            if !nested {
                self.site.top = top;
            }
        }
    }
}
