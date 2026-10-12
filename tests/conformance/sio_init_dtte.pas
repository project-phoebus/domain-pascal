{ Module-local routine at 0xE32B76, 0x42 bytes: 0xE32B76..0xE32BB8
  (tools/imgdis.sh E32B76 42).  The map does not name it; the C calls it
  SIO_$INIT_DTTE.  Semantics: ../source/domain_os/sio/init_dtte.c;
  layout: dtte_t in ../source/domain_os/term/term.h (discipline 0x34,
  flags 0x36) and ec_$eventcount_t in ../source/domain_os/ec/ec.h (12
  bytes).  External (SAU2 map): EC_$INIT 0xE151FE.

  The unit is a fragment of the OS_TERM_INIT segment, so tools/conform.py
  compiles it with `dpc --fragment-origin 0xE32B76'.  It has no loop.

  Patterns exercised (docs/codegen-patterns.md):
  - 12.4 R1/R3 (bead domain-pascal-g5q.5): the by-address slot `dtte'
         (five uses) and the by-value word `discipline' are loaded at
         entry, in parameter order (12.2), and both are live across the
         calls, so they take the lowest callee-saved registers of their
         classes, A2 and D2 (0xE32B7E, 0xE32B82);
  - 2    the save of D2/A2 with `movem.l', the restore from (-0xc,A6);
  - 1.1  `addq.w #4,SP' after the first two calls; none after the last,
         which only stores follow;
  - 1.5/1.6 `pea (d,A2)' / `pea (A2)' for an object reached through the
         held slot, `jsr (EC_$INIT).l';
  - 13   `clr.b' for `false', `move.w D2,(0x34,A2)' from the held value;
         a byte and the word below it stored separately (3 bytes cannot
         be one reference, ref 3.15.3 p. 3-57).

  Reconstructed (not known from the original): the routine, type, field
  and parameter names; `flags' as a boolean (the C has a uint8_t set to
  0; only its width and the value 0 matter here); the filler arrays;
  `internal' (ref 5.5.4): the routine has no map name and takes its word
  by value (its callers push `move.w #2' / `clr.w', 0xE32D86 and
  0xE32E56 per the C header).

  The frame: `link #-4', never touched (codegen-patterns 2, "Untouched
  frames").  What the original declared is not known; `unknown' stands in
  for the 4 bytes. }
module sio_dtte;
type
  ec_$eventcount_t = record
    value: integer32;
    waiter_list_head: univ_ptr;
    waiter_list_tail: univ_ptr;
  end;
  dtte_t = record
    ec: ec_$eventcount_t;            { 0x00 }
    input_ec: ec_$eventcount_t;      { 0x0C }
    output_ec: ec_$eventcount_t;     { 0x18 }
    reserved: array[1..4] of integer32;  { 0x24..0x34 }
    discipline: integer;             { 0x34 }
    flags: boolean;                  { 0x36 }
  end;

procedure ec_$init(var ec: ec_$eventcount_t); extern;

procedure init_dtte(var dtte: dtte_t; discipline: integer); internal;
var
  { Stand-in, not from the original: fills the frame to `link #-4'. }
  unknown: integer32;
begin
  ec_$init(dtte.input_ec);
  ec_$init(dtte.output_ec);
  ec_$init(dtte.ec);
  dtte.flags := false;
  dtte.discipline := discipline;
end;
