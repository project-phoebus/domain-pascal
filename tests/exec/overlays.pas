{ Execution test: overlay sections, `var (name)' (ref 3.14, p. 3-52:
  "just like a named common block in FORTRAN"; 7.2.4, Method 4, p. 7-7:
  "the variables x, y, and q from Math occupy the same memory locations
  as x, y, and q from Sub2.  Therefore, whatever is assigned to x in
  Math, can be retrieved in Sub2, and vice versa").  Two units declare
  the sections ovl_$init and ovl_$zero with the same layout and other
  names (overlays_peer.mod.pas); the peer gives ovl_$init its initial
  values, which this unit reads; each unit's stores are seen by the
  other.  The variables are reached as another unit's object
  (codegen-patterns 11, "Overlay sections"): absolute, `move.l #sym' for
  the section's start and `pea (sym+d).l' after it by address, an
  element off `movea.l #sym,An'; ovl_$zero is a common symbol in both
  units, ovl_$init one here and defined by the peer.

  link: overlays_peer.mod.pas
  call: OVERLAYS_$RUN w:2
  expect:
    5
    305419896
    81
    0
    5
    1000
    81
    42
    -1
    0
    -7
    99
    77
}
module overlays;
%include 'exec.ins.pas';

var (ovl_$init)
  a: integer;
  b: integer32;
  c: char;
var (ovl_$zero)
  z: integer;
  w: array[1..3] of integer32;

procedure ovp_$show; extern;
procedure ovp_$set; extern;

procedure overlays_$run(in k: integer);
options(val_param);
begin
  exec_$log_w(a);
  exec_$log_l(b);
  exec_$log_c(c);
  exec_$log_w(z);
  z := 42;
  w[k] := -1;
  b := 1000;
  ovp_$show;
  ovp_$set;
  exec_$log_w(a);
  exec_$log_l(w[3]);
  w[k + 1] := 77;
  exec_$log_l(w[3]);
end;
