{ Execution test: pointer locals assigned a pointer function's result
  that no call follows, held in A0 itself from the call to their last
  dereference (bead domain-pascal-78n2; codegen-patterns 3, "Pointer
  locals held in D0"; TTY_$I_INQ_RAW 0xE67414).  Each routine puts other
  uses of the address registers between the call and the local's last
  use: out parameters and var parameters (A1, callee-saved), a second
  pointer (another global pointer, a `with' of it, a chain of
  dereferences), an indexed global, a small set, a word `case', a
  `while' loop, a block copy between frame and global data, a nested
  routine's call after the last use, `disable'/`enable', an `and then',
  a function result in D0 and in A0, an M$ARITH call after the last use.
  The local must keep its value whatever register holds it; SHOW logs
  the data after each call and puts it back (desc a, b; h a, b; g;
  the three nodes; arr[2]).  Not expressed (dpc refuses them, not a
  failure): an M$ARITH call before the last use, a block copy through
  the local, two calls in a statement.

  call: PTR_A0_$RESET
  call: PTR_A0_$OUTP l:1 @l:99! @l:0! @l:0!
  call: PTR_A0_$SHOW
  call: PTR_A0_$VARREC l:1 @l:99! @l:5!
  call: PTR_A0_$SHOW
  call: PTR_A0_$IFELSE l:1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$IFELSE l:-1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$WITH l:1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$ARRINDEX l:1 w:2 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$TWOQ l:1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$FUNC l:1 @l:99! -> l
  call: PTR_A0_$SHOW
  call: PTR_A0_$PTRFUNC l:1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$ADDROF l:1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$WITHOTHER l:1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$CHAIN2 l:1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$CHAIN3 l:1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$HPSTORE l:1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$MARITH3 l:1 l:5 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$SETSMALL l:1 w:5 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$CASEW l:1 w:2 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$CASEW l:1 w:3 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$WHILEQ l:1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$GLOBCOPY l:1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$NESTED l:1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$DISABLE l:1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$SHORTC l:1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$SHORTC l:-1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$DEREF2 l:1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$BYADDR l:1 @l:99!
  call: PTR_A0_$SHOW
  call: PTR_A0_$WHOLE l:1 @l:99!
  call: PTR_A0_$SHOW
  link: park_int_desc.mod.pas
  expect:
    out 0
    out 7
    out 10
    7
    10
    0
    4
    3
    1
    2
    3
    22
    out 0
    out 8
    18
    10
    0
    4
    3
    1
    2
    3
    22
    out 0
    8
    10
    0
    4
    3
    1
    2
    3
    22
    out 1
    7
    0
    0
    4
    3
    1
    2
    3
    22
    out 0
    13
    13
    0
    4
    3
    1
    2
    3
    22
    out 0
    7
    33
    0
    4
    3
    1
    2
    3
    7
    out 0
    11
    11
    0
    4
    3
    1
    2
    3
    22
    out 0
    => 17
    7
    10
    0
    4
    3
    1
    2
    3
    22
    out 0
    1
    10
    0
    4
    3
    1
    2
    3
    22
    out 0
    7
    10
    7
    4
    3
    1
    2
    3
    22
    out 0
    7
    5
    7
    2
    3
    1
    2
    3
    22
    out 0
    7
    10
    0
    4
    3
    8
    7
    3
    22
    out 0
    7
    10
    0
    4
    3
    8
    2
    7
    22
    out 0
    11
    10
    10
    2
    3
    1
    2
    3
    22
    out 0
    7
    1
    0
    4
    15
    1
    2
    3
    22
    out 0
    1
    2
    0
    4
    3
    1
    2
    3
    22
    out 0
    30
    10
    0
    4
    3
    1
    2
    3
    22
    out 0
    7
    40
    0
    4
    3
    1
    2
    3
    22
    out 0
    117
    10
    0
    4
    3
    1
    2
    3
    22
    out 0
    66
    3
    0
    4
    3
    1
    2
    3
    22
    out 0
    9
    10
    0
    4
    1
    1
    2
    3
    22
    out 0
    9
    10
    0
    4
    3
    1
    2
    3
    22
    out 0
    7
    7
    0
    4
    3
    1
    2
    3
    22
    out 1
    7
    10
    0
    4
    3
    1
    2
    3
    22
    out 0
    4
    10
    10
    4
    3
    1
    2
    3
    22
    7
    out 0
    7
    10
    0
    4
    3
    1
    2
    3
    22
    out 0
    3
    10
    0
    4
    3
    1
    2
    3
    22
}
module ptr_a0;
%include 'exec.ins.pas';

type
  desc_t = record
    a: integer32;
    b: integer32;
  end;
  desc_ptr_t = ^desc_t;
  big_t = record
    x: array[1..6] of integer32;
  end;
  node_ptr_t = ^node_t;
  node_t = record
    v: integer32;
    next: node_ptr_t;
  end;
  s32_t = set of 0..31;
var
  park_int_$desc: extern desc_t;
  g: integer32 := 0;
  h: desc_t;
  hp: desc_ptr_t := addr(h);
  bg: big_t;
  n1, n2, n3: node_t;
  np: node_ptr_t := addr(n1);
  arr: array[1..4] of integer32;

function park_int_$getd(in k: integer32; out st: integer32): desc_ptr_t;
  options(extern);

function ptr_a0_$getn(in k: integer32; out st: integer32): node_ptr_t;
begin
  st := 0;
  ptr_a0_$getn := np;
end;

procedure ptr_a0_$reset;
begin
  { the higher field first: a small longword constant stored below the
    next statement's nonzero one is refused (codegen-patterns 13; bead
    domain-pascal-53xi) }
  park_int_$desc.b := 10;
  park_int_$desc.a := 7;
  h.b := 4;
  h.a := 0;
  hp := addr(h);
  g := 3;
  n1.v := 1;
  n1.next := addr(n2);
  n2.v := 2;
  n2.next := addr(n3);
  n3.v := 3;
  { the higher field first: a small longword constant stored below the
    next statement's nonzero one is refused (codegen-patterns 13; bead
    domain-pascal-53xi) }
  arr[4] := 44;
  arr[3] := 33;
  arr[2] := 22;
  arr[1] := 11;
  bg.x[2] := 66;
end;

procedure ptr_a0_$show;
begin
  exec_$logv_l(park_int_$desc.a);
  exec_$logv_l(park_int_$desc.b);
  exec_$logv_l(h.a);
  exec_$logv_l(h.b);
  exec_$logv_l(g);
  exec_$logv_l(n1.v);
  exec_$logv_l(n2.v);
  exec_$logv_l(n3.v);
  exec_$logv_l(arr[2]);
  ptr_a0_$reset;
end;

procedure ptr_a0_$outp(k: integer32; out st: integer32; out r: integer32; out r2: integer32); options(val_param);
var q: desc_ptr_t;
begin
  q := park_int_$getd(k, st);
  r := q^.a;
  r2 := q^.b;
end;

procedure ptr_a0_$varrec(k: integer32; out st: integer32; var d: integer32); options(val_param);
var q: desc_ptr_t;
begin
  q := park_int_$getd(k, st);
  d := 1;
  d := d + q^.a;
  q^.a := d + q^.b;
end;

procedure ptr_a0_$ifelse(k: integer32; out st: integer32); options(val_param);
var q: desc_ptr_t;
begin
  q := park_int_$getd(k, st);
  if st = 0 then
    q^.a := q^.a + 1
  else
    q^.b := 0;
end;

procedure ptr_a0_$with(k: integer32; out st: integer32); options(val_param);
var q: desc_ptr_t;
begin
  q := park_int_$getd(k, st);
  with q^ do begin
    a := b + 3;
    b := a;
  end;
end;

procedure ptr_a0_$arrindex(k: integer32; i: integer; out st: integer32); options(val_param);
var q: desc_ptr_t;
begin
  q := park_int_$getd(k, st);
  arr[i] := q^.a;
  q^.b := arr[i+1];
end;

procedure ptr_a0_$twoq(k: integer32; out st: integer32); options(val_param);
var p, q: desc_ptr_t;
begin
  p := park_int_$getd(k, st);
  q := park_int_$getd(k, st);
  p^.a := q^.b + 1;
  q^.b := p^.a;
end;

function ptr_a0_$func(k: integer32; out st: integer32): integer32; options(val_param);
var q: desc_ptr_t;
begin
  q := park_int_$getd(k, st);
  ptr_a0_$func := q^.a + q^.b;
end;

function ptr_a0_$ptrfunc(k: integer32; out st: integer32): desc_ptr_t; options(val_param);
var q: desc_ptr_t;
begin
  q := park_int_$getd(k, st);
  q^.a := 1;
  ptr_a0_$ptrfunc := hp;
end;

procedure ptr_a0_$addrof(k: integer32; out st: integer32); options(val_param);
var q: desc_ptr_t; p: desc_ptr_t;
begin
  q := park_int_$getd(k, st);
  p := addr(h);
  p^.a := q^.a;
end;

procedure ptr_a0_$withother(k: integer32; out st: integer32); options(val_param);
var q: desc_ptr_t;
begin
  q := park_int_$getd(k, st);
  with hp^ do begin
    a := q^.a;
    b := 2;
  end;
  q^.b := 5;
end;

procedure ptr_a0_$chain2(k: integer32; out st: integer32); options(val_param);
var q: node_ptr_t;
begin
  q := ptr_a0_$getn(k, st);
  q^.next^.v := 7;
  q^.v := 8;
end;

procedure ptr_a0_$chain3(k: integer32; out st: integer32); options(val_param);
var q: node_ptr_t;
begin
  q := ptr_a0_$getn(k, st);
  q^.next^.next^.v := 7;
  q^.v := 8;
end;

procedure ptr_a0_$hpstore(k: integer32; out st: integer32); options(val_param);
var q: desc_ptr_t; t: integer32;
begin
  q := park_int_$getd(k, st);
  t := q^.b;
  hp^.a := t;
  hp^.b := 2;
  q^.a := t + 1;
end;

procedure ptr_a0_$marith3(k: integer32; m: integer32; out st: integer32); options(val_param);
var q: desc_ptr_t;
begin
  q := park_int_$getd(k, st);
  q^.b := 1;
  g := g * m;
end;

procedure ptr_a0_$setsmall(k: integer32; i: integer; out st: integer32); options(val_param);
var q: desc_ptr_t; s: s32_t;
begin
  q := park_int_$getd(k, st);
  s := [i];
  if i in s then q^.a := 1;
  q^.b := 2;
end;

procedure ptr_a0_$casew(k: integer32; i: integer; out st: integer32); options(val_param);
var q: desc_ptr_t;
begin
  q := park_int_$getd(k, st);
  case i of
    0: q^.a := 10;
    1: q^.b := 20;
    2: q^.a := 30;
    3: q^.b := 40;
  end;
end;

procedure ptr_a0_$whileq(k: integer32; out st: integer32); options(val_param);
var q: desc_ptr_t; i: integer;
begin
  q := park_int_$getd(k, st);
  i := 1;
  while i <= 4 do begin
    q^.a := q^.a + arr[i];
    i := i + 1;
  end;
end;

procedure ptr_a0_$globcopy(k: integer32; out st: integer32); options(val_param);
var q: desc_ptr_t; e: big_t;
begin
  q := park_int_$getd(k, st);
  e := bg;
  q^.a := e.x[2];
  bg := e;
  q^.b := 3;
end;

procedure ptr_a0_$nested(k: integer32; out st: integer32); options(val_param);
var q: desc_ptr_t;
  procedure inner; begin g := 1; end;
begin
  q := park_int_$getd(k, st);
  q^.a := 9;
  inner;
end;

procedure ptr_a0_$disable(k: integer32; out st: integer32); options(val_param);
var q: desc_ptr_t;
begin
  q := park_int_$getd(k, st);
  disable;
  q^.a := 9;
  enable;
end;

procedure ptr_a0_$shortc(k: integer32; out st: integer32); options(val_param);
var q: desc_ptr_t;
begin
  q := park_int_$getd(k, st);
  if (st = 0) and then (q^.a > 0) then q^.b := q^.a;
end;

procedure ptr_a0_$deref2(k: integer32; out st: integer32); options(val_param);
var q: desc_ptr_t; r: desc_ptr_t;
begin
  q := park_int_$getd(k, st);
  r := hp;
  r^.a := q^.b;
  q^.a := r^.b;
end;

procedure ptr_a0_$byaddr(k: integer32; out st: integer32); options(val_param);
var q: desc_ptr_t;
begin
  q := park_int_$getd(k, st);
  exec_$log_l(q^.a);
end;

procedure ptr_a0_$whole(k: integer32; out st: integer32); options(val_param);
var q: desc_ptr_t;
begin
  q := park_int_$getd(k, st);
  hp := q;
  hp^.a := 3;
end;
