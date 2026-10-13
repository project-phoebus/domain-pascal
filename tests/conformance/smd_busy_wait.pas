{ SMD_$PUTC, 0xE1D89C, 2 bytes, SMD_$BUSY_WAIT, 0xE1D89E, 0x1A bytes, and
  SMD_$LITES, 0xE1D8B8, 0x24 bytes: 0xE1D89C..0xE1D8DC, the tail of the
  SMD module (map `I E1D750 SMD size = 18C'; tools/imgdis.sh E1D89C 40).
  Semantics: ../source/domain_os/smd/putc.c (a stub), smd/busy_wait.c
  (clears the status), smd/lites.c (passes both arguments to DISP_LITES,
  map `D E1E9F4 DISP_LITES').

  A fragment compiled with `dpc --fragment-origin 0xE1D89C
  --fragment-ends-module'; the module-end 0x2048 at 0xE1D8DA belongs to
  SMD_$LITES (the unit has no loop; alignment is on by default).  The
  module heading names the map's sections (codegen-patterns 11), so the
  code is `.text.SMD.WIRED_PROC' and the data area
  `.bss.moddata.SMD.WIRED_DATA', placed at the map's `D E2E060 SMD'
  (manifest --section-addr); with the default sections the bytes are
  the same.

  The segment switches to always mode at SMD_$BUSY_WAIT (codegen-patterns
  4; `tools/a5survey.py switch SMD', segment 0xE1D750:
  SMD_$COPY_FONT_TO_MD_HDM `n', SMD_$BUSY_WAIT and SMD_$LITES `s'), so the
  manifest passes `a5-always-from=SMD_$BUSY_WAIT': both load A5 although
  neither uses the data.

  Patterns exercised (docs/codegen-patterns.md):
  - 2    SMD_$PUTC, an empty procedure: a bare `rts' (0xE1D89C);
  - 4    always mode: `pea (A5); lea 0xE2E060,A5' ... `movea.l (-4,A6),A5'
         in routines that do not use the data (0xE1D8A2, 0xE1D8BC);
  - 9    `status.all := status_$ok' as `movea.l (0xc,A6),A0; clr.l (A0)'
         (0xE1D8AA);
  - 1.2, 1.3, 1.1  val_param words pushed right to left from their
         slots, `move.w (0xa,A6),-(SP)', `move.w (8,A6),-(SP)', 4 bytes
         so no pad, `jsr' to another unit, no pop before `unlk'
         (0xE1D8C4..0xE1D8CC).

  Reconstructed (not known from the original): SMD_$PUTC's parameter (an
  empty procedure's are not visible; the C has one 16-bit `c', declared
  here `in c: integer'); the names and types of the other parameters
  (from ../source; SMD_$BUSY_WAIT's first, never read, as `in
  integer32' for its 4-byte slot); val_param on SMD_$LITES and
  DISP_LITES (the image pushes the words by value); `smd_$data_stand_in',
  a variable only so that the unit has a data area for A5 to point at
  (none of these routines reads the SMD data; dpc requires data for
  always mode). }
module smd, wired_proc, wired_data;
const
  status_$ok = 0;
type
  status_$t = record
    all: integer32;
  end;
var
  smd_$data_stand_in: integer32;   { stand-in, see above }

procedure disp_lites(pattern: integer; y_pos: integer); options(val_param, extern);

procedure smd_$putc(in c: integer);
begin
end;

procedure smd_$busy_wait(in param1: integer32; out status: status_$t);
begin
  status.all := status_$ok;
end;

procedure smd_$lites(pattern: integer; y_pos: integer); options(val_param);
begin
  disp_lites(pattern, y_pos);
end;
