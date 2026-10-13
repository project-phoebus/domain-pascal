{ TAPE_$BOOT, 0xE32734, 0x10 bytes: the whole TAPE_BOOT module (map `I
  E32734 TAPE_BOOT size = 10'; tools/imgdis.sh E32734 10).  Semantics:
  ../source/domain_os/tape/boot.c: two by-address arguments; the first is
  cleared, the second (a status) never read; the boolean result is false.

  A whole map module (bead domain-pascal-xdh.2); 0x10 bytes, no module-end
  filler.

  Patterns exercised (docs/codegen-patterns.md):
  - 3    a function body of a store and then a constant boolean result:
         `link #-4' for the result variable, `clr.b D0' (0xE3273E);
  - 1.2  the stored slot `movea.l (8,A6),A0; clr.l (A0)' (0xE32738).

  Reconstructed (not known from the original): the names (../source's),
  the parameter types and modes (`out integer32' for the cleared slot,
  `out status_$t' for the one never read); status_$t as a record with
  the integer32 field `all' (ref 9.1.1-9.1.2, pp. 9-1..9-3). }
module tape_boot;
type
  status_$t = record
    all: integer32;
  end;

function tape_$boot(out entry_point: integer32; out status: status_$t): boolean;
begin
  entry_point := 0;
  tape_$boot := false;
end;
