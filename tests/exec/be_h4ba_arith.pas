{ Execution test (bead domain-pascal-h4ba): arithmetic and comparison
  forms of the link test's long tail (codegen-patterns 7.1, 8, 10.1).

  - Multiplication by a constant whose odd part has three bits: bit by
    bit (×26: NETLOG_$LOG_IT 0xE71BBA, the image's form; ×13, ×52: best
    effort); a longword by such a constant (M$ARITH, best effort); a
    longword index scaled by 84 (`muls.w` of a mask's word at
    HINT_$GET_HINTS 0xE4998E; here bit by bit at 32 bits, best effort).
  - `div`/`mod` by a computed word divisor or one reached through a
    pointer (best effort: the divisor after the dividend).
  - Unsigned `max` (`bhi`, DISK_$GROW_QBLK_POOL 0xE3BC52; best effort).
  - `!` and `xor` of a variable and a constant (the variable loaded,
    then `ori`/`eori`; best effort).
  - Comparisons with a constant outside the other operand's type: a
    `0..2147483647` against -1 and 16#80000000 compares the constant's 32
    bits, unsigned, as the image does (FILE_$SET_REFCNT 0xE5E410 `cmpi.l
    #-0x1`; AST_$UPDATE_ASTE's nested routine 0xE01518 `cmpi.l
    #-0x80000000; bcs`): the kernel passes 16#FFFFFFFF in such a
    parameter (file/set_refcnt.pas), so the expected values are the
    image's; a `0..65535` against -1 or 70000 and an `integer` against
    40000 have their mathematical result (ref p. 9-32, message 250).
  Edge values: zero, negative, the type's maximum.

  call: BH_A_$MUL26 w:5
  call: BH_A_$MUL26 w:-1260
  call: BH_A_$MUL26 w:1260
  call: BH_A_$MUL13 w:-7
  call: BH_A_$MUL13 w:2520
  call: BH_A_$MUL13 w:0
  call: BH_A_$MULL13 l:100000
  call: BH_A_$MULL13 l:-200000
  call: BH_A_$MULL13 l:0
  call: BH_A_$IDX84 l:74565 w:77
  call: BH_A_$IDX84 l:-1 w:-5
  call: BH_A_$IDX84 l:0 w:32767
  call: BH_A_$DIVC w:100 w:6
  call: BH_A_$DIVC w:-100 w:6
  call: BH_A_$DIVC w:32767 w:0
  call: BH_A_$DIVC w:-32768 w:1
  call: BH_A_$UDIVC w:65535 w:1
  call: BH_A_$UDIVC w:0 w:65534
  call: BH_A_$DIVP w:1000 w:7
  call: BH_A_$DIVP w:-1000 w:-7
  call: BH_A_$UMAX w:5
  call: BH_A_$UMAX w:33
  call: BH_A_$UMAX w:65535
  call: BH_A_$UMAX2 w:1 w:65535
  call: BH_A_$UMAX2 w:40000 w:2
  call: BH_A_$UMAX2 w:0 w:0
  call: BH_A_$ORX l:1 w:256
  call: BH_A_$ORX l:-1 w:-1
  call: BH_A_$ORX l:0 w:0
  call: BH_A_$LINT l:-1
  call: BH_A_$LINT l:5
  call: BH_A_$LINT l:-2147483648
  call: BH_A_$LINT l:2147483647
  call: BH_A_$LINT l:0
  call: BH_A_$LNEG l:0
  call: BH_A_$LNEG l:5
  call: BH_A_$LNEG l:2147483647
  call: BH_A_$LNEG l:-1
  call: BH_A_$UOUT w:65535 w:40000
  call: BH_A_$UOUT w:0 w:-32768
  call: BH_A_$UOUT w:32768 w:32767
  warns: best effort (not byte-identical): a multiplication by 13
  warns: best effort (not byte-identical): a 32-bit multiplication by a constant
  warns: best effort (not byte-identical): a divisor of `div<s>.w` that takes instructions
  warns: best effort (not byte-identical): an unsigned `max`
  warns: best effort (not byte-identical): `!`/`xor` of a variable and a constant
  warns: best effort (not byte-identical): comparing with a constant outside the other operand's type
  expect:
    130
    -32760
    32760
    -91
    -52
    32760
    32760
    0
    0
    1300000
    -2600000
    0
    77
    -5
    32767
    14
    2
    -14
    -2
    32767
    0
    -16384
    0
    32767
    1
    0
    0
    142
    6
    142
    -6
    33
    33
    65535
    65535
    1
    40000
    40000
    0
    0
    301989889
    322
    511
    301989889
    -1
    -1
    -256
    -301989889
    301989888
    66
    255
    301989888
    mark 1
    mark 1
    mark 0
    mark 0
    mark 0
    mark 1
    mark 0
    mark 1
    mark 0
    mark 0
    mark 0
    mark 1
    mark 0
    mark 0
    mark 1
    mark 1
    mark 1
    mark 0
    mark 1
    mark 1
    mark 1
    mark 0
    mark 1
    mark 1
    mark 1
    mark 0
    mark 1
    mark 0
    mark 1
    mark 0
    mark 0
    mark 0
    mark 1
    mark 0
    mark 1
    mark 0
    mark 1
    mark 0
    mark 1
    mark 0
    mark 1
    mark 0
    mark 1
}
module be_h4ba_arith;
%include 'exec.ins.pas';
type
  uw = 0..65535;
  lint = 0..2147483647;
  r84 = record a: integer32; pad: array[1..20] of integer32; end;
  wp = ^integer;
var
  arr: array[0..63] of r84;
  m: integer32;
  cell: integer;

{ ×26: `add.w; move.w; lsl.w #2; add.w; add.w; add.w' (0xE71BBA). }
procedure bh_a_$mul26(a: integer); options(val_param);
var w: integer;
begin
  w := a * 26;
  exec_$logv_w(w);
end;

{ ×13 and ×52 (odd part 13, three bits): bit by bit, best effort. }
procedure bh_a_$mul13(a: integer); options(val_param);
var w: integer;
begin
  w := a * 13;
  exec_$logv_w(w);
  w := 52 * (a div 4);
  exec_$logv_w(w);
end;

{ A longword times 13: M$ARITH. }
procedure bh_a_$mull13(a: integer32); options(val_param);
var l: integer32;
begin
  l := a * 13;
  exec_$logv_l(l);
end;

{ An element of 84 bytes at a longword index (a mask of a longword). }
procedure bh_a_$idx84(l: integer32; v: integer); options(val_param);
begin
  arr[l & 16#3F].pad[20] := v;
  exec_$clobber;
  exec_$logv_l(arr[l & 16#3F].pad[20]);
end;

{ A computed divisor (`b + 1'): the quotient and the remainder (which
  takes the dividend's sign, ref p. 4-115); `div' truncates toward zero
  (p. 4-65). }
procedure bh_a_$divc(a: integer; b: integer); options(val_param);
var q: integer;
begin
  q := a div (b + 1);
  exec_$logv_w(q);
  q := a mod (b + 1);
  exec_$logv_w(q);
end;

procedure bh_a_$udivc(a: uw; b: uw); options(val_param);
var q: uw;
begin
  q := a div (b + 1);
  exec_$logv_l(q);
  q := a mod (b + 1);
  exec_$logv_l(q);
end;

{ A divisor reached through a pointer (`movea.l' first). }
procedure bh_a_$divp(a: integer; b: integer); options(val_param);
var p: wp; q: integer;
begin
  cell := b;
  p := addr(cell);
  q := a div p^;
  exec_$logv_w(q);
  q := a mod p^;
  exec_$logv_w(q);
end;

{ An unsigned `max' with a constant first (`max(16#21, count)'). }
procedure bh_a_$umax(a: uw); options(val_param);
var u: uw;
begin
  u := max(16#21, a);
  exec_$logv_l(u);
end;

{ An unsigned `max' of two variables; `max(x, 0)'. }
procedure bh_a_$umax2(a: uw; b: uw); options(val_param);
var u: uw;
begin
  u := max(a, b);
  exec_$logv_l(u);
  u := max(a, 0);
  exec_$logv_l(u);
end;

{ `!' and `xor' of a variable (in memory, held in a register) and a
  constant. }
procedure bh_a_$orx(l: integer32; w: integer); options(val_param);
var r: integer32; k: integer;
begin
  m := l;
  r := m ! 16#12000000;
  exec_$logv_l(r);
  k := 16#42 ! w;
  exec_$logv_w(k);
  k := xor(w, 16#FF);
  exec_$logv_w(k);
  r := xor(m, 16#12000000);
  exec_$logv_l(r);
end;

{ A `0..2147483647' against constants it does not hold: the 32 bits
  compared, unsigned (the image's form). }
procedure bh_a_$lint(x: lint); options(val_param);
begin
  if x = -1 then exec_$mark(1) else exec_$mark(0);
  if x >= 16#80000000 then exec_$mark(1) else exec_$mark(0);
  if x < 16#80000000 then exec_$mark(1) else exec_$mark(0);
end;

{ Ordered against other negative constants: signed (ref 4.3.3). }
procedure bh_a_$lneg(x: lint); options(val_param);
begin
  if x > -1 then exec_$mark(1) else exec_$mark(0);
  if x >= -5 then exec_$mark(1) else exec_$mark(0);
  if x < -1 then exec_$mark(1) else exec_$mark(0);
  if -1 < x then exec_$mark(1) else exec_$mark(0);
end;

{ Words against constants outside their types: the mathematical
  results (p. 9-32). }
procedure bh_a_$uout(u: uw; w: integer); options(val_param);
begin
  if u = -1 then exec_$mark(1) else exec_$mark(0);
  if u < 70000 then exec_$mark(1) else exec_$mark(0);
  if w > 40000 then exec_$mark(1) else exec_$mark(0);
  if w > -40000 then exec_$mark(1) else exec_$mark(0);
end;
