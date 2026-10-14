{ Execution test: more operations on non-negative 4-byte subranges
  (`linteger = 0..2147483647', unsigned: ref 3.4 p. 3-9, 4.3.3 pp.
  4-6..4-7; codegen-patterns 5; bead domain-pascal-5j9) holding values
  of 2^31 and above (reached by type transfer or wrap-around, ref 3.4's
  NOTE):
  - `u := u - 1' (a signed `-' of an unsigned operand, updated in place)
    from 0 gives 16#FFFFFFFF, above 1000 unsigned;
  - `u + 1' wraps to 0;
  - `div' by a power of two (`lsr.l') and by a word variable (M$DIU$LLW);
  - `*' of two (M$MIU$LLL): the low 32 bits;
  - mixed with an integer32 (ref 4.3.3: signed): 16#FFFFFFFE div 2 = -1,
    and 16#FFFFFFFE > 2 is false (no mark 5);
  - `case' on one (a compare chain), a function result, an uplevel
    reference, a `while' loop and an array index.

  call: ULONG_OPS_$SET l:0 l:-2 l:2 w:65535
  call: ULONG_OPS_$RUN
  expect:
    mark 1
    0xffffffff
    mark 2
    0x7fffffff
    0x00010000
    0x00020001
    0xfffffffc
    -1
    mark 9
    mark 3
    mark 4
    3
    7
}
module ulong_ops;
%include 'exec.ins.pas';

type
  linteger = 0..2147483647;
  uword = 0..65535;
var
  gu, gv: linteger;
  gl: integer32;
  gq: uword;
  gc: integer;
  ga: array[0..3] of integer;

procedure ulong_ops_$set(a: linteger; b: linteger; l: integer32; q: uword);
  options(val_param);
begin
  gu := a;
  gv := b;
  gl := l;
  gq := q;
end;

function big(x: linteger): linteger; options(val_param);
begin
  big := x;
end;

procedure nest;
var
  x: linteger;

  procedure inner;
  begin
    if x > 1000 then exec_$mark(4);
  end;

begin
  x := gv;
  inner;
end;

procedure ulong_ops_$run;
var
  r: integer32;
begin
  gu := gu - 1;
  if gu > 1000 then exec_$mark(1);
  r := gu;
  exec_$log_x(r);
  gu := gu + 1;
  if gu < gv then exec_$mark(2);
  r := gv div 2;
  exec_$log_x(r);
  gu := gv div gq;
  r := gu;
  exec_$log_x(r);
  gu := gq;
  gu := gu + 2;
  gu := gu * gu;
  r := gu;
  exec_$log_x(r);
  gu := gv;
  gu := gu * gl;
  r := gu;
  exec_$log_x(r);
  r := gv div gl;
  exec_$logv_l(r);
  case gv of
    1: gc := 7;
    100: gc := 8;
    otherwise gc := 9;
  end;
  exec_$mark(gc);
  if big(gv) > 1000 then exec_$mark(3);
  nest;
  if gv > gl then exec_$mark(5);
  gu := 0;
  while gu < 3 do gu := gu + 1;
  r := gu;
  exec_$logv_l(r);
  ga[gu] := 7;
  gc := ga[3];
  exec_$logv_w(gc);
end;
