{ PBU_$FREE_ASID, 0xE590F8, PBU_$FAULTED_UNITS, 0xE590FA, and PBU_$INIT,
  0xE5910E: the whole PBU module (map `I E590F8 PBU size = 18';
  tools/imgdis.sh E590F8 18).  Semantics: ../source/domain_os/pbu/
  free_asid.c and init.c (bare `rts' stubs) and faulted_units.c (stores
  status_$pbu_not_present, 0x1E000A, "pbu not present" in the 10.2
  status-code database, and returns a 16-bit zero).

  A whole map module (bead domain-pascal-xdh.2): tools/conform.py compiles
  it without --fragment-origin; 0x18 bytes, no module-end filler.

  Patterns exercised (docs/codegen-patterns.md):
  - 2    empty procedures as a bare `rts' (0xE590F8, 0xE5910E);
  - 3    a function body of a status store and then a constant result:
         `link #-4' for the result variable, `clr.w D0' (0xE59108);
  - 9    `status.all := ...' through the status slot (0xE590FE).

  Reconstructed (not known from the original): the empty procedures
  without parameters (not visible; the C has none); `integer' for the
  16-bit result (the C's uint16_t would be `0..65535', the same
  `clr.w'); `out' as the status mode; status_$t as a record with the
  integer32 field `all' (ref 9.1.1-9.1.2, pp. 9-1..9-3).  The constant
  is ../source's name (pbu/pbu.h). }
module pbu;
const
  status_$pbu_not_present = 16#001E000A;
type
  status_$t = record
    all: integer32;
  end;

procedure pbu_$free_asid;
begin
end;

function pbu_$faulted_units(out status: status_$t): integer;
begin
  status.all := status_$pbu_not_present;
  pbu_$faulted_units := 0;
end;

procedure pbu_$init;
begin
end;
