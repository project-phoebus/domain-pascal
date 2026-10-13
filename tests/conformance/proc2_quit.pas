{ PROC2_$QUIT, 0xE3F130..0xE3F160 (0x28 bytes of code, the word cell
  0xE3F158, the filler 0x2048 and the longword cell 0xE3F15C;
  tools/imgdis.sh E3F130 30), a fragment of the PROC2 code segment
  (0xE3E748).  Semantics: ../source/domain_os/proc2/quit.c.
  PROC2_$SIGNAL 0xE3EFA0, of the same module, lies outside the unit
  (`module-routines=PROC2_$SIGNAL', `bsr.w').  The routine loads A5
  without using it (`a5-always=', codegen-patterns 4); the data area is
  placed at the PROC2 data segment 0xE7BE84.

  Patterns exercised (docs/codegen-patterns.md):
  - 1.5  two constants passed by address in one call (bead
         domain-pascal-5aw.9): the longword 0x00120010 pushed first
         (0xE3F140 -> 0xE3F15C), the word 3 second (0xE3F144 ->
         0xE3F158); laid out word first, then the 0x2048 filler and the
         4-aligned longword (an aligning routine; LOG_$SHUTDN 0xE17604 has
         the same shape).

  Reconstructed (not known from the original): the names; PROC2_$SIGNAL's
  parameter types (../source: an `int16_t' signal, SIGQUIT = 3, and a
  `uint32_t'); modes; `standin', the unit's data area. }
module proc2;
type
  status_$t = integer32;
  uid_$t = record high, low: integer32; end;
var
  standin: integer32;

procedure proc2_$signal(in uid: uid_$t; in sig: integer; in param: integer32;
                        out status: status_$t); extern;

procedure proc2_$quit(in uid: uid_$t; out status: status_$t);
begin
  proc2_$signal(uid, 3, 16#120010, status);
end;
