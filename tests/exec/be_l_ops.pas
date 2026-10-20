{ Execution test (bead domain-pascal-5kgg): operations on the lowering's
  best-effort path.

  - `rshft(w, 8) & c` of a `0..65535` word (ref p. 4-171: a logical
    shift, the high byte; codegen-patterns 8);
  - `min` and `max` of one-byte integers, signed (`-128..127`) and
    unsigned (`[byte] 0..255`, `min` only: an unsigned `max`
    is refused in the selector) (ref p. 4-111/4-115, the smaller and the
    larger operand);
  - an array index of a signed one-byte subrange (sign-extended, ref
    3.11: the element at that index);
  - `nil` passed by value next to another constant argument;
  - a store into a packed field of a volatile record (ref 3.15.1: the
    unit read and written back, the other fields kept).

  call: BE_L_OPS_$SHIFT w:0xABCD -> w
  call: BE_L_OPS_$SHIFT w:0x0700 -> w
  call: BE_L_OPS_$MINMAX b:-5 b:7
  call: BE_L_OPS_$MINMAX b:100 b:-100
  call: BE_L_OPS_$UMINMAX b:200 b:7
  call: BE_L_OPS_$INDEX b:-2 -> w
  call: BE_L_OPS_$INDEX b:3 -> w
  call: BE_L_OPS_$NIL
  call: BE_L_OPS_$PACKED w:5
  warns: best effort (not byte-identical): `rshft(w, 8) & c` of a `0..65535` word
  warns: best effort (not byte-identical): `max` or `min` of a one-byte integer
  warns: best effort (not byte-identical): an array index of a signed byte type
  warns: best effort (not byte-identical): `nil` passed by value next to another constant argument
  warns: best effort (not byte-identical): storing into a packed field of a volatile record
  expect:
    => 3
    => 7
    -5
    7
    -100
    100
    7
    => -20
    => 30
    5
    1
    5
    9
    1
}
module be_l_ops;
%include 'exec.ins.pas';
type
  uw = 0..65535;
  sb = [byte] -128..127;
  ub = [byte] 0..255;
  pk = packed record
    a: 0..15;
    b: 0..15;
    c: 0..255;
  end;
var
  tab: array[-3..3] of integer;
  pv: [volatile] pk;

function be_l_ops_$shift(w: uw): integer; options(val_param);
begin
  be_l_ops_$shift := rshft(w, 8) & 7;
end;

procedure be_l_ops_$minmax(a, b: sb); options(val_param);
var i: integer;
begin
  i := min(a, b);
  exec_$log_w(i);
  i := max(a, b);
  exec_$log_w(i);
end;

procedure be_l_ops_$uminmax(a, b: ub); options(val_param);
var i: integer;
begin
  i := min(a, b);
  exec_$log_w(i);
end;

function be_l_ops_$index(k: sb): integer; options(val_param);
var j: integer;
begin
  for j := -3 to 3 do tab[j] := j * 10;
  be_l_ops_$index := tab[k];
end;

procedure two(v: integer32; q: univ_ptr); options(val_param);
begin
  exec_$log_l(v);
  if q = nil then exec_$log_l(1) else exec_$log_l(0);
end;

procedure be_l_ops_$nil;
begin
  two(5, nil);
end;

procedure be_l_ops_$packed(k: integer); options(val_param);
var i: integer;
begin
  pv.a := 1;
  pv.c := 9;
  pv.b := k;
  i := pv.b;
  exec_$log_w(i);
  i := pv.c;
  exec_$log_w(i);
  i := pv.a;
  exec_$log_w(i);
end;
