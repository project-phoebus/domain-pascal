{ bead domain-pascal-x6a1 (review): a pointer local stored from A0 and tested
  against nil right after the store (the `stored` shortcut of compare, AST_$RESERVE
  0xE067BA) while the destination index takes more instructions than the value
  call: SKFP_$PTR w:1 l:0
  call: SKFP_$SHOW w:6
  call: SKFP_$PTR w:1 l:4
  call: SKFP_$SHOW w:6
  expect:
    1
    0
}
module skfp;
%include 'exec.ins.pas';
var
  h: integer := 2;
  g: array[1..100] of boolean;

procedure skfp_$ptr(p: integer; q: univ_ptr); options(val_param);
var x: univ_ptr;
begin
  g[p * 4 + h] := q <> nil;
  x := exec_$id_p(q);
  g[p * 4 + h] := x = nil;
  discard(exec_$id_p(x));
end;

procedure skfp_$show(i: integer); options(val_param);
begin
  if g[i] then exec_$logv_w(1) else exec_$logv_w(0);
end;
