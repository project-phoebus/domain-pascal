{ The descriptor of park_int.pas and its pointer function, in a unit of
  their own: park_int.pas declares the function with a 4-byte integer
  result in A0 (`a0_return', ref 5.5.11), as bead domain-pascal-yzxr's
  reading of TTY_$I_GET_DESC has it; the result comes back in A0 either
  way (ref 5.5.10). }
module park_int_desc;

type
  desc_t = record
    a: integer32;
    b: integer32;
  end;
  desc_ptr_t = ^desc_t;
var
  park_int_$desc: desc_t := [a := 7, b := 10];
  gp: desc_ptr_t := addr(park_int_$desc);
define park_int_$desc;

function park_int_$getd(in k: integer32; out st: integer32): desc_ptr_t;
begin
  if k < 0 then
    st := 1
  else
    st := 0;
  park_int_$getd := gp;
end;
