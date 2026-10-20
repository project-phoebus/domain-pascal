{ Execution test: bodies that do not colour, values spilled (bead
  domain-pascal-qea9, crates/dpc-m68k/src/spill.rs; best effort, the
  original has no such spills, bead domain-pascal-ei42).

  - BE_SPILL_$DATA: eight longword parameters live across a call (D2-D7
    hold six): data values in frame slots below the locals.
  - BE_SPILL_$PTRS: five by-address parameters live across calls (A2-A4
    hold three): their addresses loaded again from the parameter slots,
    and dereferenced.
  - BE_SPILL_$WITHS: five `with` records through run-time indices live
    across calls: address values parked in data registers and moved back
    to an address register to be dereferenced; one word parameter in a
    frame slot (a partial write: the slot reloaded first); the indices
    equal, so that two `with`s name one record.
  - BE_SPILL_$LOOP: a function whose eight parameters and running sum are
    live around a call inside a `for` loop; run zero times too.
  - BE_SPILL_$BOTH: data and address pressure together (seven longwords
    and five `with` records across calls), aliased records.
  - BE_SPILL_$NESTED: a nested routine with eight parameters live across
    calls that also reads and writes its parent's variables.

  call: BE_SPILL_$DATA l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8
  call: BE_SPILL_$PTRS @l:1! @l:2! @l:3! @l:4! @l:5!
  call: BE_SPILL_$WITHS w:1 w:2 w:3 w:4 w:5
  call: BE_SPILL_$WITHS w:2 w:2 w:6 w:6 w:1
  call: BE_SPILL_$LOOP l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8 w:3 -> l
  call: BE_SPILL_$LOOP l:1 l:2 l:3 l:4 l:5 l:6 l:7 l:8 w:0 -> l
  call: BE_SPILL_$BOTH l:1 l:2 l:3 l:4 l:5 l:6 l:7 w:1 w:2 w:3 w:4 w:5
  call: BE_SPILL_$BOTH l:100 l:-1 l:-2 l:-3 l:-4 l:-5 l:-6 w:2 w:2 w:6 w:6 w:6
  call: BE_SPILL_$NESTED l:10 l:20
  warns: best effort (not byte-identical): more values live at once than there are registers
  expect:
    36
    -7
    -5
    out 18
    out 4
    out 7
    out 11
    out 16
    30
    50
    75
    10
    50
    100
    100
    121
    9
    10
    37
    75
    114
    => 107
    => -7
    31
    55
    79
    10
    68
    28
    197
    197
    108
    14
    108
    79
    10
    -3
    68
}
module be_spill;
%include 'exec.ins.pas';

type
  rec = record
    a: integer32;
    b: integer;
  end;
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

procedure be_spill_$data(a, b, c, d, e, f, g, h: integer32); options(val_param);
var s: integer32;
begin
  exec_$clobber;
  s := a + b + c + d + e + f + g + h;
  exec_$logv_l(s);
  exec_$logv_l(a - h);
  exec_$logv_l(b - g);
end;

procedure be_spill_$ptrs(var a, b, c, d, e: integer32);
begin
  exec_$clobber;
  a := a + 1;
  b := b + a;
  c := c + b;
  d := d + c;
  e := e + d;
  exec_$clobber;
  a := a + e;
end;

procedure be_spill_$withs(i, j, k, l, m: integer); options(val_param);
begin
  fill;
  with arr[i] : u, arr[j] : v, arr[k] : w, arr[l] : x, arr[m] : y do begin
    exec_$clobber;
    u.a := u.a + v.a;
    exec_$clobber;
    v.a := v.a + w.a;
    w.a := w.a + x.a + y.b;
    exec_$clobber;
    x.b := x.b + y.b + u.b;
    exec_$logv_l(u.a);
    exec_$logv_l(v.a);
    exec_$logv_l(w.a);
    exec_$logv_w(x.b);
    exec_$logv_l(y.a);
  end;
end;

function be_spill_$loop(a, b, c, d, e, f, g, h: integer32; n: integer): integer32;
  options(val_param);
var i: integer;
  s: integer32;
begin
  s := 0;
  for i := 1 to n do begin
    exec_$clobber;
    s := s + a + b + c + d + e + f + g + h + i;
    exec_$logv_l(s);
  end;
  be_spill_$loop := s + a - h;
end;

procedure be_spill_$both(a, b, c, d, e, f, g: integer32; i, j, k, l, m: integer);
  options(val_param);
begin
  fill;
  with arr[i] : u, arr[j] : v, arr[k] : w, arr[l] : x, arr[m] : y do begin
    exec_$clobber;
    u.a := u.a + v.a + a;
    exec_$clobber;
    v.a := v.a + w.a + b + c;
    w.a := w.a + x.a + y.b + d;
    exec_$clobber;
    x.b := x.b + y.b + u.b;
    y.a := y.a + e + f + g;
    exec_$logv_l(u.a);
    exec_$logv_l(v.a);
    exec_$logv_l(w.a);
    exec_$logv_w(x.b);
    exec_$logv_l(y.a);
    exec_$logv_l(a + b + c + d + e + f + g);
  end;
end;

procedure be_spill_$nested(p, q: integer32); options(val_param);
var t: integer32;

  procedure inner(a, b, c, d, e, f, g, h: integer32); options(val_param);
  begin
    exec_$clobber;
    t := t + a + b + c + d + e + f + g + h + p;
    exec_$clobber;
    exec_$logv_l(a - h + q);
    exec_$logv_l(b * 2 - g);
  end;

begin
  t := 1;
  inner(p, 2, 3, 4, 5, 6, 7, q);
  exec_$logv_l(t);
end;
