{ codegen-patterns 7.1 ("A computed value against 0"); bead
  domain-pascal-qlzu; `tools/cmpsurvey.py opflags'.  A value an add, sub,
  and, or (or a shift, `not', multiply) just formed in a data register,
  tested against 0:

  - `=', `<>', `<', `>=': the branch reads the operation's own flags
    (SOCK_$ALLOCATE 0xE15EFA `or.w D4,D0; beq.s'; SIO6509_$INQ_LINE
    0xE7267C `andi.l #0x180,D1; beq.s'; tty_$i_put_chars 0xE1B046
    `subq.w #1,D1; bpl.s'), also when the value is a register-held
    local an assignment just formed (MAC_OS_$SEND's nested routine,
    module-local at 0xE0B522, at 0xE0B57C `sub.w D5,D3; bne.b');
  - `>', `<=' after an add or sub: `tst' first (SMD_$SIGNAL 0xE6F25C
    `sub.l D0,D1; tst.l D1; bgt.s'; LOG_$ADD 0xE1766A);
  - a function's result, a quotient, a remainder, a byte widened to a
    word: `tst' (SOCK_$PUT_INT 0xE161EA `tst.w D0; seq D0'; 0xE11BF4
    `divs.w #60,D1; tst.w D1'; SOCK_$CLOSE 0xE15FBE);
  - one bit of a register-held value: `btst' (pacct_$compress 0xE5A9DE
    `btst #2,D0; sne D2').

  A word difference is kept a word by `integer16' (ref "Type Transfer
  Functions", p. 4-198; without it both sides are compared as longwords,
  reference-clarifications 31). }
module czero;
type
  byte_t = [byte] 0..255;
  brec = record a: integer; q: byte_t; end;
  brecp = ^brec;
var
  w: integer;
  r: integer;
  l: integer32;
function getw(k: integer): integer; val_param; extern;

procedure flags;
begin
  if (w ! r) = 0 then r := 0;
  if integer16(w - r) < 0 then r := 1;
  if integer16(w - r) > 0 then r := 2;
  if (l & 16#180) <> 0 then r := 3;
end;

procedure tested(p: brecp); val_param;
begin
  if getw(w) = 0 then r := 0;
  if (w div 60) = 0 then r := 1;
  if (w mod 60) <> 0 then r := 2;
  if p^.q = 0 then r := 3;
end;

function is_zero(k: integer): boolean; val_param;
begin
  is_zero := getw(k) = 0;
end;

function bit2(v: integer32): boolean; val_param;
begin
  bit2 := (v & 4) <> 0;
  l := v;
end;

procedure assigned;
var x: integer;
begin
  x := getw(1);
  x := x - w;
  if x = 0 then r := 0;
  x := x - w;
  if x < 0 then x := x + 256;
  x := x + w;
  if x <= 0 then r := 2;
  r := x;
end;
