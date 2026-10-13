{ TONE_$TIME, 0xE172FC, the whole TONE module (map `I E172FC TONE size =
  64': 0x5E bytes of code and its three constant cells; tools/imgdis.sh
  E172FC 64).  Semantics: ../source/domain_os/tone/time.c.

  Patterns exercised (docs/codegen-patterns.md):
  - 1.2  a value parameter under the default convention (bead
         domain-pascal-5aw.12; ref 5.2 Table 5-1, p. 5-5): the 6-byte
         clock passed by address and copied on entry, `movea.l (8,A6),A0;
         move.l (A0),(-8,A6); move.w (4,A0),(-4,A6)' (0xE17300..0xE17308,
         the form of all six 6-byte copies), then passed on by address as
         the copy, `pea (-8,A6)' (0xE1732C);
  - 2    the copy at the top of the frame, the status local below it,
         `link #-12';
  - 1.5  constants passed by address: the boolean cells `ff00' and
         `0000' for TONE_$ENABLE and the word 0 for TIME_$WAIT, after the
         `rts' (0xE1735A..0xE1735F);
  - 1.4, 1.1  padded word pushes to PROC1_$SET_LOCK / PROC1_$CLR_LOCK, no
         pop after the last call.

  Reconstructed (not known from the original): every name; the clock as
  a record of a longword and a word (6 bytes, ../source's `clock_t');
  `in' for the parameters TONE_$ENABLE and TIME_$WAIT only read; the
  value 0 of TIME_$WAIT's first parameter (a relative wait in the C). }
module tone;
type
  status_$t = integer32;
  time_$clock_t = record high: integer32; low: integer; end;

procedure proc1_$set_lock(lock_id: integer); options(val_param, extern);
procedure proc1_$clr_lock(lock_id: integer); options(val_param, extern);
procedure tone_$enable(in enable: boolean); extern;
procedure time_$wait(in rel_abs: integer; in duration: time_$clock_t;
                     out status: status_$t); extern;

procedure tone_$time(duration: time_$clock_t);
var status: status_$t;
begin
  proc1_$set_lock(14);
  tone_$enable(true);
  time_$wait(0, duration, status);
  tone_$enable(false);
  proc1_$clr_lock(14);
end;
