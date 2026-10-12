//! Register bookkeeping for instruction selection (codegen-patterns 2, 12).
//!
//! The selector picks physical registers directly; there is no virtual
//! register pass.  [`Regs`] knows which registers hold a value right now
//! and which callee-saved registers the routine has written anywhere,
//! which is what the prologue must save ([`crate::frame::Frame`]).
//!
//! Two ways to get a register:
//!
//! - [`Regs::alloc_scratch`]: D0, D1 / A0, A1, lowest free first.  This is
//!   the only policy the selector uses today (PROC1_$TST_LOCK
//!   0xE148CE..0xE148D8; TIME_$Q_INIT_QUEUE 0xE16C62..0xE16C6A
//!   `move.b (8,A6),D0; move.w (0xa,A6),D1; movea.l (0xc,A6),A0`).
//!   Scratch registers do not survive a call.
//! - [`Regs::alloc_held`]: a callee-saved register, D2..D7 / A2..A4,
//!   lowest free first, recorded for the prologue's save.  For a value
//!   that must survive a call, e.g. a parameter held across `jsr`
//!   (0xE135A6 `move.l (0xc,A6),D2`, used after the call at 0xE135BC by
//!   0xE135D4 `move.w D2,-(SP)`).  **Uncertain:** that the original took
//!   the lowest free callee-saved register: the sites seen so far
//!   (0xE1359C D2, A2, A3; PROC1_$SET_ASID 0xE148FE D2; 0xE17778 D2, D3,
//!   A2) agree, but OSINFO_$GET_MMAP 0xE5C69C puts its first parameter in
//!   D3, not D2.  When to hold a value at all is the caller's decision,
//!   and is not catalogued as a rule (codegen-patterns 12).
//!
//! A5 (module data base, codegen-patterns 4) and A6 (frame pointer) are
//! never allocated.  A routine that loads A5 reports it with
//! [`Regs::clobber`], so that it is saved like the others.

use crate::insn::{Reg, RegSet};

/// Data or address register.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegClass {
    Data,
    Addr,
}

/// Scratch registers, in allocation order (codegen-patterns 2).
pub const SCRATCH_D: [u8; 2] = [0, 1];
pub const SCRATCH_A: [u8; 2] = [0, 1];

/// Allocatable callee-saved registers, in allocation order (ref 5.5.8
/// `nosave`; codegen-patterns 2).  A5 is the module data base and A6 the
/// frame pointer; they are callee-saved but not allocatable.
pub const HELD_D: [u8; 6] = [2, 3, 4, 5, 6, 7];
pub const HELD_A: [u8; 3] = [2, 3, 4];

/// The data base register (codegen-patterns 4).
pub const DATA_BASE: Reg = Reg::A(5);

/// Every register a routine must preserve for its caller (A6 is restored
/// by `unlk`, A7 by the frame discipline).
pub fn callee_saved() -> RegSet {
    let mut s = RegSet::EMPTY;
    for n in HELD_D {
        s.insert(Reg::D(n));
    }
    for n in HELD_A {
        s.insert(Reg::A(n));
    }
    s.insert(DATA_BASE);
    s
}

fn reg(class: RegClass, n: u8) -> Reg {
    match class {
        RegClass::Data => Reg::D(n),
        RegClass::Addr => Reg::A(n),
    }
}

/// Which registers hold a value now, and which callee-saved registers the
/// routine has written.
#[derive(Clone, Debug, Default)]
pub struct Regs {
    busy: RegSet,
    written: RegSet,
}

impl Regs {
    pub fn new() -> Regs {
        Regs::default()
    }

    fn alloc_from(&mut self, class: RegClass, pool: &[u8]) -> Option<Reg> {
        let r = pool
            .iter()
            .map(|&n| reg(class, n))
            .find(|&r| !self.busy.contains(r))?;
        self.busy.insert(r);
        if callee_saved().contains(r) {
            self.written.insert(r);
        }
        Some(r)
    }

