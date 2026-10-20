{ Execution test: spilled bodies, adversarial cases from the review of
  bead domain-pascal-qea9 (best effort; crates/dpc-m68k/src/spill.rs):
  comparisons and if/else joins in a loop, a `case` in a `for` loop,
  nested routines, pointers to records through run-time indices,
  boolean results, words, temporaries, by-address parameters.
  Expected values from a Python model of the Pascal semantics.

  call: RV_$CMPLOOP l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8 w:4 -> l
  call: RV_$CMPLOOP l:9 l:-2 l:3 l:-4 l:5 l:60 l:7 l:-8 w:3 -> l
  call: RV_$CASE l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8 w:8
  call: RV_$NEST l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8
  call: RV_$PTRS w:1 w:2 w:3 w:4 w:5 w:6
  call: RV_$PTRS w:2 w:2 w:6 w:1 w:6 w:3
  call: RV_$BOOL l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8 -> b
  call: RV_$BOOL l:100 l:2 l:3 l:4 l:5 l:6 l:7 l:8 -> b
  call: RV_$WORDS w:1 w:2 w:3 w:4 w:5 w:6 w:7 w:8 w:9
  call: RV_$TEMPS l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8
  call: RV_$VARS @l:1! @l:2! @l:3! @l:4! @l:5! @l:6! w:3
  warns: best effort (not byte-identical): more values live at once than there are registers
  expect:
    8
    -5
    16
    -1
    24
    3
    32
    7
    => 107
    53
    -4
    106
    -8
    159
    -12
    => 376
    38
    41
    45
    50
    56
    63
    71
    74
    77
    3
    5
    7
    9
    11
    13
    15
    14
    0
    -9
    82
    14
    9
    91
    144
    36
    0
    60
    30
    20
    90
    40
    50
    60
    1
    40
    40
    40
    124
    10
    124
    30
    => false
    => true
    -1
    39
    82
    3
    7
    1011
    15
    1036
    34
    out 20
    out 28
    out 36
    out 40
    out 37
    out 34
}
module be_spill2;
%include 'exec.ins.pas';

type
  rec = record
    a: integer32;
    b: integer;
  end;
  pr = ^rec;
  recs = array[1..6] of rec;

var
  arr: recs;

procedure fill;
var n: integer;
begin
  for n := 1 to 6 do begin
    arr[n].a := n * 10;
    arr[n].b := n;
  end;
end;

function rv_$cmploop(a, b, c, d, e, f, g, h: integer32; n: integer): integer32;
  options(val_param);
var i: integer;
  s, t: integer32;
begin
  s := 0;
  t := 0;
  i := 0;
  while i < n do begin
    exec_$clobber;
    if s + a > b + c then t := t + d else t := t - e;
    if (f > g) and (h <> s) then s := s + f - g else s := s + h;
    exec_$logv_l(s);
    exec_$logv_l(t);
    i := i + 1;
  end;
  rv_$cmploop := s * 2 + t + a + b + c + d + e + f + g + h;
end;

procedure rv_$case(a, b, c, d, e, f, g, h: integer32; k: integer); options(val_param);
var i: integer;
begin
  for i := 0 to k do begin
    exec_$clobber;
    case i of
      0: a := a + b;
      1: b := b + c;
      2: c := c + d;
      3: d := d + e;
      4: e := e + f;
      5: f := f + g;
      6: g := g + h;
      otherwise h := h + a;
    end;
    exec_$logv_l(a + b + c + d + e + f + g + h);
  end;
  exec_$logv_l(a);
  exec_$logv_l(b);
  exec_$logv_l(c);
  exec_$logv_l(d);
  exec_$logv_l(e);
  exec_$logv_l(f);
  exec_$logv_l(g);
  exec_$logv_l(h);
end;

procedure rv_$nest(p, q, r, s, u, v, w, x: integer32); options(val_param);
var t: integer32;
  arr2: array[1..4] of integer32;

  procedure inner(a, b, c, d, e, f, g, h: integer32); options(val_param);
  begin
    exec_$clobber;
    t := t + a + b + c + d + e + f + g + h + p + x;
    arr2[2] := arr2[2] + t;
    exec_$clobber;
    exec_$logv_l(a - h + q + arr2[1]);
    exec_$logv_l(b - g + c - f + d - e);
  end;

begin
  t := 1;
  arr2[1] := 5;
  exec_$clobber;
  arr2[2] := 7;
  exec_$clobber;
  inner(p, q, r, s, u, v, w, x);
  exec_$logv_l(t + p + q + r + s + u + v + w + x);
  inner(x, w, v, u, s, r, q, p);
  exec_$logv_l(t);
  exec_$logv_l(arr2[2]);
  exec_$logv_l(p + q + r + s + u + v + w + x);
end;

procedure rv_$ptrs(i, j, k, l, m, n: integer); options(val_param);
var p1, p2, p3, p4, p5, p6: pr;
begin
  fill;
  p1 := addr(arr[i]);
  p2 := addr(arr[j]);
  p3 := addr(arr[k]);
  p4 := addr(arr[l]);
  p5 := addr(arr[m]);
  p6 := addr(arr[n]);
  exec_$clobber;
  p1^.a := p1^.a + p2^.a;
  exec_$clobber;
  p3^.a := p3^.a + p4^.b + p5^.a + p6^.b;
  exec_$clobber;
  if p1^.b = p2^.b then exec_$logv_l(1) else exec_$logv_l(0);
  if p1^.a > p6^.a then exec_$logv_l(p1^.a) else exec_$logv_l(p6^.a);
  exec_$logv_l(p1^.a);
  exec_$logv_l(p2^.a);
  exec_$logv_l(p3^.a);
  exec_$logv_l(p4^.a);
  exec_$logv_l(p5^.a);
  exec_$logv_l(p6^.a);
end;

function rv_$bool(a, b, c, d, e, f, g, h: integer32): boolean; options(val_param);
var x: integer32;
begin
  exec_$clobber;
  x := a + b + c + d;
  exec_$clobber;
  rv_$bool := (x + e > f + g + h) and (a <> b) and (c < d) and (e < f) and (g < h);
end;

procedure rv_$words(a, b, c, d, e, f, g, h, k: integer); options(val_param);
var x, y: integer;
begin
  exec_$clobber;
  x := a + b;
  exec_$clobber;
  y := c - d;
  if x = 0 then exec_$logv_w(-1);
  if y < 0 then exec_$logv_w(y) else exec_$logv_w(x);
  exec_$clobber;
  x := x + e + f + g + h + k;
  if x > y then exec_$logv_w(x - y);
  exec_$logv_w(a + b + c + d + e + f + g + h + k + x + y);
end;

procedure rv_$temps(a, b, c, d, e, f, g, h: integer32); options(val_param);
var loc: integer32;
begin
  loc := 1000;
  exec_$clobber;
  exec_$log_l(a + b);
  exec_$log_l(c + d);
  exec_$clobber;
  exec_$log_l(e + f + loc);
  exec_$log_l(g + h);
  exec_$logv_l(a + b + c + d + e + f + g + h + loc);
end;

procedure rv_$vars(var a, b, c, d, e, f: integer32; n: integer); options(val_param);
var i: integer;
  p: ^integer32;
begin
  p := addr(a);
  for i := 1 to n do begin
    exec_$clobber;
    a := a + b;
    b := b + c;
    c := c + d;
    exec_$clobber;
    d := d + e;
    e := e + f;
    f := f + p^;
    if i = 2 then p := addr(f);
  end;
  exec_$logv_l(p^);
end;
