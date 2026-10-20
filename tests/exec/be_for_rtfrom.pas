{ Execution test (bead domain-pascal-sq4a): `for` loops with a run-time
  lower bound (ref "For", pp. 4-88..4-90; codegen-patterns 7.4,
  "Run-time lower bounds"): `to` into a value of its own, `sub.w from,T;
  blt` (`bcs` for an unsigned index), the count copied into a counter and
  the index loaded from `from`.  Bounds from value parameters, a sum, a
  call's result, the unit's data and a record element (KBD's
  module-local 0xE1CA10 form); loops that run 0 times, once and several
  times, and 40001 times (`to - from` above 32767); negative bounds; a call in the body; arrays reached by the index
  (indexed, not stepped); an unsigned index; a volatile lower bound (a
  counter that steps at each read), read once into a register.

  call: BF_$SUM w:3 w:6
  call: BF_$SUM w:5 w:5
  call: BF_$SUM w:6 w:3
  call: BF_$SUM w:-4 w:2
  call: BF_$COUNT w:-20000 w:20000
  call: BF_$CALLS w:2 w:4
  call: BF_$CALLS w:4 w:2
  call: BF_$ARRAYS w:2 w:4
  call: BF_$ARRAYS w:4 w:4
  call: BF_$PLUS w:1
  call: BF_$PLUS w:5
  call: BF_$PLUS w:-2
  call: BF_$FN w:3 w:5
  call: BF_$TABLE w:1
  call: BF_$TABLE w:2
  call: BF_$UNSIGNED w:40000 w:40003
  call: BF_$UNSIGNED w:40003 w:40000
  call: BF_$UNSIGNED w:0 w:65535
  call: BF_$VOL
  warns: best effort (not byte-identical): a `for` loop with a run-time lower bound
  warns: best effort (not byte-identical): a `for` loop with a run-time lower bound over an array reached by its index
  expect:
    18
    5
    0
    -7
    40001
    mark 2
    mark 3
    mark 4
    mark 99
    mark 99
    9
    -18
    91
    4
    -8
    46
    14
    0
    14
    12
    4
    12
    5
    4
    0
    65536
    mark 100
    mark 101
    mark 102
    mark 103
}
module be_for_rtfrom;
%include 'exec.ins.pas';
type
  uw = 0..65535;
  span = record lo: integer; hi: integer; end;
var
  w: integer;
  lw: integer32;
  ws: array[1..8] of integer;
  ls: array[1..8] of integer32;
  lo_v: integer;
  tbl: array[1..2] of span;
  n: integer32;

procedure bf_$sum(lo: integer; hi: integer); options(val_param);
var i: integer;
begin
  w := 0;
  for i := lo to hi do w := w + i;
  exec_$logv_w(w);
end;

procedure bf_$count(lo: integer; hi: integer); options(val_param);
var i: integer;
begin
  n := 0;
  for i := lo to hi do n := n + 1;
  exec_$logv_l(n);
end;

procedure bf_$calls(lo: integer; hi: integer); options(val_param);
var i: integer;
begin
  for i := lo to hi do exec_$mark(i);
  exec_$mark(99);
end;

procedure bf_$arrays(lo: integer; hi: integer); options(val_param);
var i: integer;
begin
  for i := 1 to 8 do begin
    ws[i] := i;
    ls[i] := -2 * i;
  end;
  w := 0;
  lw := 0;
  for i := lo to hi do begin
    w := w + ws[i];
    lw := lw + ls[i];
  end;
  exec_$logv_w(w);
  exec_$logv_l(lw);
  { The elements written through the index. }
  for i := lo to hi do ws[i] := 10 * i;
  exec_$logv_w(ws[1] + ws[2] + ws[3] + ws[4]);
end;

procedure bf_$plus(lo: integer); options(val_param);
var i: integer;
begin
  w := 0;
  for i := lo + 1 to 5 do w := w + i;
  exec_$logv_w(w);
end;

procedure bf_$fn(lo: integer; hi: integer); options(val_param);
var i: integer;
begin
  lo_v := lo;
  w := 0;
  for i := exec_$id_w(lo) to hi do w := w + i;
  exec_$logv_w(w);
  w := 0;
  for i := lo_v to hi + 1 do w := w + 1;
  exec_$logv_w(w);
end;

procedure bf_$table(k: integer); options(val_param);
var i: integer;
begin
  tbl[1].lo := 3;
  tbl[1].hi := 6;
  tbl[2].lo := 2;
  tbl[2].hi := 4;
  w := 0;
  for i := tbl[k].lo to tbl[k].hi - 1 do w := w + i;
  exec_$logv_w(w);
end;

procedure bf_$unsigned(lo: uw; hi: uw); options(val_param);
var i: uw;
begin
  n := 0;
  for i := lo to hi do n := n + 1;
  exec_$logv_l(n);
end;

{ `exec_$counter` reads 100, 101, ... after the store: the bound read
  twice would start the index at 101. }
procedure bf_$vol;
var i: integer;
begin
  exec_$counter := 100;
  for i := exec_$counter to 103 do exec_$mark(i);
end;
