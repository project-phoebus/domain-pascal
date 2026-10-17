{ Execution test: functions returning records of more than 4 bytes
  through a hidden address (codegen-patterns 3, "Record results over 4
  bytes"; bead domain-pascal-ytu3).  The caller passes the address of a
  frame temporary as a hidden first argument, pushed last with no pad,
  and copies the record out of it; the callee stores through the
  address, on any path, as often as it assigns.  Here: an 8-byte record
  built from a word argument (no pad: 6 bytes of arguments), assigned on
  several paths, assigned to a variable of the unit and to a field
  through a pointer, passed on by address (two in one call, each in its
  own temporary), and a 6-byte record whose argument is its
  destination.

  call: HIDDEN_RESULTS_$BUILD w:7
  call: HIDDEN_RESULTS_$PICK w:0
  call: HIDDEN_RESULTS_$PICK w:3
  call: HIDDEN_RESULTS_$PASS w:5 w:9
  call: HIDDEN_RESULTS_$CLOCK l:0x40000001 w:0x8001
  expect:
    305419896
    7
    17
    34
    17
    34
    51
    68
    17
    34
    5
    -5
    9
    -9
    -2147483645
    2
    5
}
module hidden_results;
%include 'exec.ins.pas';

type
  pair_t = record
    a: integer32;
    b: integer32;
  end;
  holder_t = record
    tag: integer32;
    p: pair_t;
  end;
  clock_t = record
    high: integer32;
    low: 0..65535;
  end;
var
  saved: pair_t;
  holder: holder_t;
  one: pair_t := [a := 17, b := 34];
  three: pair_t := [a := 51, b := 68];

function make(w: integer): pair_t; internal;
var
  r: pair_t;
begin
  r.a := 16#12345678;
  r.b := w;
  make := r;
end;

function choose(in k: integer): pair_t;
begin
  if k = 1 then
    choose := one
  else if k = 3 then
    choose := three
  else if k = 0 then
    choose := one;
end;

function neg(in x: integer): pair_t;
var
  r: pair_t;
begin
  r.a := x;
  r.b := -x;
  neg := r;
end;

procedure log_pair(in p: pair_t);
begin
  exec_$log_l(p.a);
  exec_$log_l(p.b);
end;

procedure log_two(in p: pair_t; in q: pair_t);
begin
  log_pair(p);
  log_pair(q);
end;

procedure hidden_results_$build(w: integer); options(val_param);
var
  u: pair_t;
  v: integer;
begin
  v := w;
  u := make(v);
  log_pair(u);
end;

procedure hidden_results_$pick(k: integer); options(val_param);
var
  hp: ^holder_t;
begin
  hp := addr(holder);
  saved := choose(k);
  log_pair(saved);
  hp^.p := choose(1);
  log_pair(holder.p);
end;

procedure hidden_results_$pass(x, y: integer); options(val_param);
begin
  log_two(neg(x), neg(y));
end;

function dbl(in c: clock_t): clock_t;
var
  r: clock_t;
begin
  r.high := c.high + c.high;
  r.low := c.low + c.low;
  if c.low >= 16#8000 then
    r.high := r.high + 1;
  dbl := r;
end;

procedure hidden_results_$clock(h: integer32; l: integer); options(val_param);
var
  c: clock_t;
  w: integer;
begin
  c.high := h;
  c.low := l;
  c := dbl(c);
  exec_$log_l(c.high);
  w := c.low;
  exec_$log_w(w);
  exec_$log_w(sizeof(clock_t) - 1);
end;
