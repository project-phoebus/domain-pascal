{ TIME_$GET_ADJUST, 0xE16AA8, 0x40 bytes: 0xE16AA8..0xE16AE8, the last
  routine of the TIME_ module (tools/imgdis.sh E16AA8 40).  Semantics:
  ../source/domain_os/time/get_adjust.c.  Externals (SAU2 map):
  TIME_$CURRENT_DELTA 0xE2B0FC (TIME_ASM), M$DIS$LLL 0xE0ACD8, M$OIS$LLL
  0xE0AD7C (M$ARITH).

  The unit is a fragment of the TIME_ module, so tools/conform.py compiles
  it with `dpc --fragment-origin 0xE16AA8 --fragment-ends-module'.  The
  module is in "used mode" for A5 (tools/a5survey.py switch TIME_), and
  the routine has no loop.

  Patterns exercised (docs/codegen-patterns.md):
  - 10   `disable' as `ori.w #0x700,SR' (0xE16AB0) and `enable' as
         `andi.w #0xF8FF,SR' (0xE16ABA) around one read;
  - 12.5 a local held in a register: `current_delta', assigned once and
         read by later statements, is loaded into D1 (0xE16AB4), never
         stored; its 4-byte frame slot is still allocated (`link.w
         A6,#-4', 0xE16AA8);
  - 12.2 the by-address `delta' slot, used twice, loaded at entry into A1
         (0xE16AAC), before the `disable';
  - 8    `div'/`mod' of a longword by 250000 as M$DIS$LLL / M$OIS$LLL
         (0xE16AC6, 0xE16AD8), D1 and A1 live across them (12.4 R3);
         `* 4' as `lsl.l #2,D0' (0xE16ADE); the second call not popped
         before `unlk' (1.1).

  The record type name is ours (the C passes `int32_t *delta'); the
  parameter mode is `out' (written only). }
module time_;
type
  time_$timeval_t = record
    sec: integer32;     { 0 }
    usec: integer32;    { 4 }
  end;
var
  time_$current_delta: extern integer32;

procedure time_$get_adjust(out delta: time_$timeval_t);
var
  current_delta: integer32;
begin
  disable;
  current_delta := time_$current_delta;
  enable;
  delta.sec := current_delta div 250000;
  delta.usec := (current_delta mod 250000) * 4;
end;
