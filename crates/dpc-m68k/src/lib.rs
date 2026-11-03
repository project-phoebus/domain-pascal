//! m68k backend: target description, instruction selection and the
//! gas-syntax emitter.  Output must follow the code patterns of Apollo's
//! SR10.2 compiler; see `docs/codegen-patterns.md`.
//!
//! - [`isel`]: IR -> [`insn::Insn`] over virtual registers, pattern by
//!   pattern, body first;
//! - [`regs`]: liveness and register assignment (codegen-patterns 12.4
//!   R1-R3), and so which callee-saved registers the body writes;
//! - [`sched`]: the order of instructions inside basic blocks, after
//!   allocation (12.4 R9-R12);
//! - [`frame`]: frame layout, prologue and epilogue around the body;
//! - [`insn`]: instructions and their gas text;
//! - [`emit`]: a whole module as an assembly file;
//! - [`runtime`]: the run-time library M$ARITH and its entry names.

pub mod crossjump;
pub mod emit;
pub mod frame;
pub mod insn;
pub mod isel;
pub mod locals;
pub mod regs;
pub mod runtime;
pub mod sched;
pub mod spill;

use dpc_target::Target;

/// The processor the code is generated for (ref 6.4.9 `-cpu`, Table 6-3,
/// p. 6-16; `docs/compiler-options.md`).
///
/// The SR10.2 kernels come in two families: SAU2..SAU5 are MC68010 code
/// (CLAUDE.md 3.16), SAU6..SAU9 MC68020-class code, the same sources
/// compiled with the processor-specific instructions of Table 6-3's
/// `mathchip` group (measured over the SR10.2 images with
/// `../source/domain_os/tools/sau_catalog.py`; bead domain-pascal-vzzh;
/// codegen-patterns 14).  dpc's default is the MC68010: the kernel build
/// that is checked byte for byte (SAU2) must not change under an option
/// the reference defaults differently (`-cpu mathlib_sr10`, p. 6-14, is
/// the reference's default and selects MC68020 code here).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Cpu {
    /// MC68010 instructions only: SAU2..SAU5.
    #[default]
    M68010,
    /// MC68020 instructions: scaled and base-displacement indexed
    /// operands, bit-field instructions, 32-bit multiply and divide:
    /// SAU6..SAU9 (codegen-patterns 14).
    M68020,
    /// The DSP160, DN460 and DN660 (`-cpu 160`/`460`/`660`, "These three
    /// arguments generate identical code", Table 6-3, p. 6-16): the
    /// MC68010 code, but for a longword `*` and `div` the extended
    /// multiply and divide instructions of those machines' bit-slice
    /// MC68010 emulators, in place of the M$ARITH calls: the per-SAU
    /// modules of the SAU4 kernel (codegen-patterns 14.1).
    Dn460,
}

impl Cpu {
    /// Whether MC68020 instructions may be generated.
    pub fn is_68020(self) -> bool {
        self == Cpu::M68020
    }

    /// Whether a longword `*` and `div` are instructions (MC68020
    /// `mul<s>.l`/`div<s>.l`, or the DN460's extended ones) rather than
    /// M$ARITH calls.  `mod` is an instruction only on the MC68020: the
    /// SAU4 kernel still calls `M$OI*` (TIME_$GET_ADJUST 0xF817486).
    pub fn long_mul_div(self) -> bool {
        matches!(self, Cpu::M68020 | Cpu::Dn460)
    }
}

/// The m68k target as used by the SR10.2 kernel (68010 instructions only,
/// CLAUDE.md 3.16), or with [`Cpu::M68020`] the SAU6..SAU9 kernels'
/// (`-cpu`, codegen-patterns 14).
#[derive(Debug, Default, Clone, Copy)]
pub struct M68k {
    pub cpu: Cpu,
}

impl M68k {
    /// The `link.w A6,#-n` size for a frame of `raw` bytes
    /// (`dpc_sema::Routine::frame_size`): the total rounded up to a
    /// multiple of 4.
    ///
    /// Evidence (codegen-patterns 2): all 1171 `link.w A6,#d` at routine
    /// starts named in the map have `d mod 4 = 0`; PROC1_$TST_LOCK
    /// 0xE148CA has `link #-4` for a 1-byte frame (its boolean result
    /// variable), and TIME_$Q_INIT_QUEUE 0xE16C5E / LOG_$READ 0xE17800
    /// have `link #0` with no locals.  **Uncertain:** whether the original
    /// rounded the total or gave each frame object a 4-byte-aligned slot;
    /// the two differ once a routine has several small locals.
    /// PKT_$NEXT_ID 0xE1248E favours the total: a word local at
    /// `(-2,A6)` and its word result variable in `link #-4` (codegen-
    /// patterns 2, "Frame size").
    pub fn link_size(raw: u32) -> u32 {
        raw.div_ceil(4) * 4
    }
}

