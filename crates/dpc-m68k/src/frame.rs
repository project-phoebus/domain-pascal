//! Frame layout, prologue and epilogue (codegen-patterns 2, 12.1).
//!
//! ```text
//!   (8,A6)...          arguments
//!   (4,A6)             return address
//!   (0,A6)             caller's A6
//!   (-1,A6)..(-link,A6)            locals (`link.w A6,#-link`)
//!   (-link-4*n,A6)..(-link-1,A6)   the n saved registers
//!   below                          pushes (argument areas, ...)
//! ```
//!
//! The saved registers sit directly below the `link` area, so the
//! epilogue restores them from `(-(link + 4n),A6)` whatever SP is:
//! 0xE17778 `link #-0x10`, D2-D3/A2 restored from `(-0x1c,A6)`;
//! 0xE1359C `link #-4`, D2/A2-A3 from `(-0x10,A6)`; OSINFO_$GET_MMAP
//! `link #-0x18`, 9 registers from `(-0x3c,A6)`; MAC_OS_$OPEN `link
//! #-0x28`, 10 registers from `(-0x50,A6)`; one register: PROC1_$SET_ASID
//! `link #0`, D2 from `(-4,A6)`, TONE_$ENABLE `link #-8`, A5 from
//! `(-0xc,A6)`.  A sweep of every `link` + save at a routine start in
//! 0xE00800..0xE88834 finds no exception (1162 `movem` restores, 309
//! single-register restores).
//!
//! **How many registers are saved decides the instruction** (same sweep):
//!
//! - none: nothing (TIME_$Q_INIT_QUEUE 0xE16C5E);
//! - one: `move.l Dn,-(SP)` (34 sites, all D2, e.g. PROC1_$SET_ASID
//!   0xE148FC) or `pea (An)` (A2 ×92, e.g. 0xE14922; A5 ×183, e.g.
//!   PROC1_$GET_LOADAV 0xE14BBE), restored with `move.l (-k,A6),Dn` /
//!   `movea.l (-k,A6),An` (0xE14916, 0xE14BD2);
//! - two or more: `movem.l <regs>,-(SP)` and `movem.l (-k,A6),<regs>`;
//!   no `movem` with a one-register mask follows a `link` in the image.
//!
//! A5 is saved like the other callee-saved registers: inside the `movem`
//! when there is one (MAC_OS_$OPEN 0xE0B24A, 685 sites), with `pea (A5)`
//! when it is the only one; `pea (A5)` is never followed by a `movem`.

use dpc_syntax::Diagnostic;

use crate::insn::{Ea, Insn, Reg, RegSet, Size};
use crate::M68k;

/// The frame pointer and stack pointer numbers.
pub const FP: u8 = 6;
pub const SP: u8 = 7;

/// A routine's frame: the `link` displacement and the registers saved
/// below it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    /// Bytes reserved by `link.w A6,#-link` (a multiple of 4).
    pub link: u32,
    /// Callee-saved registers written by the body.
    pub saves: RegSet,
}

impl Frame {
    /// The frame for `raw` bytes of locals (`dpc_ir::Function::frame_size`)
    /// and the save set `saves`.  `Err(what)` if `link.w` cannot reach.
    pub fn new(raw: u32, saves: RegSet) -> Result<Frame, String> {
        let link = M68k::link_size(raw);
        // `link.w` only (codegen-patterns 2); with the save area below it
        // the restore displacement must fit 16 bits as well.
        let total = link + 4 * saves.len();
        if total > 0x8000 {
            return Err(format!("a {total}-byte frame (beyond `link.w`) is"));
        }
        Ok(Frame { link, saves })
    }

    /// The A6 offset of the save area's lowest byte, where the epilogue
    /// restores from.
    pub fn save_area_offset(&self) -> i32 {
        -((self.link + 4 * self.saves.len()) as i32)
    }

    /// `link.w`, then the register save.
    pub fn prologue(&self) -> Vec<Insn> {
        let mut out = vec![Insn::Link {
            disp: -(self.link as i32),
        }];
        match self.single() {
            None if self.saves.is_empty() => {}
            None => out.push(Insn::MovemSave { regs: self.saves }),
            Some(Reg::D(d)) => out.push(Insn::Move {
                size: Size::L,
                src: Ea::Reg(Reg::D(d)),
                dst: Ea::PreDec(SP),
            }),
            Some(Reg::A(a)) => out.push(Insn::Pea(Ea::Ind(a))),
        }
        out
    }

    /// The restore from the frame, then `unlk A6; rts`.
    pub fn epilogue(&self) -> Vec<Insn> {
        let disp = self.save_area_offset();
        let mut out = Vec::new();
        match self.single() {
            None if self.saves.is_empty() => {}
            None => out.push(Insn::MovemRestore {
                regs: self.saves,
                disp,
            }),
            Some(Reg::D(d)) => out.push(Insn::Move {
                size: Size::L,
                src: Ea::Disp(disp, FP),
                dst: Ea::Reg(Reg::D(d)),
            }),
            Some(Reg::A(a)) => out.push(Insn::Movea {
                size: Size::L,
                src: Ea::Disp(disp, FP),
                dst: a,
            }),
        }
        out.push(Insn::Unlk);
        out.push(Insn::Rts);
        out
    }

    fn single(&self) -> Option<Reg> {
        if self.saves.len() == 1 {
            self.saves.iter().next()
        } else {
            None
        }
    }
}

