{ IO_$DISABLE_DEVICE 0xE1A7E8, 0x56 bytes, and IO_$ENABLE_DEVICE 0xE1A83E,
  0x56 bytes (tools/imgdis.sh E1A7E8 AC), of the IO_ module (map `I
  E1A404 IO_ size = 5D0'; data `D E2C368 IO_ size = 554', manifest
  --section-addr).  Semantics from ../source io/disable_device.pas and
  io/enable_device.pas: for a unit io_$check_unit accepts, the call of
  its bus's disable_device (enable_device) routine with the unit's
  interrupt id, when the bus has one.  io_$check_unit is the module-local
  0xE1A4A4 (manifest module-routines; called with `bsr.w').

  Patterns exercised (docs/codegen-patterns.md):
  - 1.6  "Held across an `if''s test" (bead domain-pascal-59st): the
         vector element's address `lea (0,A5,D0.w),A2' before the test
         (0xE1A820, 0xE1A876), `tst.l (d,A2)', and the routine pointer read
         through A2 for the call, `movea.l (d,A2),A0; jsr (A0)'
         (0xE1A82E, 0xE1A884); the `with''s element address in A3
         (0xE1A810, 0xE1A866);
  - 12.4 R4(c) the held address in a callee-saved register;
  - 12.4 R11 the pad above the entry load of `unit' (0xE1A7F6,
         0xE1A84C);
  - 1.1  no pop after the last call (`unlk' follows).

  Reconstructed (not known from the original): the names of the types
  and fields but for the routines', the `with' (../source's choice; the
  frame's 4 bytes are its slot, codegen-patterns 2), the layout of the
  data area before IO_$BUS_EPV (+0x60) and between the vector and
  IO_$DCTES (+0x118): only the offsets the routines use are known. }
module io_, wired_proc, wired_data;

type
  unit_t = 0..31;
  status_t = record
    all: integer32;
  end;
  int_proc_t = ^procedure(id: integer32); options(val_param);
  epv_t = record
    init: univ_ptr;                       { 0x00 }
    xlate_iova: univ_ptr;                 { 0x04 }
    unused_08: univ_ptr;                  { 0x08 }
    define_int: univ_ptr;                 { 0x0C }
    enable_device: int_proc_t;            { 0x10 }
    disable_device: int_proc_t;           { 0x14 }
    device_interrupting: univ_ptr;        { 0x18 }
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

function io_$check_unit(unit: unit_t; out status: status_t): boolean;
  internal; extern;

procedure io_$disable_device(unit: unit_t; out status: status_t);
  options(val_param);
begin
  if io_$check_unit(unit, status) then
    with dctes[unit] do
      if epv[bus].disable_device <> nil then
        epv[bus].disable_device^(id);
end;

procedure io_$enable_device(unit: unit_t; out status: status_t);
  options(val_param);
begin
  if io_$check_unit(unit, status) then
    with dctes[unit] do
      if epv[bus].enable_device <> nil then
        epv[bus].enable_device^(id);
end;
