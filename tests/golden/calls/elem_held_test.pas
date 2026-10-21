{ Golden: codegen-patterns 1.6 ("Held across an `if''s test"); bead
  domain-pascal-59st.  The IO_ module's layout, as in
  calls/elem_routine_pointers: IO_$BUS_EPV at +0x60 of the unit's data
  (0x24-byte vectors), IO_$DCTES at +0x118 (0x20-byte slots).

  - `if v[i].p <> nil then v[i].p^(...)': the element's address with
    `lea (0,A5,D0.w),A2' before the test, `tst.l (d,A2)', and the
    pointer read through the same register for the call, `movea.l
    (d,A2),A0; jsr (A0)' (IO_$ENABLE_DEVICE 0xE1A876..0xE1A888,
    IO_$DISABLE_DEVICE 0xE1A820..0xE1A832); the held address in A2, the
    enclosing `with''s in A3 (0xE1A866, 0xE1A810).  Both routines are
    byte-identical to the image but for the relocations.
  - The next statement forms the address again (IO_$RELEASE 0xE1A724..
    0xE1A730 `lea (0,A5,D1.w),A0; movea.l (0x6c,A0),A1; jsr (A1)').

  Not as the image: IO_$RELEASE's next statement calls define_int with
  two `nil' arguments (`clr.l -(SP)' twice, 0xE1A71C), which `dpc
  --strict' refuses; here a call with one argument.  Its index in D1
  with D0 the temporary (0xE1A724..0xE1A72E), where `dpc' has them the
  other way round. }
module ehold;

type
  unit_t = 0..31;
  status_t = record
    all: integer32;
  end;
  int_proc_t = ^procedure(id: integer32); options(val_param);
  int_func_t = ^function(id: integer32): boolean; options(val_param);
  xlate_t = ^function(a: integer32): integer32; options(val_param);
  epv_t = record
    init: univ_ptr;                       { 0x00 }
    xlate_iova: xlate_t;                  { 0x04 }
    unused_08: univ_ptr;                  { 0x08 }
    define_int: univ_ptr;                 { 0x0C }
    enable_device: int_proc_t;            { 0x10 }
    disable_device: int_proc_t;           { 0x14 }
    device_interrupting: int_func_t;      { 0x18 }
    clear_pending: int_proc_t;            { 0x1C }
    csr_align: integer;                   { 0x20 }
    unused_22: integer;                   { 0x22 }
  end;
  dcte_t = record
    id: integer32;                        { 0x00 }
    pad: array[0..3] of integer32;
    tag: integer;                         { 0x14 }
    bus: integer;                         { 0x16 }
    rest: array[0..1] of integer32;
  end;

var
  front: array[0..23] of integer32;       { 0x00..0x60 }
  epv: array[0..4] of epv_t;              { 0x60 }
  gap: integer32;                         { 0x114 }
  dctes: array[0..31] of dcte_t;          { 0x118 }

function check_unit(unit: unit_t; out status: status_t): boolean;
  internal;
begin
  status.all := 0;
  if not in_range(unit) then
    status.all := 16#100008
  else if dctes[unit].tag = 0 then
    status.all := 16#100009;
  check_unit := status.all = 0;
end;

procedure release(unit: unit_t; out status: status_t);
  options(val_param);
begin
  if check_unit(unit, status) then
    with dctes[unit] do begin
      if epv[bus].disable_device <> nil then
        epv[bus].disable_device^(id);
      epv[bus].clear_pending^(id);
      tag := 0;
    end;
end;

procedure disable_device(unit: unit_t; out status: status_t);
  options(val_param);
begin
  if check_unit(unit, status) then
    with dctes[unit] do
      if epv[bus].disable_device <> nil then
        epv[bus].disable_device^(id);
end;

procedure enable_device(unit: unit_t; out status: status_t);
  options(val_param);
begin
  if check_unit(unit, status) then
    with dctes[unit] do
      if epv[bus].enable_device <> nil then
        epv[bus].enable_device^(id);
end;
