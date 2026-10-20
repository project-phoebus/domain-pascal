{ Execution test: spilled bodies, review cases part 3 (bead
  domain-pascal-qea9): sibling nested calls, `repeat` loops, `dbf` loops
  with many values live.  Expected values from a Python model of the
  Pascal semantics.

  call: RV3_$SIB l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8
  call: RV3_$REPEAT l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8 l:40
  call: RV3_$DBF l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8
  call: RV3_$CASESEL l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8 w:0
  call: RV3_$CASESEL l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8 w:2
  call: RV3_$CASESEL l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8 w:9
  call: RV3_$ANDTHEN l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8
  warns: best effort (not byte-identical): more values live at once than there are registers
  expect:
    36
    38
    36
    90
    3
    7
    12
    18
    25
    33
    42
    120
    180
    16
    -7
    36
    6
    38
    8
    40
    17
    47
    1
    54
}
module be_spill4;
%include 'exec.ins.pas';

{ nested routines calling each other under pressure, with the static link }
procedure rv3_$sib(a, b, c, d, e, f, g, h: integer32); options(val_param);
var t: integer32;

  procedure leaf(x: integer32); options(val_param);
  begin
    t := t + x;
  end;

  procedure mid(p, q, r, s, u, v, w, y: integer32); options(val_param);
  begin
    exec_$clobber;
    leaf(p + q);
    exec_$clobber;
    leaf(r + s + u);
    t := t + v + w + y + a + h;
    exec_$logv_l(p + q + r + s + u + v + w + y);
    leaf(p - y);
  end;

begin
  t := 0;
  mid(a, b, c, d, e, f, g, h);
  exec_$logv_l(t);
  mid(h, g, f, e, d, c, b, a);
  exec_$logv_l(t);
end;

{ a `repeat` whose condition compares spilled values }
procedure rv3_$repeat(a, b, c, d, e, f, g, h, lim: integer32); options(val_param);
var s: integer32;
begin
  s := 0;
  repeat
    exec_$clobber;
    s := s + a + b;
    a := a + 1;
    exec_$clobber;
    h := h + c - d + e - f + g;
    exec_$logv_l(s);
  until (s > lim) or (h < a);
  exec_$logv_l(a + b + c + d + e + f + g + h + s);
end;

{ a constant count `for` (dbf) with spilled values }
procedure rv3_$dbf(a, b, c, d, e, f, g, h: integer32); options(val_param);
var i: integer;
  s: integer32;
begin
  s := 0;
  for i := 1 to 5 do begin
    exec_$clobber;
    s := s + a + b + c + d + e + f + g + h;
    a := a + i;
    h := h - i;
  end;
  exec_$logv_l(s);
  exec_$logv_l(a);
  exec_$logv_l(h);
  exec_$logv_l(a + b + c + d + e + f + g + h);
end;

{ a `case` on a value live across calls }
procedure rv3_$casesel(a, b, c, d, e, f, g, h: integer32; k: integer); options(val_param);
begin
  exec_$clobber;
  a := a + b;
  exec_$clobber;
  case k of
    0: exec_$logv_l(a + c);
    1: exec_$logv_l(b + d);
    2: exec_$logv_l(c + e);
    3: exec_$logv_l(d + f);
    4: exec_$logv_l(e + g);
    otherwise exec_$logv_l(h + k);
  end;
  exec_$clobber;
  exec_$logv_l(a + b + c + d + e + f + g + h + k);
end;

{ short-circuit conditions on spilled values }
procedure rv3_$andthen(a, b, c, d, e, f, g, h: integer32); options(val_param);
var n: integer;
begin
  n := 0;
  exec_$clobber;
  while (a < h) and then (b < g) do begin
    exec_$clobber;
    a := a + c;
    b := b + d;
    if (e > f) or else (a = b) then n := n + 1;
    e := e + 2;
  end;
  exec_$logv_w(n);
  exec_$logv_l(a + b + c + d + e + f + g + h);
end;
