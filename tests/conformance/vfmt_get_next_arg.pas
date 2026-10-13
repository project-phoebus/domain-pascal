{ Module-local at 0xE6AA38 of VFMT, 0x14 bytes (tools/imgdis.sh E6AA38 14),
  the routine ../source/domain_os/vfmt/main.c calls `vfmt_get_next_arg':
  it takes the address of a cell holding a pointer and returns that
  pointer, in A0.

  A fragment of the VFMT module (code segment 0xE6A704), compiled with
  `dpc --fragment-origin 0xE6AA38'.  It has no loop and does not use the
  unit's data (no A5, codegen-patterns 4).

  Patterns exercised (docs/codegen-patterns.md):
  - 3    a pointer result in A0 (ref 5.5.10, p. 5-20), the only statement
         `f := cell' from memory: stored into the result variable's slot,
         `move.l (A0),(-8,A6)', then loaded from it, `movea.l (-8,A6),A0'
         (0xE6AA40, 0xE6AA44);
  - 2    `link #-8': the reserved 4-byte slot of a routine that is not
         a global symbol, then the result variable at (-8,A6);
  - 1.2  the default convention: the cell by address at (8,A6), loaded
         where it is needed (`movea.l (8,A6),A0', 0xE6AA3C).

  Reconstructed (not known from the original): the names; `internal' for
  a routine the map does not name; the cell's type (`univ_ptr', the C's
  `void *'); `var' as the mode (`in' compiles the same, ref 5.2 Table
  5-1). }
module vfmt;

function get_next_arg(var cell: univ_ptr): univ_ptr; internal;
begin
  get_next_arg := cell;
end;
