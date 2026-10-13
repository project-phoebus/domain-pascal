{ MT_$RELEASE, 0xE70A4C, 4 bytes: the whole MT module (map `I E70A4C MT
  size = 4'; tools/imgdis.sh E70A4C 4).  Semantics:
  ../source/domain_os/mt/release.c: the tape driver is configured out and
  MT_$RELEASE does nothing; its callers pass three arguments by address
  (unit, mode, status: proc2's cleanup, ../source/domain_os/proc2).

  The unit is a whole map module, so tools/conform.py compiles it without
  --fragment-origin: `rts' and the module-end filler 0x2048 (the
  hand-written LPR_ASM 0xE70A40 is `rts' padded with zeros instead,
  codegen-patterns 2, 7).

  Patterns exercised (docs/codegen-patterns.md):
  - 2    an empty procedure with parameters: a bare `rts', no `link';
  - 7    the module end padded to 4 bytes with 0x2048.

  Reconstructed (not known from the original): the parameters (an empty
  procedure's are not visible; ../source's three, by address: `in' for
  the 16-bit unit and mode, `out' for the status); status_$t as a record
  with the integer32 field `all' (ref 9.1.1-9.1.2, pp. 9-1..9-3). }
module mt;
type
  status_$t = record
    all: integer32;
  end;

procedure mt_$release(in unit: integer; in mode: integer; out status: status_$t);
begin
end;
