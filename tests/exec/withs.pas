{ Execution test: `with' (ref "With", pp. 4-211..4-214; C.3.18) over a
  variable, a pointer, an array element, an element through a pointer, a
  field of an earlier record (`with o, sub'), the alias form `with r : u'
  and a pointer parameter; `addr(x)' as a value stored in a pointer and
  passed by value (ref "Addr", p. 4-13).  The record is "evaluated only
  once" (p. 4-212): assigning the pointer or the index inside the body
  does not move the references ("changing p does not affect access to
  the record identified by the with statement"), and the record address
  survives a call (codegen-patterns 5.2, "`with`").

  Not expressible yet: `nil' (refused as a constant), adjacent stores of
  two byte, two word or a word and a longword constants (refused: they may
  be merged, ref 3.15.3; codegen-patterns 13).

  call: WITHS_$RUN
  expect:
    5
    70000
    122
    1
    2
    0
    7
    8
    0
    9
    10
    113
    3
    4
    11
    mark 6
    -6
}
module withs;
%include 'exec.ins.pas';

type
  rec = record
    a: integer;
    b: integer32;
    c: char;
  end;
  rec_p = ^rec;
  inner = record
    x: integer;
    pad: integer32;
    y: integer;
  end;
  outer = record
    pad: integer;
    sub: inner;
  end;
  recs = array[1..4] of rec;
  recs_p = ^recs;

var
  g: rec;
  r1: rec;
  r2: rec;
  p: rec_p;
  arr: recs;
  ap: recs_p;
  o: outer;
  idx: integer;

procedure negate(q: rec_p); options(val_param);
begin
  with q^ do begin
    exec_$mark(a);
    b := -a;
  end;
  idx := 0;
end;

procedure withs_$run;
begin
  with g do begin
    a := 5;
    c := 'z';
    b := 70000;
  end;
  exec_$log_w(g.a);
  exec_$log_l(g.b);
  exec_$log_c(g.c);

  p := addr(r1);
  with p^ do begin
    a := 1;
    p := addr(r2);
    b := 2;
  end;
  exec_$log_w(r1.a);
  exec_$log_l(r1.b);
  exec_$log_l(r2.b);

  idx := 2;
  with arr[idx] do begin
    a := 7;
    idx := 3;
    b := 8;
  end;
  exec_$log_w(arr[2].a);
  exec_$log_l(arr[2].b);
  exec_$log_l(arr[3].b);

  ap := addr(arr);
  idx := 4;
  with ap^[idx] do begin
    a := 9;
    idx := 1;
    exec_$clobber;
    b := a + 1;
    c := 'q';
  end;
  exec_$log_w(arr[4].a);
  exec_$log_l(arr[4].b);
  exec_$log_c(arr[4].c);

  with o, sub do begin
    x := 3;
    y := x + 1;
  end;
  exec_$log_w(o.sub.x);
  exec_$log_w(o.sub.y);

  with r1 : u, r2 : v do
    v.a := u.a + 10;
  exec_$log_w(r2.a);

  r2.a := 6;
  negate(addr(r2));
  exec_$log_l(r2.b);
end;
