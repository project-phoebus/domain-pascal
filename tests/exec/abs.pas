{ Execution test (bead domain-pascal-g6xa): `abs' of run-time integers
  ("Abs", p. 4-12: "the number if it is nonnegative, and the negative of
  the number if it is negative"); codegen-patterns 8, "abs".  The value
  is formed in a register, `bpl' over `neg', and has a non-negative type
  (`0..32767' in a word, `0..2147483647'), unsigned as the image treats
  it: compared unsigned with constants (TPAD_$DATA 0xE69674 `bls',
  TIME_$ADJUST_TIME_OF_DAY 0xE168FA `bls') and unsigned longwords
  (LOCATE_SERVER 0xE4A452 `bls'), signed with a signed word at word width
  (0xE69250 `bge'), zero-extended into longword sums (0xE69284,
  0xE6976A).  The most negative value gives itself (`neg' of 16#8000 is
  16#8000): 32768 against a constant or in a longword sum, -32768 against
  a signed word; 16#80000000 above any constant.  Zero, positive and
  negative values, the word bounds, a product's and a quotient's
  `abs' (0xE69626 on the `muls.w' flags, 0xE6966E after `tst.w'),
  an argument passed by value (PROC2_$WAIT 0xE3FE5E).  The best-effort
  forms (no image site; codegen-patterns 8, "abs"): `abs' of an unsigned
  word (itself), of a signed byte (widened first: abs(-128) = 128), an
  `abs' value stored or returned as it is (an `integer' result keeps the
  low word, so abs(-32768) stays 16#8000, read -32768: no range check,
  Appendix D, ISO 6.4.6, p. D-3), `c - abs(x)', a word difference's
  `abs' against a longword (computed in 32 bits: 30000 - (-30000) is
  60000, not the wrapped word), a quotient's `abs' widened.

  call: ABS_$LT_SIGNED w:-3 w:4 -> w
  call: ABS_$LT_SIGNED w:-4 w:4 -> w
  call: ABS_$LT_SIGNED w:3 w:4 -> w
  call: ABS_$LT_SIGNED w:0 w:0 -> w
  call: ABS_$LT_SIGNED w:-32768 w:0 -> w
  call: ABS_$GT_CONST w:6 -> w
  call: ABS_$GT_CONST w:-6 -> w
  call: ABS_$GT_CONST w:5 -> w
  call: ABS_$GT_CONST w:-5 -> w
  call: ABS_$GT_CONST w:0 -> w
  call: ABS_$GT_CONST w:-32768 -> w
  call: ABS_$GT_CONST w:32767 -> w
  call: ABS_$SUM w:3 w:-4 -> l
  call: ABS_$SUM w:-32768 w:-32768 -> l
  call: ABS_$SUM w:32767 w:-32767 -> l
  call: ABS_$PLUS10 w:-7 -> l
  call: ABS_$PLUS10 w:-32768 -> l
  call: ABS_$PRODUCTS w:-3 w:5 w:-2 w:-7 -> w
  call: ABS_$PRODUCTS w:0 w:5 w:2 w:0 -> w
  call: ABS_$QUOT w:-12 w:2 -> w
  call: ABS_$QUOT w:-10 w:2 -> w
  call: ABS_$QUOT w:11 w:-2 -> w
  call: ABS_$L_CONST l:-8001 -> w
  call: ABS_$L_CONST l:-8000 -> w
  call: ABS_$L_CONST l:8001 -> w
  call: ABS_$L_CONST l:0 -> w
  call: ABS_$L_CONST l:0x80000000 -> w
  call: ABS_$L_CONST l:0x7fffffff -> w
  call: ABS_$L_DIFF l:100 l:350 l:250 -> w
  call: ABS_$L_DIFF l:350 l:100 l:249 -> w
  call: ABS_$L_DIFF l:100 l:100 l:0 -> w
  call: ABS_$ARG w:-9
  call: ABS_$ARG w:9
  call: ABS_$ARG w:0
  call: ABS_$ARG w:-32768
  call: ABS_$UNSIGNED w:40000 -> l
  call: ABS_$UNSIGNED w:0 -> l
  call: ABS_$BYTE b:128 -> w
  call: ABS_$BYTE b:251 -> w
  call: ABS_$BYTE b:5 -> w
  call: ABS_$STORE w:-7 -> w
  call: ABS_$STORE w:7 -> w
  call: ABS_$STORE w:-32768 -> w
  call: ABS_$L_STORE l:-9 -> l
  call: ABS_$L_STORE l:0x80000000 -> l
  call: ABS_$GLOBAL w:-1234
  call: ABS_$C_MINUS w:3 w:10 -> w
  call: ABS_$C_MINUS w:10 w:3 -> w
  call: ABS_$C_MINUS w:0 w:0 -> w
  call: ABS_$W_DIFF w:30000 w:-30000 l:59999 -> w
  call: ABS_$W_DIFF w:30000 w:-30000 l:60000 -> w
  call: ABS_$W_DIFF w:-5 w:5 l:9 -> w
  call: ABS_$W_DIFF w:5 w:5 l:0 -> w
  call: ABS_$QUOT_L w:-12 w:2 l:5 -> w
  call: ABS_$QUOT_L w:-12 w:2 l:6 -> w
  expect:
    => 1
    => 0
    => 1
    => 0
    => 1
    => 1
    => 1
    => 0
    => 0
    => 0
    => 1
    => 1
    => 7
    => 65536
    => 65534
    => 17
    => 32778
    => 29
    => 0
    => 6
    => 5
    => 5
    => 1
    => 0
    => 1
    => 0
    => 1
    => 1
    => 0
    => 1
    => 0
    9
    9
    0
    -32768
    => 40000
    => 0
    => 128
    => 5
    => 5
    => 7
    => 7
    => -32768
    => 9
    => -2147483648
    1234
    => -8
    => -8
    => -1
    => 1
    => 0
    => 1
    => 0
    => 1
    => 0
}
module abs;
%include 'exec.ins.pas';

type
  linteger = 0..2147483647;
  pinteger = 0..65535;
  sbyte = [byte] -128..127;

var
  g: integer;

function abs_$lt_signed(x, h: integer): integer; options(val_param);
begin
  if abs(x) < h then abs_$lt_signed := 1 else abs_$lt_signed := 0;
end;

function abs_$gt_const(x: integer): integer; options(val_param);
begin
  if abs(x) > 5 then abs_$gt_const := 1 else abs_$gt_const := 0;
end;

function abs_$sum(x, y: integer): integer32; options(val_param);
begin
  abs_$sum := abs(x) + abs(y);
end;

function abs_$plus10(x: integer): integer32; options(val_param);
begin
  abs_$plus10 := abs(x) + 10;
end;

function abs_$products(x, a, y, b: integer): integer; options(val_param);
begin
  abs_$products := abs(x * a) + abs(y * b);
end;

function abs_$quot(x, y: integer): integer; options(val_param);
begin
  if abs(x div y) > 5 then abs_$quot := 6 else abs_$quot := 5;
end;

function abs_$l_const(x: integer32): integer; options(val_param);
begin
  if abs(x) > 8000 then abs_$l_const := 1 else abs_$l_const := 0;
end;

function abs_$l_diff(a, b: linteger; c: linteger): integer; options(val_param);
begin
  if abs(a - b) > c then abs_$l_diff := 1 else abs_$l_diff := 0;
end;

procedure abs_$arg(x: integer); options(val_param);
begin
  exec_$logv_w(abs(x));
end;

function abs_$unsigned(u: pinteger): integer32; options(val_param);
begin
  abs_$unsigned := abs(u);
end;

function abs_$byte(b: sbyte): integer; options(val_param);
begin
  abs_$byte := abs(b);
end;

function abs_$store(x: integer): integer; options(val_param);
begin
  abs_$store := abs(x);
end;

function abs_$l_store(x: integer32): integer32; options(val_param);
begin
  abs_$l_store := abs(x);
end;

procedure abs_$global(x: integer); options(val_param);
begin
  g := abs(x);
  exec_$logv_w(g);
end;

function abs_$c_minus(x, y: integer): integer; options(val_param);
begin
  abs_$c_minus := -1 - abs(x - y);
end;

function abs_$w_diff(x, y: integer; l: integer32): integer; options(val_param);
begin
  if abs(x - y) > l then abs_$w_diff := 1 else abs_$w_diff := 0;
end;

function abs_$quot_l(x, y: integer; l: integer32): integer; options(val_param);
begin
  if abs(x div y) > l then abs_$quot_l := 1 else abs_$quot_l := 0;
end;