    /// The lowest free scratch register of `class`.
    pub fn alloc_scratch(&mut self, class: RegClass) -> Option<Reg> {
        let pool: &[u8] = match class {
            RegClass::Data => &SCRATCH_D,
            RegClass::Addr => &SCRATCH_A,
        };
        self.alloc_from(class, pool)
    }

    /// The lowest free callee-saved register of `class`, for a value that
    /// must survive a call.  The register joins the prologue's save set.
    pub fn alloc_held(&mut self, class: RegClass) -> Option<Reg> {
        let pool: &[u8] = match class {
            RegClass::Data => &HELD_D,
            RegClass::Addr => &HELD_A,
        };
        self.alloc_from(class, pool)
    }

    /// Release `r`.  It stays in the save set if it is callee-saved.
    pub fn free(&mut self, r: Reg) {
        debug_assert!(self.busy.contains(r), "freeing free register {r}");
        self.busy.remove(r);
    }

    pub fn is_busy(&self, r: Reg) -> bool {
        self.busy.contains(r)
    }

    /// The body writes `r` outside the allocator (A5 loaded with the
    /// module's data base, codegen-patterns 4).  A callee-saved `r` joins
    /// the save set.
    pub fn clobber(&mut self, r: Reg) {
        if callee_saved().contains(r) {
            self.written.insert(r);
        }
    }

    /// Scratch registers holding a value now.  A value in one of them
    /// does not survive a call; the call selector must find this empty
    /// (or the value must have been put in a held register).
    pub fn live_scratch(&self) -> RegSet {
        self.busy
            .intersection(RegSet::of(&[Reg::D(0), Reg::D(1), Reg::A(0), Reg::A(1)]))
    }

    /// The callee-saved registers written so far: what the prologue saves.
    pub fn to_save(&self) -> RegSet {
        self.written
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratch_order_and_reuse() {
        let mut r = Regs::new();
        assert_eq!(r.alloc_scratch(RegClass::Data), Some(Reg::D(0)));
        assert_eq!(r.alloc_scratch(RegClass::Data), Some(Reg::D(1)));
        assert_eq!(r.alloc_scratch(RegClass::Data), None);
        assert_eq!(r.alloc_scratch(RegClass::Addr), Some(Reg::A(0)));
        r.free(Reg::D(0));
        assert_eq!(r.alloc_scratch(RegClass::Data), Some(Reg::D(0)));
        assert_eq!(
            r.live_scratch(),
            RegSet::of(&[Reg::D(0), Reg::D(1), Reg::A(0)])
        );
        // Scratch registers are never saved.
        assert!(r.to_save().is_empty());
    }

    #[test]
    fn held_registers_join_the_save_set() {
        let mut r = Regs::new();
        let d = r.alloc_held(RegClass::Data).unwrap();
        let a = r.alloc_held(RegClass::Addr).unwrap();
        assert_eq!((d, a), (Reg::D(2), Reg::A(2)));
        assert!(r.live_scratch().is_empty());
        r.free(d);
        // Freed, but written: still saved.  The next held is D2 again.
        assert_eq!(r.to_save(), RegSet::of(&[Reg::D(2), Reg::A(2)]));
        assert_eq!(r.alloc_held(RegClass::Data), Some(Reg::D(2)));
        assert_eq!(r.alloc_held(RegClass::Addr), Some(Reg::A(3)));
        assert_eq!(r.alloc_held(RegClass::Addr), Some(Reg::A(4)));
        // A5 is never allocated.
        assert_eq!(r.alloc_held(RegClass::Addr), None);
        for n in 3..=7 {
            assert_eq!(r.alloc_held(RegClass::Data), Some(Reg::D(n)));
        }
        assert_eq!(r.alloc_held(RegClass::Data), None);
    }

    #[test]
    fn clobber_records_only_callee_saved() {
        let mut r = Regs::new();
        r.clobber(Reg::A(1));
        r.clobber(Reg::D(0));
        assert!(r.to_save().is_empty());
        r.clobber(DATA_BASE);
        assert_eq!(r.to_save(), RegSet::of(&[Reg::A(5)]));
    }
}
