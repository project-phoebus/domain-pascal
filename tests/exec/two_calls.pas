{ Execution test: several calls in one statement (bead domain-pascal-ej8m;
  codegen-patterns 1.1, "Calls among the arguments of a call" and "Two
  calls in one statement").  dpc either makes a call where its argument
  is pushed and pops it at once (PROC2_$SIGNAL_PGROUP_OS 0xE3F296), or,
  where an earlier result is still needed when the next call is made,
  selects the statement again with each result copied out of D0 / A0
  into a register of its own (callee-saved when a call follows).  Every
  combination must compute what the source says, whatever D0, D1, A0 and
  A1 hold after each call (the stubs clobber them).

  Covered: two results combined (`+', `-', `*' by M$ARITH, `<' in a
  condition and as a value, `=' of booleans tested by `and then'); a
  call as another call's argument, by value and by address, the inner
  call pushed after other arguments, with its own pad and a byte slot
  next to it; nested three deep; a call in an index and in the value of
  the same store; M$ARITH results combined with each other and with a
  call; a function result whose last statement branches after M$ARITH
  (`div 256', PROC1_$LOADAV_CALLBACK's shape); `or else' / `and then' with a call on
  each side, whose second call runs only when needed (ref "And", p.
  4-18; "Or", p. 4-134; C.3.3, p. C-6: the order of `and then' and `or
  else' operands is the written one).  Calls with side effects are
  combined only where the result does not depend on their order
  (`bump + bump'): the reference fixes no order for other operators.

  call: TWO_CALLS_$ADD @w:5 @w:-7
  call: TWO_CALLS_$ADD @w:32767 @w:1
  call: TWO_CALLS_$SUB @l:100000 @l:-3
  call: TWO_CALLS_$SUB @l:-2147483648 @l:1
  call: TWO_CALLS_$LT @w:3 @w:4
  call: TWO_CALLS_$LT @w:4 @w:3
  call: TWO_CALLS_$LT @w:-1 @w:-1
  call: TWO_CALLS_$LT_VALUE @l:-5 @l:2
  call: TWO_CALLS_$LT_VALUE @l:7 @l:2
  call: TWO_CALLS_$NESTED @w:41
  call: TWO_CALLS_$NESTED @w:-32768
  call: TWO_CALLS_$BY_ADDRESS @w:12 @l:70000
  call: TWO_CALLS_$MIXED @w:300 @w:65 @w:-9
  call: TWO_CALLS_$MIXED @w:-1 @w:255 @w:32767
  call: TWO_CALLS_$INDEX @w:3 @w:77
  call: TWO_CALLS_$INDEX @w:8 @w:-2
  call: TWO_CALLS_$PRODUCTS @l:3 @l:-4 @l:100000 @l:7
  call: TWO_CALLS_$CALL_PRODUCT @l:10 @l:6 @l:-7
  call: TWO_CALLS_$PRODUCT_CALL @l:10 @l:6 @l:-7
  call: TWO_CALLS_$LOADAV @l:65536 @w:3 -> l
  call: TWO_CALLS_$LOADAV @l:-65536 @w:-2 -> l
  call: TWO_CALLS_$OR_ELSE @w:1
  call: TWO_CALLS_$OR_ELSE @w:5
  call: TWO_CALLS_$AND_THEN @w:1
  call: TWO_CALLS_$AND_THEN @w:5
  call: TWO_CALLS_$BUMPS
  call: TWO_CALLS_$BOOLS @b:255 @b:255
  call: TWO_CALLS_$BOOLS @b:255 @b:0
  call: TWO_CALLS_$BOOLS @b:0 @b:255
  call: TWO_CALLS_$RESULT @w:20 @w:22 -> w
  call: TWO_CALLS_$RESULT @w:-20 @w:-30000 -> w
  warns: best effort (not byte-identical): a call whose result is still needed at a later call of its statement
  warns: best effort (not byte-identical): a comparison of a value whose evaluation calls
  warns: best effort (not byte-identical): a comparison of a computed value with one whose evaluation calls
  warns: best effort (not byte-identical): a conditional branch after a call whose pop was skipped
  expect:
    -2
    -32768
    100003
    2147483647
    mark 1
    mark 0
    mark 0
    true
    false
    43
    -32766
    70012
    mark 300
    mark 65
    mark -9
    mark -1
    mark 255
    mark 32767
    74
    -10
    699988
    -32
    -32
    => 66262
    => -65899
    mark 1
    mark 0
    mark 1
    mark 1
    mark 0
    mark 1
    mark 0
    mark 0
    mark 5
    mark 3
    mark 1
    mark 0
    mark 0
    => 42
    => -30020
}
module two_calls;
%include 'exec.ins.pas';

var
  counter: integer := 0;
  ws: array[1..8] of integer;
  ga, gb, gc, gd: integer32;

{ A side effect: the counter's next value. }
function bump: integer;
begin
  counter := counter + 1;
  bump := counter;
end;

{ By value (internal, ref 5.2): a word slot with the pad, a byte slot. }
function wid(w: integer): integer; internal;
begin
  wid := w;
end;

function cid(c: char): char; internal;
begin
  cid := c;
end;

procedure three(a: integer; c: char; b: integer); internal;
begin
  exec_$mark(a);
  exec_$mark(ord(c));
  exec_$mark(b);
end;

procedure two_calls_$add(in a, b: integer);
var
  r: integer;
begin
  r := exec_$id_w(a) + exec_$id_w(b);
  exec_$logv_w(r);
end;

procedure two_calls_$sub(in a, b: integer32);
var
  r: integer32;
begin
  r := exec_$id_l(a) - exec_$id_l(b);
  exec_$logv_l(r);
end;

procedure two_calls_$lt(in a, b: integer);
var
  r: integer;
begin
  if exec_$id_w(a) < exec_$id_w(b) then
    r := 1
  else
    r := 0;
  exec_$mark(r);
end;

procedure two_calls_$lt_value(in a, b: integer32);
var
  t: boolean;
begin
  t := exec_$id_l(a) < exec_$id_l(b);
  exec_$log_bool(t);
end;

procedure two_calls_$nested(in a: integer);
begin
  exec_$logv_w(exec_$id_w(wid(exec_$id_w(a) + 1)) + 1);
end;

procedure two_calls_$by_address(in a: integer; in l: integer32);
begin
  exec_$log_l(exec_$id_l(l) + exec_$id_w(a));
end;

procedure two_calls_$mixed(in a, c, b: integer);
begin
  three(exec_$id_w(a), cid(chr(c)), wid(b));
end;

procedure two_calls_$index(in i, v: integer);
begin
  ws[3] := exec_$id_w(v) - wid(i);
  exec_$logv_w(ws[3]);
end;

procedure two_calls_$products(in a, b, c, d: integer32);
var
  r: integer32;
begin
  ga := a;
  gb := b;
  gc := c;
  gd := d;
  r := ga * gb + gc * gd;
  exec_$logv_l(r);
end;

procedure two_calls_$call_product(in a, b, c: integer32);
var
  r: integer32;
begin
  gb := b;
  gc := c;
  r := exec_$id_l(a) + gb * gc;
  exec_$logv_l(r);
end;

procedure two_calls_$product_call(in a, b, c: integer32);
var
  r: integer32;
begin
  gb := b;
  gc := c;
  r := gb * gc + exec_$id_l(a);
  exec_$logv_l(r);
end;

{ PROC1_$LOADAV_CALLBACK's last statement (0xE14C56): an M$ARITH call
  followed by the branch of `div 256' (popped in the image, 0xE14C76;
  dpc skips the pop, best effort). }
function two_calls_$loadav(in l: integer32; in w: integer): integer32;
begin
  two_calls_$loadav := w * 16#016B + l div 256 * 16#FE95 div 256;
end;

procedure two_calls_$or_else(in a: integer);
var
  r: integer;
begin
  counter := 0;
  if (exec_$id_w(a) = 1) or else (bump = 1) then
    r := 1
  else
    r := 0;
  exec_$mark(r);
  exec_$mark(counter);
end;

procedure two_calls_$and_then(in a: integer);
var
  r: integer;
begin
  counter := 0;
  if (exec_$id_w(a) = 1) and then (bump = 7) then
    r := 1
  else
    r := 0;
  exec_$mark(r);
  exec_$mark(counter);
end;

procedure two_calls_$bumps;
var
  r: integer;
begin
  counter := 1;
  r := bump + bump;
  exec_$mark(r);
  exec_$mark(counter);
end;

procedure two_calls_$bools(in x, y: boolean);
var
  r: integer;
begin
  if exec_$id_bool(x) and then exec_$id_bool(y) then
    r := 1
  else
    r := 0;
  exec_$mark(r);
end;

function two_calls_$result(in a, b: integer): integer;
begin
  two_calls_$result := exec_$id_w(a) + wid(b);
end;
