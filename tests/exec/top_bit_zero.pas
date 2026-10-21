{ Execution test (bead domain-pascal-wkm5): a word type that uses the top
  bit (pinteger, 0..65535) compared with 0 (codegen-patterns 7.1,
  "Signed or unsigned").  The original compiler branches on the `tst.w'
  or load's flags with the signed condition, so 16#8000..16#FFFF count
  as below 0: `x <= 0' `ble' (MT_$ACQUIRE SAU5 0xE760B6, MT_PURGE_UCB's
  module-local neighbor SAU5 0xE75916, SOCK_$PUT_INT SAU2 0xE161AA),
  `x > 0' `ble' around the part (PKT_$BLD_INTERNET_HDR 0xE122D6), `x <
  0' `bmi' (PBU_$GET_DCTE SAU5 0xE59212, WIN_$DINIT SAU5 0xE19EDE); the
  operands are unsigned by their `cmpi.w #c; bhi/bls' bounds and their
  zero-extensions.

  The expected values are the image's, not ref 4.3.3's (p. 4-6: an
  unsigned run-time value against a constant in 0..2147483647 is
  compared unsigned, which would make `x <= 0' mean `x = 0'): they
  differ only for 16#8000..16#FFFF, marked "(image)" below.  The
  MT-shaped `(x <= 0) or else (x > 16#4000)' gives the same result
  either way, since the second arm takes those values.

  call: TOP_BIT_ZERO_$REL w:0
  call: TOP_BIT_ZERO_$REL w:1
  call: TOP_BIT_ZERO_$REL w:0x7fff
  call: TOP_BIT_ZERO_$REL w:0x8000
  call: TOP_BIT_ZERO_$REL w:0xffff
  call: TOP_BIT_ZERO_$LE w:0 -> b
  call: TOP_BIT_ZERO_$LE w:0x4000 -> b
  call: TOP_BIT_ZERO_$LE w:0x8000 -> b
  call: TOP_BIT_ZERO_$LE w:0xfffe -> b
  call: TOP_BIT_ZERO_$GT w:0 -> b
  call: TOP_BIT_ZERO_$GT w:2 -> b
  call: TOP_BIT_ZERO_$GT w:0x8001 -> b
  call: TOP_BIT_ZERO_$CHECK @w:0 @l:7!
  call: TOP_BIT_ZERO_$CHECK @w:1 @l:7!
  call: TOP_BIT_ZERO_$CHECK @w:0x4000 @l:7!
  call: TOP_BIT_ZERO_$CHECK @w:0x4001 @l:7!
  call: TOP_BIT_ZERO_$CHECK @w:0x8000 @l:7!
  call: TOP_BIT_ZERO_$CHECK @w:0xffff @l:7!
  call: TOP_BIT_ZERO_$FIELD w:0
  call: TOP_BIT_ZERO_$FIELD w:5
  call: TOP_BIT_ZERO_$FIELD w:0xc000
  expect:
    mark 1
    mark 4
    mark 9
    mark 2
    mark 4
    mark 9
    mark 2
    mark 4
    mark 9
    mark 1
    mark 3
    mark 9
    mark 1
    mark 3
    mark 9
    => true
    => false
    => true
    => true
    => false
    => true
    => false
    out 2228227
    out 0
    out 0
    out 2228227
    out 2228227
    out 2228227
    mark 21
    mark 29
    mark 22
    mark 29
    mark 21
    mark 29
}
module top_bit_zero;
%include 'exec.ins.pas';

type
  pinteger = 0..65535;
  rec_t = record
    pad: integer;
    n: pinteger;
  end;

var
  rec: rec_t;

{ Each relation alone: 16#8000 and 16#FFFF are `<= 0' and `< 0'
  (image). }
procedure top_bit_zero_$rel(u: pinteger); options(val_param);
begin
  if u <= 0 then exec_$mark(1);
  if u > 0 then exec_$mark(2);
  if u < 0 then exec_$mark(3);
  if u >= 0 then exec_$mark(4);
  exec_$mark(9);
end;

{ As a boolean value: 16#8000 and 16#FFFE true (image). }
function top_bit_zero_$le(u: pinteger): boolean; options(val_param);
begin
  top_bit_zero_$le := u <= 0;
end;

{ 16#8001 false (image). }
function top_bit_zero_$gt(u: pinteger): boolean; options(val_param);
begin
  top_bit_zero_$gt := u > 0;
end;

{ MT_$ACQUIRE's length check (SAU5 0xE760B6): by address, the default
  convention. }
procedure top_bit_zero_$check(in length: pinteger; out status: integer32);
begin
  if (length <= 0) or else (length > 16#4000) then
    status := 16#220003
  else
    status := 0;
end;

{ A record field in memory: 16#C000 is `<= 0' (image). }
procedure top_bit_zero_$field(u: pinteger); options(val_param);
begin
  rec.n := u;
  if rec.n > 0 then exec_$mark(22) else exec_$mark(21);
  exec_$mark(29);
end;
