{ Execution test (bead domain-pascal-5kgg): best-effort arithmetic
  (codegen-patterns 8): a constant or a variable minus a computed value,
  unsigned `mod' by a power of two (`divu.w'/M$ARITH, not a mask), a
  constant of 9..127 added to a held value, an M$ARITH product of two
  computed operands (no calls), a word `mod' a longword (M$OIS$LLL), a
  conditional branch after a call whose pop was skipped (`max' of a
  function's result), a longword constant stored and used again, and a
  negative constant added.  `mod' takes the dividend's sign (ref p. 4-115).

  call: BX_ARITH_$CMINUS w:6 w:7
  call: BX_ARITH_$VMINUS w:3 w:4
  call: BX_ARITH_$UMOD w:50001
  call: BX_ARITH_$LUMOD l:1000003
  call: BX_ARITH_$ADD9 w:-5
  call: BX_ARITH_$BOTH l:9 l:-3
  call: BX_ARITH_$WMODL w:-17 l:5
  call: BX_ARITH_$WMODL w:17 l:-5
  call: BX_ARITH_$UNPOP l:10 -> l
  call: BX_ARITH_$UNPOP l:-10 -> l
  call: BX_ARITH_$SETL2
  call: BX_ARITH_$CSTORE2
  call: BX_ARITH_$NEG w:9
  warns: best effort (not byte-identical): a constant minus a computed value
  warns: best effort (not byte-identical): a variable minus a computed value
  warns: best effort (not byte-identical): an unsigned `mod` by a power of two
  warns: best effort (not byte-identical): an unsigned `div`/`mod` by a power of two
  warns: best effort (not byte-identical): adding a constant of 9..127 to a value held in a register
  warns: best effort (not byte-identical): an M$ARITH call with both operands computed
  warns: best effort (not byte-identical): a word `mod` a longword
  warns: best effort (not byte-identical): a conditional branch after a call whose pop was skipped
  warns: best effort (not byte-identical): a longword constant in `moveq`'s range stored, its value used again
  warns: best effort (not byte-identical): adding or subtracting a negative constant
  expect:
    58
    988
    1
    3
    15
    25
    -10
    -2
    2
    => 10
    => 3
    12
    5
    4
}
module be_x_arith;
%include 'exec.ins.pas';
type uw = 0..65535;
var
  w: integer;
  u: uw;
  l, l2: integer32;
  r: record a: integer32; c: integer32; d: integer32; end;

procedure bx_arith_$cminus(a: integer; c: integer); options(val_param);
begin
  w := 100 - (a * c);
  exec_$logv_w(w);
end;

procedure bx_arith_$vminus(a: integer; c: integer); options(val_param);
begin
  u := 1000;
  w := u - (a * c);
  exec_$logv_w(w);
end;

procedure bx_arith_$umod(a: uw); options(val_param);
begin
  u := a mod 16;
  exec_$logv_w(u);
end;

procedure bx_arith_$lumod(a: integer32); options(val_param);
var x: 0..16#7fffffff;
begin
  x := a;
  l := x mod 64;
  exec_$logv_l(l);
end;

procedure bx_arith_$add9(a: integer); options(val_param);
begin
  w := a + 20;
  exec_$logv_w(w);
  w := a + 30;
  exec_$logv_w(w);
end;

procedure bx_arith_$both(a: integer32; c: integer32); options(val_param);
begin
  l := (a + 1) * (c + 2);
  exec_$logv_l(l);
end;

procedure bx_arith_$wmodl(a: integer; c: integer32); options(val_param);
begin
  l := a mod c;
  exec_$logv_l(l);
end;

function bx_arith_$unpop(a: integer32): integer32; options(val_param);
begin
  bx_arith_$unpop := max(exec_$id_l(a), 3);
end;

procedure bx_arith_$setl2;
begin
  l2 := 7;
end;

procedure bx_arith_$cstore2;
begin
  r.a := 5;
  l := l2 + 5;
  exec_$logv_l(l);
  exec_$logv_l(r.a);
end;

procedure bx_arith_$neg(a: integer); options(val_param);
var k: integer;
begin
  k := a;
  exec_$clobber;
  k := k + (-5);
  exec_$logv_w(k);
end;
