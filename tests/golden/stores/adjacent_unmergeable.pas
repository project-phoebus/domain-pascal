{ Golden: codegen-patterns 13, "Store merging": consecutive stores to
  adjacent words that no one reference could make are not merging
  candidates (bead domain-pascal-2m0).  set_scales has the shape of
  TPAD_$SET_UNIT_MODE 0xE6982C..0xE69840 (three words of one record from
  three by-address parameters, `move.w (A0),(-40,A1)` ..); set_mode a
  constant next to a value read through a parameter.  Only the stores
  are this test's subject: which address registers hold the pointers is
  dpc's register rule (codegen-patterns 12.4), not compared with
  0xE6982C, whose base is a `with' element in A1. }
module adj;
type
  unit_t = record
    mode: integer;
    x_scale: integer;
    y_scale: integer;
  end;
  unit_ptr = ^unit_t;
procedure set_scales(u: unit_ptr; var m, x, y: integer); options(val_param);
begin
  u^.mode := m;
  u^.x_scale := x;
  u^.y_scale := y;
end;
procedure set_mode(u: unit_ptr; var x: integer); options(val_param);
begin
  u^.mode := 0;
  u^.x_scale := x;
end;
