# Image conformance tests

This is the main gate (CLAUDE.md section 6, item 3).  Each test is a Pascal
version of a real routine from the SR10.2 SAU2 kernel.  The harness:

1. compiles and assembles it;
2. links it standalone at the routine's image address;
3. compares the bytes with the image.

Relocations are matched symbolically, as `../source`'s `asm_compare.py`
does.  The harness keeps a table of external symbols and their image
addresses, taken from the SAU2 link map.

The first candidates are listed in CLAUDE.md section 6 (PROC1_$TST_LOCK,
TIME_$Q_INIT_QUEUE, PROC1_$GET_LOADAV, AST_$PAGE_ZERO, LOG_$READ).
Compiled so far: `proc1_tst_lock.pas` (PROC1_$TST_LOCK and
PROC1_$GET_LOCKS, 0xE148CA..0xE148F8), `time_q_init_queue.pas`
(TIME_$Q_INIT_QUEUE and TIME_$Q_FLUSH_QUEUE, 0xE16C5E..0xE16C8E),
`proc1_get_loadav.pas` (PROC1_$GET_LOADAV, 0xE14BBA..0xE14BDA),
`proc2_get_boot_flags.pas` (PROC2_$GET_BOOT_FLAGS, 0xE41B56..0xE41B72)
and `ast_page_zero.pas` (the module-local routine at 0xE00EB0 and
AST_$PAGE_ZERO, 0xE00EB0..0xE00F16: calls, constant arguments, a `dbf`
`for` loop; `handasm/ast_page_zero.s` is the same unit transcribed by
hand) and `name_convert_acl_status.pas` (NAME_CONVERT_ACL_STATUS,
0xE5861C..0xE58656: an `if ... else if` chain, a by-address parameter
held in A0) and `sio_init_dtte.pas` (the module-local routine at
0xE32B76, `sio/init_dtte.c`, 0xE32B76..0xE32BB8: parameters held across
calls in A2 and D2 by the register allocator, codegen-patterns 12.4)
and `proc1_set_asid.pas` (PROC1_$SET_ASID, 0xE148F8..0xE1491E: a
parameter whose last use is a call's argument, held through the call in
D2, codegen-patterns 12.4 R4(c)).  Expressions (codegen-patterns 5.2, 8,
3; beads domain-pascal-5aw.13, -d7i): `rip_packet_length.pas` (the
module-local routine at 0xE68864, `rip/server.c`'s RIP_$PACKET_LENGTH:
`* 6` with shifts and adds, `+ 2` with `addq`), `bat_get_bat_step.pas`
(BAT_$GET_BAT_STEP 0xE3BB58: a parameter index copied and scaled with
`mulu.w`, `lea (0,A5,D1.w)`), `fim_get_fim_addr.pas` (FIM_$GET_FIM_ADDR
0xE0AA04: `lsl.w #2`, `(0x3c,A5,D0.w)`) and
`audit_is_process_audited.pas` (AUDIT_$IS_PROCESS_AUDITED 0xE70D94: a
1-based array, `(-2,A5,D0.w)`, a comparison as the result with `seq`).
Status out-parameters (codegen-patterns 9, 12.4 R4(b); bead
domain-pascal-5aw.2): `iic.pas` (IIC_$ACQUIRE..IIC_$RECEIVE_CHECK,
0xE70A56..0xE70AF4, the IIC module's status stubs without IIC_$INIT, a
bare `rts`: by-address slots used once in A0, A1, A2, a status store
then a constant boolean result), `xns_idp_get_stats.pas`
(XNS_IDP_$GET_STATS 0xE18FD6: the status slot in A1 after the dead A0)
and `io_alloc.pas` (IO_$ALLOC 0xE1A9C2: a status cleared, then `clr.l
D0`).
Each `.pas` starts with a comment
naming the routines, their addresses and the codegen-patterns sections
they exercise.

`pending/` holds reconstructions that `dpc` cannot compile yet and that
are therefore not in the manifest: `pending/log_read.pas` (LOG_$READ
and its worker 0xE17778; what is missing is in codegen-patterns 12.3).

## Module data

A unit with static variables gets one zero-filled data area, the NOBITS
section `.bss.moddata.<UNIT>` (codegen-patterns 4, 11).  Its routines
load A5 with a relocated `lea (.Ldata).l,%a5`.  The manifest places the
section at the module's `D` segment of the map with
`--section-addr .bss.moddata.<UNIT>=0xADDR`; `tools/conform.py` then
checks that relocation against that address like any other.  A NOBITS
section has no bytes to compare; a reference into a data section with no
`--section-addr` is a failure.  Only the variables a unit needs are
declared, at the offsets the image uses (each `.pas` header says so).

