{ Golden: codegen-patterns 1.6 ("Through an element of the unit's
  array"), 3 ("Result variable"), 7.1 ("A branch past the test of a
  store"); bead domain-pascal-do4s, ../source pascal-translation-notes
  373, 374, 378.  The IO_ module's layout: IO_$BUS_EPV at +0x60 of the
  unit's data (0x24-byte vectors), IO_$DCTES at +0x118 (0x20-byte slots).

  - A call through a routine pointer in an element of the unit's array:
    the element's address `lea (0,A5,D0.w),A0', then the pointer through
    it, `movea.l (d,A0),A1; jsr (A1)', the array's offset in `d'
    (IO_$CLEAR_PENDING 0xE1A7C8..0xE1A7DC; IO_$DEVICE_INTERRUPTING
    0xE1A784; IO_$MAP_CSR 0xE1A8C0).
  - A function result assigned on one path only, after the last call:
    left in D0, no register for the result variable
    (IO_$DEVICE_INTERRUPTING 0xE1A788..0xE1A78E, IO_$XLATE_IOVA
    0xE1A9B2..0xE1A9B8: `jsr (A1)' then the epilogue).
  - A `bra' right after a status store goes past the status's `tst'
    (io_$check_unit 0xE1A4BE `move.l #0x100008,(A0); bra.s' to the `seq
    D0'), and after `clr', past a `bne' that cannot be taken too
    (IO_$ACQUIRE 0xE1A566 `clr.l (A4); bra.b 0xE1A576'); the fall-through
    store tested again (0xE1A4CE..0xE1A4D4).

  - The parameter's entry load after the call's pad (0xE1A756; 12.4
    R11, "The pad and the last entry load", bead domain-pascal-dfjh).

  Not as the image: the tag tested with `tst.w' where io_$check_unit
  loads it (0xE1A4C8; pascal-translation-notes 375). }
module eptr;

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

{ io_$check_unit }
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

{ IO_$CLEAR_PENDING }
procedure clear_pending(unit: unit_t; out status: status_t);
  options(val_param);
begin
  if check_unit(unit, status) then
    with dctes[unit] do
      epv[bus].clear_pending^(id);
end;

{ IO_$DEVICE_INTERRUPTING }
function device_interrupting(unit: unit_t; out status: status_t): boolean;
  options(val_param);
begin
  if check_unit(unit, status) then
    with dctes[unit] do
      device_interrupting := epv[bus].device_interrupting^(id);
end;

{ IO_$XLATE_IOVA }
function xlate_iova(unit: unit_t; iova: integer32;
                    out status: status_t): integer32;
  options(val_param);
begin
  if check_unit(unit, status) then
    with dctes[unit] do
      xlate_iova := epv[bus].xlate_iova^(iova);
end;

{ IO_$ACQUIRE's first loop }
procedure first_free(out unit: unit_t; out status: status_t);
label
  done;
var
  i: integer;
begin
  status.all := 16#10000b;
  for i := 0 to 31 do
    if dctes[i].tag = 0 then begin
      unit := i;
      status.all := 0;
      exit;
    end;
  if status.all <> 0 then
    goto done;
  dctes[unit].tag := 1;
done:
end;