impl Target for M68k {
    fn name(&self) -> &'static str {
        "m68k"
    }

    /// The MC68020 reads and writes words and longwords at odd addresses
    /// (SAU9 SCSI_WAIT 0x3C4778F2 `move.l (35,A5),(-116,A6)`; codegen-
    /// patterns 14, "Odd addresses").
    fn unaligned_access(&self) -> bool {
        self.cpu.is_68020()
    }

    fn pointer_size(&self) -> u32 {
        4
    }

    /// 16-bit values are pushed as 2 bytes; 8-bit values occupy the high
    /// byte of a 2-byte slot (CLAUDE.md 3.3; ../source/docs/pascal-abi.md).
    fn arg_slot_size(&self) -> u32 {
        2
    }

    /// A total that is not a multiple of 4 gets `subq.l #2,SP` before the
    /// pushes (CLAUDE.md 3.3).
    fn arg_area_align(&self) -> u32 {
        4
    }

    /// An 8-bit value pushed with `move.b x,-(SP)` lands in the HIGH
    /// (even-address) byte of its 2-byte slot: TIME_$Q_INIT_QUEUE 0xE16C62
    /// `move.b (0x8,A6),D0b` (codegen-patterns 1.3).
    fn small_arg_offset_in_slot(&self, _size: u32) -> u32 {
        0
    }

    /// Return address (4) + saved A6 (4): argument 1 is at `(8,A6)`
    /// (codegen-patterns 1.1; PROC1_$TST_LOCK 0xE148D4 `move.w (0x8,A6),D0w`).
    fn first_arg_offset(&self) -> i32 {
        8
    }

    /// Ref 5.5.7, p. 5-19: "four bytes or less in size (on 680x0
    /// workstations)".
    fn val_param_max_size(&self) -> u32 {
        4
    }

    /// The reference's names (p. 4-46): `_ISP__M68K` true "for 68000 code
    /// generation", `_ISP__A88K` false, `_BFMT__COFF` true "whenever the
    /// compiler is generating COFF" (we model the SR10 compiler, which
    /// "generates COFF files whenever it compiles", p. 4-46, although dpc
    /// itself writes gas assembly).  Plus the names
    /// Apollo's SR10.2 inserts test (`/sys/ins/base.ins.pas`), per the
    /// owner's ruling on reference question 21 (docs/reference-
    /// clarifications.md, "4.7 Predeclared conditional variables"):
    /// `isp_$m68k` true, `isp_$a88k` false, `obj_$coff` true
    /// (**provisional**: the owner is not sure the compiler predeclared
    /// it) and `obj_$bin` false (the pre-Domain/OS object format).
    fn predeclared_cond_vars(&self) -> &'static [(&'static str, bool)] {
        &[
            ("_ISP__M68K", true),
            ("_ISP__A88K", false),
            ("_BFMT__COFF", true),
            ("isp_$m68k", true),
            ("isp_$a88k", false),
            ("obj_$coff", true),
            ("obj_$bin", false),
        ]
    }

    /// M$ARITH, shipped as `runtime/m68k/m_arith.s` ([`runtime`]).
    fn runtime_library(&self) -> Option<&'static str> {
        Some(runtime::LIBRARY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_description() {
        let t: &dyn Target = &M68k::default();
        assert_eq!(t.name(), "m68k");
        assert_eq!(t.pointer_size(), 4);
        assert_eq!(t.arg_slot_size(), 2);
        assert_eq!(t.arg_area_align(), 4);
        assert_eq!(t.small_arg_offset_in_slot(1), 0);
        assert_eq!(t.first_arg_offset(), 8);
        assert_eq!(t.val_param_max_size(), 4);
        assert_eq!(t.runtime_library(), Some("libm_arith.a"));
        let vars = t.predeclared_cond_vars();
        assert_eq!(vars.len(), 7);
        for (n, v) in [
            ("_ISP__M68K", true),
            ("_ISP__A88K", false),
            ("_BFMT__COFF", true),
            ("isp_$m68k", true),
            ("isp_$a88k", false),
            ("obj_$coff", true),
            ("obj_$bin", false),
        ] {
            assert!(vars.contains(&(n, v)), "{n}");
        }
    }

    #[test]
    fn link_size_rounds_total_to_4() {
        assert_eq!(M68k::link_size(0), 0);
        assert_eq!(M68k::link_size(1), 4);
        assert_eq!(M68k::link_size(4), 4);
        assert_eq!(M68k::link_size(6), 8);
    }
}
