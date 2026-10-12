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
hand).  Each `.pas` starts with a comment
naming the routines, their addresses and the codegen-patterns sections
they exercise.

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
