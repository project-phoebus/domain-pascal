{ The descriptor of late_slot_pass.pas and its pointer function, in a
  unit of their own (as park_int_desc.mod.pas): late_slot_pass.pas
  declares the function with a 4-byte integer result in A0 (`a0_return',
  ref 5.5.11); the result comes back in A0 either way (ref 5.5.10). }
module lsp_desc;

type
  uid_$t = record
    high, low: integer32;
  end;
  desc_t = record
    pad: integer32;
    uid: uid_$t;
  end;
  desc_ptr_t = ^desc_t;
var
  lsp_$desc: desc_t := [pad := 1, uid := [high := 5, low := 6]];
  gp: desc_ptr_t := addr(lsp_$desc);
define lsp_$desc;

function lsp_$getd(in k: integer32; out st: integer32): desc_ptr_t;
begin
  if k < 0 then
    st := 1
  else
    st := 0;
  lsp_$getd := gp;
end;
