{ PCHIST_$INTERRUPT, 0xE1A1F6..0xE1A20C (0x14 bytes of code and the word
  cell 0xE1A20A; tools/imgdis.sh E1A1F6 16), the last routine of the
  PCHIST code segment (0xE1A134, size 0xD8): a module-tail fragment.
  Semantics: ../source/domain_os/pchist/interrupt.c.  PCHIST_$COUNT
  0xE1A134, the first routine of the module, lies outside the unit
  (`module-routines=PCHIST_$COUNT', `bsr.w').

  Patterns exercised (docs/codegen-patterns.md):
  - 1.5  a word constant passed by address (bead domain-pascal-5aw.9):
         1 in a cell after the routine (0xE1A20A), `pea (0xe,PC)' at
         0xE1A1FA, the first argument pushed (the last parameter); the
         by-address parameter passed on, `move.l (8,A6),-(SP)';
  - 2    `link #-4' in a map-named routine with no locals in the C:
         not explained by the frame model (codegen-patterns 2); the unit
         declares a 4-byte stand-in local.

  Reconstructed (not known from the original): the names, the parameter
  types and modes; `standin', a 4-byte local for `link #-4' (the C has
  no local; what the original's frame held is not known). }
module pchist;

procedure pchist_$count(in pc: integer32; in mode: integer); extern;

procedure pchist_$interrupt(in pc: integer32);
var standin: integer32;
begin
  pchist_$count(pc, 1);
end;
