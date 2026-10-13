{ SYSBUS_$DEFINE_INT, 0xE0ABBC..0xE0ABD4 (0x12 bytes of code, the filler
  0x2048 at 0xE0ABCE and the longword cell 0xE0ABD0; tools/imgdis.sh
  E0ABBC 18), the last routine of the ATBUS_ code segment (0xE0AABC, size
  0x118): a module-tail fragment.  Semantics:
  ../source/domain_os/sysbus/define_int.c.  Externals (SAU2 map):
  CRASH_SYSTEM 0xE1E700.

  Patterns exercised (docs/codegen-patterns.md):
  - 1.5  a longword constant passed by address (bead domain-pascal-5aw.9):
         the status 0x003E0002 (`status_$sysbus_unknown_interrupt_id' in
         ../source) in a cell after the routine, `pea (0xe,PC)' at
         0xE0ABC0; in an aligning routine the cell is 4-aligned with the
         0x2048 filler before it (0xE0ABCE), which also leaves the module
         end aligned (no end padding).

  Reconstructed (not known from the original): the names, `in' for
  CRASH_SYSTEM's parameter. }
module atbus_;
type
  status_$t = integer32;

procedure crash_system(in status: status_$t); extern;

procedure sysbus_$define_int;
begin
  crash_system(16#003e0002);
end;
