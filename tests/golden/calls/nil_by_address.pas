{ Golden: codegen-patterns 1.5, "`nil` by address" (bead
  domain-pascal-dtbg): `nil' passed by address is a longword 0 cell in
  the code section, `pea (d,PC)', one cell for the module: SAU5
  PBU_$RELEASE_INT 0xE59498, PBU_$SHUTDOWN 0xE59BE0 and PBU_$FREE_ASID
  0xE5A77A all name 0xE594B0 `0000 0000', placed after PBU_$RELEASE_INT
  (its first user).  The cell is keyed as an integer32 0, so the
  integer32 0 and `ddf_ptr_t(0)' below share it.  A pointer constant
  sharing with an equal integer32 is Verified (SAU2 OS_$INIT 0xE34130,
  0xE3414E: 0xE34AA0); `nil' sharing with them is Uncertain (no module
  is known to pass `nil' and another longword 0 by address). }
module pbu;
type
  ddf_t = record
    i: integer;
  end;
  ddf_ptr_t = ^ddf_t;

procedure pbu_$setdown(in unit: integer; in ddf: ddf_ptr_t;
                       out status: integer32); extern;
procedure pbu_$count(in n: integer32); extern;

procedure pbu_$release_int(in unit: integer; out status: integer32);
begin
  pbu_$setdown(unit, nil, status);
end;

procedure pbu_$shutdown(in unit: integer; out status: integer32);
begin
  pbu_$setdown(unit, nil, status);
  pbu_$count(0);
  pbu_$setdown(unit, ddf_ptr_t(0), status);
end;
