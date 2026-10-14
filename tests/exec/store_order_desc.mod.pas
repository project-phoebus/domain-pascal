{ The descriptor of store_order.pas and its pointer function, in a unit of
  their own (as park_int_desc.mod.pas): store_order.pas declares the
  function with a 4-byte integer result in A0 (`a0_return', ref 5.5.11),
  the reading of TTY_$I_GET_DESC in ../source tty/tty.ins.pas. }
module store_order_desc;

type
  k_t = (k0, k1, k2, k3, k4);
  desc_t = record
    pad: integer32;
    delay: array[k_t] of integer;
  end;
  desc_ptr_t = ^desc_t;
var
  store_order_$desc: desc_t := [pad := 0, delay := [10, 11, 12, 13, 14]];
  gp: desc_ptr_t := addr(store_order_$desc);
define store_order_$desc;

function store_order_$getd(in k: integer; out st: integer32): desc_ptr_t;
begin
  if k < 0 then
    st := 1
  else
    st := 0;
  store_order_$getd := gp;
end;
