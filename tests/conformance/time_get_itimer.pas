{ TIME_$GET_ITIMER, 0xE58F06, 0x5E bytes: 0xE58F06..0xE58F64
  (tools/imgdis.sh E58F06 5E), a fragment of TIME_UNWIRED's code
  segment.  Semantics: ../source domain_os/time/get_itimer.c; the
  module-locals it calls, 0xE58C74 (time/get_itimer_internal.c) and
  0xE58C02 (time/clock_to_itimer.c), are outside the unit
  (`module-routines=', called with `bsr.w').

  Patterns exercised (docs/codegen-patterns.md):
  - 3    calls of a function whose 6-byte record result goes through a
         hidden address ("Record results over 4 bytes"): the argument's
         address, then the frame temporary's, pushed last (`pea (A3); pea
         (-0x8,A6); bsr.w 0xE58C02; addq.w #8,SP', 0xE58F30..0xE58F3A),
         and the record copied from the temporary into the destination,
         here the argument itself (`move.l (-0x8,A6),(A3); move.w
         (-0x4,A6),(0x4,A3)'); one temporary for both calls, -8 (a
         6-byte record in a longword place, `link #-8');
  - 1.1  the second call not popped: only the copy and the epilogue
         follow (0xE58F4C);
  - 12.2 the three by-address parameters held in A2..A4 from the entry.

  Reconstructed (not known from the original): the names and headings
  (../source's time/get_itimer.pas, get_itimer_internal.pas,
  clock_to_itimer.pas); time_$clock_t as a plain record of a longword and
  a word (../source's base.ins.pas has variants). }
module time_unwired;
type
  time_$clock_t = record
    high: integer32;
    low: 0..65535;
  end;

procedure time_$get_itimer_internal(which: integer;
                                    out interval: time_$clock_t;
                                    out value: time_$clock_t);
  internal; extern;
function time_$clock_to_itimer(in clock: time_$clock_t): time_$clock_t;
  internal; extern;

procedure time_$get_itimer(in which: integer;
                           in out interval: time_$clock_t;
                           in out value: time_$clock_t);
begin
  time_$get_itimer_internal(which, interval, value);
  if which = 1 then begin
    interval := time_$clock_to_itimer(interval);
    value := time_$clock_to_itimer(value);
  end;
end;
