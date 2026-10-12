{ Golden: codegen-patterns 7, `repeat ... until': the body, then the
  test, branching back to the head while the condition is false.  The
  head is entered by falling through and gets the 0x2048 filler when it
  would start at 2 mod 4 (aligning routines: 312 such heads have the
  filler, 158 are 4-aligned already, 25 are not).  An `or else' test
  branches out of the loop when an operand is true and back from the
  last one (0xE031A2..0xE031B6).  A call in the body is popped: the
  test, a conditional branch, follows it (codegen-patterns 1.1). }
module rpt;
var
  w: extern integer;
  b: extern boolean;
  r: extern integer;

procedure ext_$p(x: integer); options(val_param, extern);

procedure rpt_$one; internal;
begin
  w := 0;
  repeat
    r := 1;
  until w = 3;
end;

procedure rpt_$or_else; internal;
begin
  r := 0;
  repeat
    ext_$p(5);
  until (w >= 10) or else b;
end;
