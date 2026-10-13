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
`time_q_init_queue.pas` starts at TIME_$Q_INIT 0xE16C5C, an empty
procedure (codegen-patterns 2, bead domain-pascal-lkr), and
`mt_release.pas` is the whole one-routine MT module (MT_$RELEASE
0xE70A4C: `rts` and the module-end 0x2048).  Which routines load A5
(codegen-patterns 4, bead domain-pascal-5aw.14; "Module data" below):
`smd_busy_wait.pas` (SMD_$PUTC, SMD_$BUSY_WAIT, SMD_$LITES,
0xE1D89C..0xE1D8DC, the tail of the SMD module: always mode from
SMD_$BUSY_WAIT, a val_param call), `mmap_alloc_contig.pas`
(MMAP_$ALLOC_CONTIG 0xE0D8F0) and `xns_idp_get_port_info.pas`
(XNS_IDP_$GET_PORT_INFO 0xE18FB8), each always mode for itself, and
`rem_name_register_server.pas` (REM_NAME_$REGISTER_SERVER 0xE4A4AE: its
data addressed with `lea <block>.l,A0`, no A5).
Status out-parameters (codegen-patterns 9, 12.4 R4(b); bead
domain-pascal-5aw.2): `iic.pas` (IIC_$INIT..IIC_$RECEIVE_CHECK,
0xE70A54..0xE70AF4, the whole IIC module: IIC_$INIT an empty procedure,
a bare `rts`, then status stubs with by-address slots used once in A0,
A1, A2, a status store then a constant boolean result), `xns_idp_get_stats.pas`
(XNS_IDP_$GET_STATS 0xE18FD6: the status slot in A1 after the dead A0)
and `io_alloc.pas` (IO_$ALLOC 0xE1A9C2: a status cleared, then `clr.l
D0`).  Interrupt built-ins (codegen-patterns 10, 12.5; beads
domain-pascal-5aw.7, -0wc): `time_get_adjust.pas` (TIME_$GET_ADJUST
0xE16AA8, the last routine of TIME_: `disable`/`enable` around a read
into a local held in D1 across two M$ARITH calls, `link #-4`).  Loops
(codegen-patterns 7.4; bead domain-pascal-5aw.15): `disk_unassign_all.pas`
(DISK_$UNASSIGN_ALL 0xE6BE1C: a `for` loop with constant bounds, its
index in D3 beside the `dbf` count in D2 across a `bsr.w` to
DISK_$UNASSIGN, a routine of the module outside the unit; "Routines
outside the unit" below) and `disk_set_chain_status.pas` (the DISK_
module-local 0xE3CCCE: a `while` loop, its test at the bottom, the walk
pointer held in A1; loop alignment off).  Function results
(codegen-patterns 3, 12.5; beads domain-pascal-5aw.11, -8aj):
`pkt_next_id.pas` (PKT_$NEXT_ID 0xE1248E: the result variable assigned
before a call, held in D2 and moved to D0 at the end; the spin-lock token
stored to its slot and pushed from D0) and `vfmt_get_next_arg.pas` (the
VFMT module-local 0xE6AA38: a pointer result read from memory, stored
into the result variable's slot and loaded into A0).  Callers of
pointer functions (codegen-patterns 3, 12.4 R4(a); beads
domain-pascal-ae9, -3h9): `mst_maps.pas` (MST_$MAP, MST_$MAP_TOP,
MST_$MAP_GLOBAL, MST_$MAPS, 0xE4386C..0xE439D4: each a call of the
unnamed module-local 0xE43182 whose A0 is stored into the result slot
and left in A0; by-address slots read once in one argument list in A0,
A1, A2, A3, A4) and `disk_get_block.pas` (DISK_$GET_BLOCK 0xE3BB80: a
pointer result variable held in A2 across ML_$UNLOCK, `movea.l A0,A2`
/ `movea.l A2,A0`).  Pointer locals assigned from pointer functions
(codegen-patterns 3, 12.5; bead domain-pascal-500): `cal_shutdown.pas`
(CAL_$SHUTDOWN 0xE3E6E4: `movea.l A0,A3`, fields through A3, `pea
(A3)` as DBUF_$SET_BUFF's last argument, held through it with the
status address in A2 in a body with a branch).
Each `.pas` starts with a comment
naming the routines, their addresses and the codegen-patterns sections
they exercise.

`pending/` holds reconstructions that `dpc` cannot compile yet and that
are therefore not in the manifest: `pending/log_read.pas` (LOG_$READ
and its worker 0xE17778; what is missing is in codegen-patterns 12.3).

Constants and expressions passed by address (codegen-patterns 1.5; bead
domain-pascal-5aw.9; "Constants in the code section" below):
`term_send_kbd_string.pas` (TERM_$SEND_KBD_STRING 0xE1AAFC: one word
cell for two arguments, `pea (d,PC)` then `move.l (SP),-(SP)`),
`mst_maps.pas` (now with MST_$REMAP 0xE439D4), `mst_set_touch_ahead_cnt.pas`
(MST_$SET_TOUCH_AHEAD_CNT 0xE446FC), `sysbus_define_int.pas`
(SYSBUS_$DEFINE_INT 0xE0ABBC: a longword cell after a 0x2048 filler),
`pchist_interrupt.pas` (PCHIST_$INTERRUPT 0xE1A1F6), `file_set_len.pas`
(FILE_$SET_LEN 0xE73F90), `asknode_info.pas` (ASKNODE_$INFO 0xE64598:
word cell before longword cell), `proc2_quit.pas` (PROC2_$QUIT
0xE3F130: word, filler, longword) and `tone_enable.pas` (TONE_$ENABLE
0xE1ACE8: `addr(...)` through a frame temporary).

Nested routines (codegen-patterns 6; bead domain-pascal-5aw.18; "Nested
routines" below): `ring_discard_packet.pas` (the RING nested routine at
0xE76470, nested in the module-local 0xE7649E: the dynamic link in A2
across a call, an uplevel by-address parameter dereferenced).

Calls through routine pointers (codegen-patterns 1.6; bead
domain-pascal-5aw.8): `atbus_ring_interrupt.pas` (RING_INTERRUPT
0xE0AB0C: a `val_param` routine pointer in the unit's data, its argument
pushed by value from `(4,A5)`, then `movea.l (A5),A0; jsr (A0)`, not
popped before `unlk`).

Value parameters under the default convention (codegen-patterns 1.2,
"Value parameters passed by address"; bead domain-pascal-5aw.12):
`tone_time.pas` (TONE_$TIME 0xE172FC, the whole TONE module: a 6-byte
clock copied on entry with `move.l (A0); move.w (4,A0)` and passed on as
the copy, `pea (-8,A6)`) and `smd_set_tp_cursor.pas` (SMD_$SET_TP_CURSOR
0xE6E964: two copies through A0 and A1 pushed by value from the frame,
the `in` slot then in A2).

Instruction order (codegen-patterns 12.4 R9-R11; bead
domain-pascal-g5q.8): every unit is compiled with the scheduler on (`dpc
--schedule`, default `r11:3`) and none changed.  `xpd_reset_ptrace_opts.pas`
(XPD_$RESET_PTRACE_OPTS 0xE5B156: five stores through A0 in the image's
field order, which the scheduler does not change).

## Routines outside the unit

A unit is usually part of a module, and its routines may call a routine
of the module that the unit does not contain.  The original compiled
the whole module and called it with `bsr` (codegen-patterns 1.6); the
unit declares it `extern`, which alone would give `jsr abs.l`.  The
manifest token

- `module-routines=NAME,...` (`.pas` fragments only, at most once)

names such routines; `tools/conform.py` looks each up in the map (it
must lie in the unit's map module and outside the unit) and passes `dpc
--module-routine=NAME=0xADDR`.  dpc then calls it with `bsr`, sized from
the address as for a routine of the unit, and the reference is resolved
to the map name at link time like any other (DISK_$UNASSIGN_ALL 0xE6BE34
`bsr.w DISK_$UNASSIGN`, `disk_unassign_all.pas`).

A module-local routine the map does not name (bead domain-pascal-3h9)
has no symbol to look up.  The unit declares it under a reconstructed
name with `internal; extern` (`internal` gives the by-value convention
of a routine of the same module, ref 5.2 p. 5-4, and a local symbol;
`extern`, that its body is elsewhere), and the manifest item is
`NAME=0xADDR`, NAME being dpc's symbol for it (the Pascal name
upper-cased):

    tests/conformance/mst_maps.pas  ...  module-routines=ALLOC_SEGS=0xE43182

`tools/conform.py` checks that ADDR lies in the unit's map module and
outside the unit, that the map has no name at ADDR (give that name
instead) and that NAME is not a map name; passes `dpc
--module-routine=NAME=0xADDR`; and links the unit with `--defsym
NAME=0xADDR`, so the `bsr` displacement is checked against the image
like any relocation (a wrong address fails).  A NAME the object does
not reference is a setup error.  `conform.py check` takes the same as
`--local-sym NAME=0xADDR`.  This is test scaffolding only: the reference allows
calls of an `internal` routine only within its compilation unit (5.2
p. 5-4), so dpc rejects a call of an `internal` routine with no body in
the unit unless `--module-routine` names it; in a whole-module
translation the body is in the unit.  NAME is upper-cased (dpc's
symbols are).

A `bsr.s` to a routine outside the unit is written by dpc as its
encoding, `.short 0x61dd | bsr.s NAME` (the displacement from the
address `--module-routine` gives): gas cannot relocate an 8-bit
displacement to a symbol it does not define (MST_$REMAP 0xE439FC `610e`
to MST_$REMAP_PRIVI, TERM_$SEND_KBD_STRING 0xE1AB20 `6186` to the
module-local 0xE1AAA8).  The call then names its routine only in that
comment; `tools/conform.py` counts it as a reference of a module-local
NAME, and the byte comparison checks the address.

## Nested routines

A nested routine comes before its enclosing routine (codegen-patterns 6,
"Order"), so a family is contiguous: its nested routines, then the parent.
Every family in the image is large (`python3 tools/nestsurvey.py
families`, smallest extent 0x112), and every parent needs more than `dpc`
has (codegen-patterns 6, "Conformance").  A unit may therefore be the
nested routines of a parent without the parent, with the manifest token

- `enclosing=NAME` (`.pas` fragments only, at most once):

NAME is `dpc`'s symbol for the enclosing routine (its Pascal name
upper-cased; the map name if it has one).  The `.pas` holds the
enclosing routine with the heading and the locals the nested code reaches
(their frame offsets are visible in it, so they must be reproduced) and a
stand-in body (its header says so).  `dpc` emits it after the nested
routines; `tools/conform.py` checks that the symbol NAME lands at the
unit's end, which must be where the map has NAME (or no name, for a
routine the map does not name), and leaves its bytes and relocations out
of every comparison.  `conform.py check` takes the same as `--enclosing
NAME`.  `ring_discard_packet.pas` (the RING nested routine at 0xE76470,
before the module-local 0xE7649E) is such a unit.

## Constants in the code section

A constant passed by address is a cell in the code section after the
routine that uses it first (codegen-patterns 1.5; bead
domain-pascal-5aw.9), inside that routine's map extent (MST_$REMAP's
0x38 bytes end with its cell 0xE43A0A), so a unit's last routine
carries its cells and nothing extra is needed: the module-end padding,
for a module-tail unit, follows them (`term_send_kbd_string.pas`,
`sysbus_define_int.pas`).  A routine that names a cell placed after an
earlier routine of its module (305 sites in the image, e.g.
SMD_$INVERT_S 0xE6DDE4) cannot be a unit without that routine: dpc
would place a cell of its own.  There is no harness support for that
yet; such routines are left out.

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

Which routines load A5 is not recorded in the source (codegen-patterns
4): in "used mode", dpc's default, an exported routine loads it iff it
uses the unit's data (directly or through `internal` callees that
inherit A5); in "always mode" every exported routine with a frame does.
A manifest line passes dpc's options as tokens after its specs (`.pas`
units only, each at most once):

- `a5-always-from=NAME,...`: `dpc --a5-always-from=NAME,...`, always
  mode from routine NAME on within its code section, one NAME per code
  section, as `loop-align-off-from=`;
- `a5-always=NAME,...`: `dpc --a5-always=...`, these routines load A5
  whatever their body (`internal` ones too);
- `a5-absolute=NAME,...`: `dpc --a5-absolute=...`, these exported
  routines load no A5 and address the unit's data with `lea
  <block>.l,A0` (REM_NAME_$REGISTER_SERVER 0xE4A4B2).

A name that is not a routine of the unit, or a routine in both lists, is
dpc's error (a setup error here).  Always mode needs a data area: a unit
that has no variables of its own but loads A5 in the image declares a
stand-in variable (its header says so); the image's 4-byte blocks with
A5 at their end are not covered.

    python3 tools/a5survey.py switch SEG...

prints a segment's routines by mode (`S` loads and uses A5, `s` loads it
without using it, `n` neither) and where the switch to always mode lies.

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

## Named sections

A unit whose module heading or routines name sections (codegen-patterns
11; ref 7.1.1, 5.7.1) gets one output section per Pascal section:
`.text.<UNIT>.<SECTION>` for code, `.bss.moddata.<UNIT>.<SECTION>` for
its data areas (the defaults stay `.text` and `.bss.moddata.<UNIT>`).
The manifest places a data area as before, with `--section-addr`.  A
code section named in `--section-addr` is another code section of the
unit's module, a segment of another output section of the map: the
harness links it at that address in an output section of its own and
compares it whole (bytes, relocations, its globals at their map
addresses, literal addresses; the address must start a code segment of
the map and the section be exactly its length), and prints `OK section
NAME 0xADDR LEN`.
The unit's routine specs are in the code section without
`--section-addr`.  Each code section is a whole map segment (a fragment
has one code section).  `scsi.pas` is the whole SCSI_ module:
SCSI_$CINIT in OS_INIT_PROC (0xE34F04), SCSI_$FREE_ASID in
SCSI_WIRED_PROC (0xE88800), SCSI_$PRESENT in SCSI_WIRED_DATA (0xE88804).
`proc1_get_loadav.pas` and `smd_busy_wait.pas` name their real sections
(WIRED_PROC, WIRED_DATA); their bytes are those of the default
sections.

## Loop alignment on and off

In the image, loop alignment is on for a leading run of the routines of a
map segment and off for the rest (codegen-patterns 7): PMAP_ and PROC1_
never align, AST_ stops after the module-local routine at 0xE01502,
VTOC_ at VTOC_$DISMOUNT, RING at RING_$SVC_CLOSE, XNS_IDP at
XNS_IDP_$OS_OPEN.  dpc reproduces this with options, which a manifest
line passes as tokens after its specs (`.pas` units only):

- `loop-align=on` (the default) / `loop-align=off`: `dpc --loop-align=...`;
- `loop-align-off-from=NAME,...`: `dpc --loop-align-off-from=NAME,...`,
  on for the routines of NAME's code section before routine NAME (its
  Pascal name or symbol, any case) and off from it on; one NAME per code
  section, as each map segment has its own switch.  A name that is not a
  routine of the unit, or two in one code section, is an error.

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
unit with no loop and no module end needs no option.
`disk_set_chain_status.pas` (the DISK_ module-local 0xE3CCCE, a `while`
head at 2 mod 4 without the filler) uses `loop-align=off`.

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