/// [`Frame::new`] with a diagnostic at `span`.
pub(crate) fn frame_for(
    raw: u32,
    saves: RegSet,
    span: dpc_syntax::Span,
) -> Result<Frame, Diagnostic> {
    Frame::new(raw, saves)
        .map_err(|what| Diagnostic::error(span, format!("{what} not supported yet")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(insns: &[Insn]) -> Vec<String> {
        insns.iter().map(|i| i.to_string()).collect()
    }

    fn d(n: u8) -> Reg {
        Reg::D(n)
    }
    fn a(n: u8) -> Reg {
        Reg::A(n)
    }

    #[test]
    fn no_saved_registers() {
        // TIME_$Q_INIT_QUEUE 0xE16C5E / 0xE16C7C.
        let f = Frame::new(0, RegSet::EMPTY).unwrap();
        assert_eq!(text(&f.prologue()), ["link.w\t%fp,#0"]);
        assert_eq!(text(&f.epilogue()), ["unlk\t%fp", "rts"]);
        assert_eq!(f.save_area_offset(), 0);
    }

    #[test]
    fn movem_save_and_restore_below_the_locals() {
        // 0xE17778: link #-0x10; movem.l D2-D3/A2,-(SP) ...
        // 0xE177F6: movem.l (-0x1c,A6),D2-D3/A2; unlk; rts.
        let f = Frame::new(16, RegSet::of(&[d(2), d(3), a(2)])).unwrap();
        assert_eq!(f.save_area_offset(), -28);
        assert_eq!(
            text(&f.prologue()),
            ["link.w\t%fp,#-0x10", "movem.l\t%d2-%d3/%a2,-(%sp)"]
        );
        assert_eq!(
            text(&f.epilogue()),
            ["movem.l\t(-0x1c,%fp),%d2-%d3/%a2", "unlk\t%fp", "rts"]
        );
    }

    #[test]
    fn restore_offsets_from_the_image() {
        // (raw locals, saves, restore displacement, image address)
        let all_d = [d(2), d(3), d(4), d(5), d(6), d(7)];
        let mut osinfo = all_d.to_vec();
        osinfo.extend([a(2), a(3), a(4)]);
        let mut mac_os = osinfo.clone();
        mac_os.push(a(5));
        let cases: Vec<(u32, Vec<Reg>, i32)> = vec![
            (4, vec![d(2), a(2), a(3)], -0x10), // 0xE13602
            (0x18, osinfo, -0x3c),              // 0xE5C9F6
            (0x28, mac_os, -0x50),              // 0xE0B452
            (0, vec![d(2)], -4),                // 0xE14916
            (8, vec![a(5)], -0xc),              // 0xE1AD0E
            (0, vec![a(5)], -4),                // 0xE14BD2
            (12, vec![a(2)], -0x10),            // 0xE1491E (pea (A2))
        ];
        for (raw, regs, want) in cases {
            let f = Frame::new(raw, RegSet::of(&regs)).unwrap();
            assert_eq!(f.save_area_offset(), want, "{regs:?}");
        }
    }

    #[test]
    fn rounded_link_is_the_base_of_the_save_area() {
        // A 1-byte frame is `link #-4` (PROC1_$TST_LOCK); saves go below.
        let f = Frame::new(1, RegSet::of(&[d(2), d(3)])).unwrap();
        assert_eq!(f.link, 4);
        assert_eq!(f.save_area_offset(), -12);
    }

    #[test]
    fn single_data_register() {
        // PROC1_$SET_ASID 0xE148F8..0xE1491C.
        let f = Frame::new(0, RegSet::of(&[d(2)])).unwrap();
        assert_eq!(
            text(&f.prologue()),
            ["link.w\t%fp,#0", "move.l\t%d2,-(%sp)"]
        );
        assert_eq!(
            text(&f.epilogue()),
            ["move.l\t(-4,%fp),%d2", "unlk\t%fp", "rts"]
        );
    }

    #[test]
    fn single_address_register_uses_pea() {
        // TONE_$ENABLE 0xE1ACE8 (A5), 0xE1491E (A2).
        let f = Frame::new(8, RegSet::of(&[a(5)])).unwrap();
        assert_eq!(text(&f.prologue()), ["link.w\t%fp,#-8", "pea\t(%a5)"]);
        assert_eq!(
            text(&f.epilogue()),
            ["movea.l\t(-0xc,%fp),%a5", "unlk\t%fp", "rts"]
        );
        let f = Frame::new(12, RegSet::of(&[a(2)])).unwrap();
        assert_eq!(text(&f.prologue())[1], "pea\t(%a2)");
        assert_eq!(text(&f.epilogue())[0], "movea.l\t(-0x10,%fp),%a2");
    }

    #[test]
    fn a5_goes_inside_the_movem() {
        // MAC_OS_$OPEN 0xE0B24A: movem.l D2-D7/A2-A5,-(SP).
        let f = Frame::new(0x28, RegSet::of(&[d(2), a(5)])).unwrap();
        assert_eq!(text(&f.prologue())[1], "movem.l\t%d2/%a5,-(%sp)");
    }

    #[test]
    fn frame_limit_counts_the_save_area() {
        assert!(Frame::new(0x8000, RegSet::EMPTY).is_ok());
        assert!(Frame::new(0x8000, RegSet::of(&[d(2)])).is_err());
        assert!(Frame::new(0x8001, RegSet::EMPTY).is_err());
    }
}
