{ Execution test: a comparison of a word sum or difference.  Ref 4.3.1
  (p. 4-5): "The compiler computes operands smaller than 32 bits with 32
  bits of precision when necessary to achieve correct arithmetic"; ref
  "Type Transfer Functions" (p. 4-198): for `x + y > 5' on integer16 x,
  y "the compiler expands operands x and y to 32-bit integers and
  performs 32-bit addition before making the comparison", and
  `INTEGER16(x + y) > 5' is the way to ask for the 16-bit one, which
  "ignores the possibility of integer overflow".  So 32767 + 1 > 5, and
  -32768 - 1 < -32768.

  xfail: dpc compares `w + b' in 16 bits (`add.w; cmpi.w'), wrapping
  call: WIDE_COMPARE_$MAIN w:32767 w:1
  call: WIDE_COMPARE_$MAIN w:-32768 w:0
  call: WIDE_COMPARE_$MAIN w:-32768 w:1
  expect:
    mark 1
    mark 2
    mark 3
    mark 2
}
module wide_compare;
%include 'exec.ins.pas';

var
  w: integer;

procedure wide_compare_$main(a: integer; b: integer); options(val_param);
begin
  w := a;
  if w + b > 5 then exec_$mark(1);
  if w - b < -32768 then exec_$mark(3);
  if w + b < 1 then exec_$mark(2);
end;
