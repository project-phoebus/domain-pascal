{ Execution test, review of bead domain-pascal-xm5w: `for' loops stepping
  a pointer through arrays of the unit's data, the frame and another unit
  (codegen-patterns 7.4, "Forming"; ref "For", pp. 4-88..4-90; "With",
  pp. 4-211..4-214): a negative lower bound (`lea (-12,A5)'), run-time
  counts that run zero times, part of the range, `downto' (in place,
  stepping back), 6- and 10-byte elements, nested `with' over a stepped
  element, a `with' inside an inner loop over the outer element, a `with'
  around an inner loop, two `with's of one element, `with' and direct
  references together, `exit' and `next', nested loops, frame arrays at
  `lea' and in place (from 3: `lea (12,A0),A0'), an element passed by
  address (`pea (d,Ap)'), the index read with a run-time count and a
  call, one pass (count `clr.w').  Could not express (refused): an
  in-place pointer from a negative start (another unit's array from -2),
  an in-place pointer with another stepped pointer.  Expected values are
  the Pascal semantics of each routine.

  link: stepped_data_more_ext.mod.pas
  call: SDM_$SETUP
  call: SDM_$NEG -> l
  call: SDM_$SUM6 @w:0 -> l
  call: SDM_$SUM6 @w:10 -> l
  call: SDM_$PART -> l
  call: SDM_$DOWN
  call: SDM_$NESTWITH -> l
  call: SDM_$NESTED -> l
  call: SDM_$EXITNEXT -> l
  call: SDM_$WITHLOOP -> l
  call: SDM_$FAR @w:0 -> l
  call: SDM_$FAR @w:30 -> l
  call: SDM_$FRAME @w:0 -> l
  call: SDM_$FRAME @w:5 -> l
  call: SDM_$EXTDOWN -> l
  call: SDM_$RESET
  call: SDM_$INNERWITH -> l
  call: SDM_$MIXED -> l
  call: SDM_$FARDOWN -> l
  call: SDM_$K12 -> l
  call: SDM_$RUNIDX @w:3 -> l
  call: SDM_$ONE_PASS -> l
  call: SDM_$TWOWITH -> l
  expect:
    => -8988
    => 0
    => 550
    => 250
    10
    9
    8
    7
    6
    5
    4
    3
    2
    1
    => 1515
    => 15055
    => 20
    => 1200
    => 0
    => 1395
    => 3000
    7
    14
    21
    28
    35
    => 3105
    6
    5
    4
    3
    2
    => -55
    => 34
    => 79
    => 4960
    => 663
    => 6
    => 7
    => 134
}
module sdm;
%include 'exec.ins.pas';
type
  e6 = record a: integer; b: integer; c: integer; end;
  e10 = record a, b, c, d, e: integer; end;
  inner = record x: integer; y: integer; end;
  outer = record id: integer; sub: inner; end;
var
  neg: array[-3..4] of integer32;
  t6: array[1..10] of e6;
  o6: array[1..5] of outer;
  pad: array[1..100] of integer32;
  big: array[1..30] of e10;
  sdm_$x: extern array[-2..5] of e6;

procedure sdm_$setup;
var i: integer;
begin
  for i := -3 to 4 do
    neg[i] := i * 3;
  for i := 1 to 10 do
    with t6[i] do begin
      a := i; b := i * 10; c := 0;
    end;
  for i := 1 to 5 do
    with o6[i] do begin
      sub.x := 0; id := i * 100; sub.y := 1;
    end;
  for i := 1 to 5 do
    with sdm_$x[i] do begin
      a := i; b := -i; c := i + 1;
    end;
  sdm_$x[-2].a := -2; sdm_$x[-1].a := -1; sdm_$x[0].a := 0;
  sdm_$x[-2].c := -1; sdm_$x[-1].c := 0; sdm_$x[0].c := 1;
  sdm_$x[-2].b := 2; sdm_$x[-1].b := 1; sdm_$x[0].b := 0;
end;

{ neg[i] = 3i: 3 * (-3 + ... + 4) = 12, plus neg[-3] * 1000 = -9000 }
function sdm_$neg: integer32;
var i: integer; s: integer32;
begin
  s := 0;
  for i := -3 to 4 do
    s := s + neg[i];
  sdm_$neg := s + neg[-3] * 1000;
end;

{ run-time count, zero-trip: 0, then 550 }
function sdm_$sum6(in n: integer): integer32;
var i: integer; s: integer32;
begin
  s := 0;
  for i := 1 to n do begin
    s := s + t6[i].b;
    exec_$clobber;
  end;
  sdm_$sum6 := s;
end;

{ part of the range: 30+40+50+60+70 = 250 }
function sdm_$part: integer32;
var i: integer; s: integer32;
begin
  s := 0;
  for i := 3 to 7 do
    s := s + t6[i].b;
  sdm_$part := s;
end;

{ downto: logs 10 .. 1; c := a }
procedure sdm_$down;
var i: integer;
begin
  for i := 10 downto 1 do
    with t6[i] do begin
      exec_$logv_w(a);
      c := a;
    end;
end;

{ nested with: sub.x = i, sub.y = id + i; sum of y = 1500 + 15 = 1515 }
function sdm_$nestwith: integer32;
var i: integer; s: integer32;
begin
  for i := 1 to 5 do
    with o6[i] do
      with sub do begin
        x := i;
        y := id + x;
      end;
  s := 0;
  for i := 1 to 5 do
    s := s + o6[i].sub.y;
  sdm_$nestwith := s;
end;

{ nested loops: t6[j].c += o6[i].id for all i: each c = j (from sdm_$down)
  + 1500; sum 55 + 15000 }
function sdm_$nested: integer32;
var i, j: integer; s: integer32;
begin
  for i := 1 to 5 do
    for j := 1 to 10 do
      t6[j].c := t6[j].c + o6[i].id;
  s := 0;
  for j := 1 to 10 do
    s := s + t6[j].c;
  sdm_$nested := s;
end;

{ exit/next: skip a = 3 and 5, stop at a > 7: 1+2+4+6+7 = 20 }
function sdm_$exitnext: integer32;
var i: integer; s: integer32;
begin
  s := 0;
  for i := 1 to 10 do begin
    if (t6[i].a = 3) or (t6[i].a = 5) then next;
    if t6[i].a > 7 then exit;
    s := s + t6[i].a;
  end;
  sdm_$exitnext := s;
end;

{ a with over a loop: o6[i].sub.x = 10 * (1+2+3+4) = 100 for i = 1..3,
  id := sub.x: sum of ids 300 + 400 + 500 = 1200 }
function sdm_$withloop: integer32;
var i, j: integer; s: integer32;
begin
  for i := 1 to 3 do
    with o6[i] do begin
      sub.x := 0;
      for j := 1 to 4 do
        sub.x := sub.x + t6[j].b;
      id := sub.x;
      exec_$clobber;
    end;
  s := 0;
  for i := 1 to 5 do
    s := s + o6[i].id;
  sdm_$withloop := s;
end;

{ far (in place) with a with and a run-time count: e = 2i, a = i:
  0, then 3 * 465 = 1395 }
function sdm_$far(in n: integer): integer32;
var i: integer; s: integer32;
begin
  for i := 1 to n do
    with big[i] do begin
      a := i;
      e := i * 2;
      exec_$clobber;
    end;
  s := 0;
  for i := 1 to n do begin
    s := s + big[i].a + big[i].e;
    exec_$clobber;
  end;
  sdm_$far := s;
end;

{ frame arrays, negative lower bound (near) and far run-time, an
  element passed by address: logs fa[1..n] (= 7i), returns sum of
  nr[-2..3] (= -2..3 sum 3) * 1000 + sum of fa }
function sdm_$frame(in n: integer): integer32;
var
  nr: array[-2..3] of integer32;
  fa: array[1..40] of integer32;
  i: integer;
  s: integer32;
begin
  for i := -2 to 3 do
    nr[i] := i;
  for i := 1 to n do begin
    fa[i] := i * 7;
    exec_$clobber;
  end;
  s := 0;
  for i := -2 to 3 do
    s := s + nr[i] * 1000;
  for i := 1 to n do begin
    exec_$log_l(fa[i]);
    s := s + fa[i];
  end;
  sdm_$frame := s;
end;

{ another unit's array (from -2), downto over 5..1 (in place from 5):
  logs c = 6 .. 2; returns the sum of a * b = -(25 + 16 + 9 + 4 + 1) }
function sdm_$extdown: integer32;
var i: integer; s: integer32;
begin
  s := 0;
  for i := 5 downto 1 do begin
    exec_$logv_w(sdm_$x[i].c);
    s := s + sdm_$x[i].a * sdm_$x[i].b;
  end;
  sdm_$extdown := s;
end;

{ b := 10i and c := 0 again for the routines below. }
procedure sdm_$reset;
var i: integer;
begin
  for i := 1 to 10 do
    with t6[i] do begin
      b := i * 10;
      c := 0;
    end;
end;

{ with over the outer element inside an inner loop: c := c + j for j = 1..3
  for i = 1..4: each c = 6, sum over i of a + c = 10 + 24 = 34 }
function sdm_$innerwith: integer32;
var i, j: integer; s: integer32;
begin
  s := 0;
  for i := 1 to 4 do begin
    for j := 1 to 3 do
      with t6[i] do
        c := c + j;
    s := s + t6[i].a + t6[i].c;
  end;
  sdm_$innerwith := s;
end;

{ with and a direct reference together: b := a + c (c = 6 for 1..4, else 0);
  sum of b = 55 + 24 = 79 }
function sdm_$mixed: integer32;
var i: integer; s: integer32;
begin
  s := 0;
  for i := 1 to 10 do
    with t6[i] do begin
      t6[i].b := a + t6[i].c;
      s := s + b;
    end;
  sdm_$mixed := s;
end;

{ downto in place: big[i].c := 31 - i (the k-th pass sets k); returns the
  sum of i * (31 - i) over 1..30 = 31 * 465 - 9455 }
function sdm_$fardown: integer32;
var i: integer; s, k: integer32;
begin
  k := 0;
  for i := 30 downto 1 do begin
    k := k + 1;
    big[i].c := k;
  end;
  s := 0;
  for i := 1 to 30 do
    s := s + big[i].c * i;
  sdm_$fardown := s;
end;

{ frame in place, from 3 (k = 12): fa[i] := i for 3..36, sum = 666 - 3 }
function sdm_$k12: integer32;
var
  fa: array[1..40] of integer32;
  i: integer;
  s: integer32;
begin
  for i := 3 to 36 do
    fa[i] := i;
  s := 0;
  for i := 3 to 36 do
    s := s + fa[i];
  sdm_$k12 := s;
end;

{ run-time count with a call and the index: big[i].e := i; returns the sum }
function sdm_$runidx(in n: integer): integer32;
var i: integer; s: integer32;
begin
  s := 0;
  for i := 1 to n do begin
    big[i].e := i;
    exec_$clobber;
  end;
  for i := 1 to n do begin
    s := s + big[i].e;
    exec_$clobber;
  end;
  sdm_$runidx := s;
end;

{ one pass: the count `clr.w'; t6[7].b = 7 + 0 (sdm_$mixed) }
function sdm_$one_pass: integer32;
var i: integer; s: integer32;
begin
  s := 0;
  for i := 7 to 7 do
    s := s + t6[i].b;
  sdm_$one_pass := s;
end;

{ two with statements on one element in a body }
function sdm_$twowith: integer32;
var i: integer; s: integer32;
begin
  s := 0;
  for i := 1 to 10 do begin
    with t6[i] do
      s := s + a;
    with t6[i] do
      s := s + b;
  end;
  sdm_$twowith := s;
end;
