{ Execution test (bead domain-pascal-k1sv): stores whose destination and
  value both need a register (codegen-patterns 12.4, "Stores whose
  destination and value both need a register"), in the shape of
  TTY_$K_SET_DELAY 0xE67A40 (`d^.delay[k] := v', the element's address
  set up first) and TTY_$K_INQ_DELAY 0xE67A96 (`v := d^.delay[k]', the
  element first as the value): the descriptor reached through an integer
  local held in D0 (`a0_return'), the index and the value by address.
  The element written must be the one indexed, the value the one passed;
  an index outside the enumeration is refused first with `in_range' (ref
  p. 4-104), as TTY_$K_SET_FUNC_CHAR 0xE67502 checks its slot.

  link: store_order_desc.mod.pas
  call: STORE_ORDER_$SET @w:1 @w:4 @w:-7 @l:99!
  call: STORE_ORDER_$GET @w:1 @w:4 @w:0! @l:99!
  call: STORE_ORDER_$GET @w:1 @w:0 @w:0! @l:99!
  call: STORE_ORDER_$GET @w:1 @w:3 @w:0! @l:99!
  call: STORE_ORDER_$SET @w:-1 @w:2 @w:5 @l:99!
  call: STORE_ORDER_$SET @w:1 @w:5 @w:5 @l:99!
  call: STORE_ORDER_$SET @w:1 @w:-1 @w:5 @l:99!
  call: STORE_ORDER_$GET @w:1 @w:2 @w:0! @l:99!
  call: STORE_ORDER_$GET @w:1 @w:5 @w:0! @l:99!
  expect:
    out 0
    out -7
    out 0
    out 10
    out 0
    out 13
    out 0
    out 1
    out 2
    out 2
    out 12
    out 0
    out 0
    out 2
}
module store_order;
%include 'exec.ins.pas';

type
  k_t = (k0, k1, k2, k3, k4);
  desc_t = record
    pad: integer32;
    delay: array[k_t] of integer;
  end;
  desc_ptr_t = ^desc_t;

function store_order_$getd(in k: integer; out st: integer32): integer32;
  options(extern, a0_return);

procedure store_order_$set(in line: integer; in k: k_t; in v: integer;
                           out st: integer32);
var
  d: integer32;
begin
  if not in_range(k) then begin
    st := 2;
    return;
  end;
  d := store_order_$getd(line, st);
  if st <> 0 then
    return;
  desc_ptr_t(d)^.delay[k] := v;
end;

procedure store_order_$get(in line: integer; in k: k_t; out v: integer;
                           out st: integer32);
var
  d: integer32;
begin
  if not in_range(k) then begin
    st := 2;
    return;
  end;
  d := store_order_$getd(line, st);
  if st <> 0 then
    return;
  v := desc_ptr_t(d)^.delay[k];
end;
