{ Execution test: spilled bodies, review cases part 2 (bead
  domain-pascal-qea9): address and data pressure together through
  run-time indices, a nested routine reading its parent's parameters.
  Expected values from a Python model of the Pascal semantics.

  call: RV2_$APTRS w:1 w:2 w:3 w:4 w:5 w:6 l:7 l:8 l:9 l:10 l:11 l:12 l:13
  call: RV2_$APTRS w:6 w:6 w:1 w:2 w:2 w:3 l:-7 l:8 l:-9 l:10 l:-11 l:12 l:-13
  call: RV2_$PNEST l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8
  call: RV2_$BOOLS l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8
  call: RV2_$BOOLS l:8 l:7 l:6 l:5 l:4 l:3 l:2 l:1
  call: RV2_$JOIN l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8 w:5
  call: RV2_$JOIN l:-1 l:2 l:-3 l:4 l:-5 l:6 l:-7 l:8 w:4
  call: RV2_$PFUN_CHK
  call: RV2_$WITHLOOP w:1 w:2 w:3 w:4 w:5 w:6 w:3
  warns: best effort (not byte-identical): more values live at once than there are registers
  expect:
    25
    42
    30
    87
    50
    60
    70
    63
    63
    10
    13
    13
    30
    -10
    143
    26
    30
    103
    333
    100
    1
    5
    15
    8
    true
    true
    36
    1
    false
    false
    36
    19
    27
    29
    37
    39
    5
    20
    36
    11
    -5
    3
    -13
    1
    -26
    4
    19
    40
    4
    58
    77
    97
    16
    81
}
module be_spill3;
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

{ pointer locals and data values live across calls together }
procedure rv2_$aptrs(i, j, k, l, m, n: integer; a, b, c, d, e, f, g: integer32);
  options(val_param);
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
  p1^.a := p1^.a + a + b;
  exec_$clobber;
  p2^.a := p2^.a + c + d + p3^.b;
  exec_$clobber;
  p4^.a := p4^.a + e + f + g + p5^.b + p6^.b;
  exec_$clobber;
  if p1^.a > p2^.a then exec_$logv_l(p1^.a - p2^.a);
  exec_$logv_l(p1^.a);
  exec_$logv_l(p2^.a);
  exec_$logv_l(p3^.a);
  exec_$logv_l(p4^.a);
  exec_$logv_l(p5^.a);
  exec_$logv_l(p6^.a);
  exec_$logv_l(a + b + c + d + e + f + g);
end;

{ the parent under pressure around calls of a nested routine that reaches
  the parent's locals and parameters }
procedure rv2_$pnest(a, b, c, d, e, f, g, h: integer32); options(val_param);
var t, u, w: integer32;
  v: array[1..3] of integer32;

  procedure inner(x: integer32); options(val_param);
  begin
    exec_$clobber;
    t := t + x + a;
    v[2] := v[2] + u + h;
    u := u + 1;
  end;

begin
  t := 0;
  u := 100;
  v[1] := 1;
  exec_$clobber;
  v[2] := 2;
  exec_$clobber;
  v[3] := 3;
  inner(b + c);
  exec_$logv_l(a + b + c + d + e + f + g + h + t + u);
  inner(d + e + f);
  exec_$logv_l(t - a + b - c + d - e + f - g + h);
  inner(g);
  exec_$logv_l(t);
  exec_$logv_l(u);
  exec_$logv_l(v[1] + v[2] + v[3]);
  w := a * b;
  w := w + c * d;
  w := w + e * f;
  w := w + g * h;
  exec_$logv_l(w);
end;

{ booleans (bytes, `s<cc>`) live across calls }
procedure rv2_$bools(a, b, c, d, e, f, g, h: integer32); options(val_param);
var x1, x2, x3, x4, x5, x6, x7: boolean;
begin
  x1 := a < b;
  x2 := b < c;
  x3 := c < d;
  x4 := d < e;
  x5 := e < f;
  x6 := f < g;
  x7 := g < h;
  exec_$clobber;
  if x1 then exec_$logv_l(a);
  exec_$clobber;
  if x2 and x3 then exec_$logv_l(b + c);
  if x4 or x5 then exec_$logv_l(d + e + f);
  if (x6 and not x7) or (x7 and not x6) then exec_$logv_l(g) else exec_$logv_l(h);
  exec_$log_bool(x1);
  exec_$log_bool(x7);
  exec_$logv_l(a + b + c + d + e + f + g + h);
end;

{ values written on both sides of an `if` and around a loop's back edge }
procedure rv2_$join(a, b, c, d, e, f, g, h: integer32; n: integer); options(val_param);
var s, t, i32: integer32;
  i: integer;
begin
  s := 1;
  t := 2;
  for i := 1 to n do begin
    exec_$clobber;
    if odd(i) then begin
      s := s + a + c;
      t := t - b;
    end
    else begin
      s := s - d;
      t := t + e + g;
    end;
    exec_$clobber;
    i32 := s + t + f + h;
    if i32 < 0 then s := -s;
    exec_$logv_l(i32);
  end;
  exec_$logv_l(s);
  exec_$logv_l(t);
  exec_$logv_l(a + b + c + d + e + f + g + h);
end;

{ a pointer function under pressure }
function rv2_$pfun(i, j, k, l, m, n: integer): pr; options(val_param);
var p1, p2, p3, p4, p5: pr;
begin
  fill;
  p1 := addr(arr[i]);
  p2 := addr(arr[j]);
  p3 := addr(arr[k]);
  p4 := addr(arr[l]);
  p5 := addr(arr[m]);
  exec_$clobber;
  p1^.b := p1^.b + p2^.b + p3^.b + p4^.b + p5^.b;
  exec_$clobber;
  exec_$logv_w(p1^.b);
  if p1^.b > 10 then rv2_$pfun := p2 else rv2_$pfun := p3;
end;

{ the pointer rv2_$pfun returns: arr[4] (40, 4), not arr[5] }
procedure rv2_$pfun_chk;
var p: pr;
begin
  p := rv2_$pfun(3, 4, 5, 6, 1, 2);
  exec_$logv_l(p^.a);
  exec_$logv_w(p^.b);
end;

{ `with` records in a loop with calls }
procedure rv2_$withloop(i, j, k, l, m, n, cnt: integer); options(val_param);
var c: integer;
begin
  fill;
  with arr[i] : u, arr[j] : v, arr[k] : w, arr[l] : x, arr[m] : y, arr[n] : z do
    for c := 1 to cnt do begin
      exec_$clobber;
      u.a := u.a + v.b;
      exec_$clobber;
      w.a := w.a + x.b + y.b + z.b + c;
      exec_$logv_l(u.a + w.a);
    end;
  exec_$logv_l(arr[i].a);
  exec_$logv_l(arr[k].a);
end;