## Fragments and 0x2048 alignment

The compiler aligns loop heads to 4 bytes with the no-op
`movea.l A0,A0` (0x2048), and pads the end of each module's code to a
multiple of 4 with it (codegen-patterns 7).  The alignment is relative to
the module's code base, which is 4-aligned, so it is absolute.  gas aligns
relative to the section start.  A conformance unit is usually a *fragment*
of a module, and may start at 2 mod 4 (PROC1_$TST_LOCK 0xE148CA).  So:

- `make conform` compiles each `.pas` unit with
  `dpc --fragment-origin 0xLO`, LO being the unit's first address.  dpc
  then emits no module-end padding, records LO in the absolute local
  symbol `__dpc_fragment_origin`, and starts the code section with
  `LO mod 4` bytes of `.space`, so that `.balignw 4,0x2048` lands on
  absolute 4-byte boundaries.  A unit that is a whole map module is
  compiled without the option.  One that ends its module without starting
  it also gets `--fragment-ends-module`, which keeps the module-end
  padding (in the image it belongs to the unit's last routine).  A unit
  that runs past the end of its module is refused: one `.pas` is one
  module.
- `tools/conform.py` sees the symbol (it must equal LO), links the first
  code section at `LO & ~3`, checks that the leading pad bytes are zero
  and leaves them out of every other comparison.  Every other check (bytes, relocations, globals at their map
  addresses, literal addresses) works on absolute addresses and is
  unchanged.  `.s` files without the symbol are linked at LO as before.
- A fragment may have only one code section (dpc rejects more): where a
  second one would start in the module is not known.

## Loop alignment on and off

In the image, loop alignment is on for a leading run of the routines of a
map segment and off for the rest (codegen-patterns 7): PMAP_ and PROC1_
never align, AST_ stops after the module-local routine at 0xE01502,
VTOC_ at VTOC_$DISMOUNT, RING at RING_$SVC_CLOSE, XNS_IDP at
XNS_IDP_$OS_OPEN.  dpc reproduces this with options, which a manifest
line passes as tokens after its specs (`.pas` units only):

- `loop-align=on` (the default) / `loop-align=off`: `dpc --loop-align=...`;
- `loop-align-off-from=NAME`: `dpc --loop-align-off-from=NAME`, on for
  the unit's routines before routine NAME (its Pascal name or symbol, any
  case) and off from it on.  A name that is not a routine of the unit is
  an error.

A routine that does not align gets no `0x2048` before its loop heads.
The end of a module, or of a unit compiled with `--fragment-ends-module`,
is padded with `0x2048` if its last routine aligns and with zero words if
not (`rts; 0x0000`, e.g. AST_ 0xE0736E).

    python3 tools/loopalign.py switch SEG...

prints, for map segment SEG, the class of each routine (`F` filler, `u`
loop head at 2 mod 4 without one, `X` both, `a` only 4-aligned heads,
`.` no loop), the last aligning routine, the first non-aligning one after
it, and the routines between them, which cannot be classed (a routine
the map does not name is shown as `module-local`).  An entry's
option must put the switch in that interval.  A segment where a
non-aligning routine comes before an aligning one is reported as an
error (exit status 1); the only one is MMAP_ (MMAP_$WS_SCAN 0xE0D364
before MMAP_$GET_IMPURE 0xE0D5EA; codegen-patterns 7 explains it).  A
unit with no loop and no module end needs no option.  No conformance
unit uses these options yet: the non-aligning loops and module tails in
the image all need constructs dpc does not compile yet.

`handasm/pkt_recently_missing.s` (PKT_$RECENTLY_MISSING 0xE128BA, a loop
with its filler at 0xE128DA) tests this on a routine that starts at
2 mod 4.

Scratch files go to `tmp/conform/<unit path>/`, and each `OK` line ends
with the unit's file.

The harness is `tools/conform.py` (map reader: `tools/sau2map.py`).  Units
are listed in `manifest.txt` and run with `make conform`; one unit can be
checked with

    python3 tools/conform.py check FILE.s --routine NAME [--addr 0xADDR[:LEN]]

`handasm/` holds harness fixtures transcribed by hand from the image.  They
test the harness itself and are **not** compiler output.
