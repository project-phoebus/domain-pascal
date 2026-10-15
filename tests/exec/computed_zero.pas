{ Execution test (bead domain-pascal-qlzu): a computed value compared
  with 0 (codegen-patterns 7.1, "A computed value against 0").  `<' and
  `>=' branch on (or `s<cc>' reads) the flags of the add or sub that
  formed the value, `bmi'/`bpl' (tty_$i_put_chars 0xE1B046 `subq.w #1,D1;
  bpl.s'); `>' and `<=' test it first, `tst; bgt'/`ble' (SMD_$SIGNAL
  0xE6F25C).  N and Z of an add or sub are those of the 32-bit result,
  so the relations are those of the wrapped sum (Pascal does not detect
  overflow): 16#7FFFFFFF + 1 < 0, 16#80000000 - 1 > 0.  `bgt'/`ble' on
  the sub's own flags would test the exact difference (N xor V) and get
  these wrong.  Also `=', `<>' on the flags of `or'/`and', `btst' for one
  bit (the mask on either side; bit 31 alone is refused, bead
  domain-pascal-3xwx), `tst' of a function's result
  (with junk above its word), of a quotient and of a byte widened to a
  word, and a register-held local an assignment just formed.  Word sums stay clear of 16-bit overflow: their
  width in a comparison is bead domain-pascal-9js (wide_compare.pas).

  call: COMPUTED_ZERO_$ADD_LT l:0x7fffffff l:1 -> b
  call: COMPUTED_ZERO_$ADD_LT l:1 l:-2 -> b
  call: COMPUTED_ZERO_$ADD_LT l:5 l:-5 -> b
  call: COMPUTED_ZERO_$ADD_LT l:0x80000000 l:-1 -> b
  call: COMPUTED_ZERO_$SUB_GE l:0x80000000 l:1 -> b
  call: COMPUTED_ZERO_$SUB_GE l:0x7fffffff l:-1 -> b
  call: COMPUTED_ZERO_$SUB_GE l:3 l:3 -> b
  call: COMPUTED_ZERO_$SUB_GT l:0x80000000 l:1
  call: COMPUTED_ZERO_$SUB_GT l:0x7fffffff l:-1
  call: COMPUTED_ZERO_$SUB_GT l:3 l:3
  call: COMPUTED_ZERO_$SUB_GT l:3 l:2
  call: COMPUTED_ZERO_$ADD_LE l:0x7fffffff l:1
  call: COMPUTED_ZERO_$ADD_LE l:0x80000000 l:-1
  call: COMPUTED_ZERO_$ADD_LE l:5 l:-5
  call: COMPUTED_ZERO_$ADD_LE l:5 l:-4
  call: COMPUTED_ZERO_$LOGIC w:0 w:0 l:0x10000
  call: COMPUTED_ZERO_$LOGIC w:256 w:0 l:0x100
  call: COMPUTED_ZERO_$LOGIC w:0 w:-32768 l:0x200
  call: COMPUTED_ZERO_$BIT l:0x10000
  call: COMPUTED_ZERO_$BIT l:0xfffeffff
  call: COMPUTED_ZERO_$BIT l:0x3fffffff
  call: COMPUTED_ZERO_$TESTED w:0 w:6 b:0
  call: COMPUTED_ZERO_$TESTED w:-1 w:13 b:128
  call: COMPUTED_ZERO_$ASSIGNED w:5 w:6
  call: COMPUTED_ZERO_$ASSIGNED w:5 w:5
  call: COMPUTED_ZERO_$ASSIGNED w:5 w:4
  expect:
    => true
    => true
    => false
    => false
    => true
    => false
    => true
    mark 1
    mark 0
    mark 0
    mark 1
    mark 1
    mark 0
    mark 1
    mark 0
    mark 11
    mark 19
    mark 21
    mark 19
    mark 19
    mark 31
    mark 33
    mark 39
    mark 39
    mark 31
    mark 33
    mark 39
    mark 41
    mark 51
    mark 61
    mark 71
    mark 49
    mark 61
    mark 49
    mark 91
    mark 89
    mark 81
    mark 91
    mark 89
    mark 89
}
module computed_zero;
%include 'exec.ins.pas';

type
  byte_t = [byte] 0..255;

var
  l: integer32;
  bt: byte_t;

function computed_zero_$add_lt(a, b: integer32): boolean; options(val_param);
begin
  computed_zero_$add_lt := (a + b) < 0;
end;

function computed_zero_$sub_ge(a, b: integer32): boolean; options(val_param);
begin
  computed_zero_$sub_ge := (a - b) >= 0;
end;

procedure computed_zero_$sub_gt(a, b: integer32); options(val_param);
begin
  if (a - b) > 0 then exec_$mark(1) else exec_$mark(0);
end;

procedure computed_zero_$add_le(a, b: integer32); options(val_param);
begin
  if (a + b) <= 0 then exec_$mark(1) else exec_$mark(0);
end;

procedure computed_zero_$logic(a, b: integer; m: integer32); options(val_param);
begin
  if (a ! b) = 0 then exec_$mark(11);
  if (m & 16#180) <> 0 then exec_$mark(21);
  l := m;
  exec_$mark(19);
end;

procedure computed_zero_$bit(m: integer32); options(val_param);
begin
  if (m & 16#10000) <> 0 then exec_$mark(31);
  if (16#40000000 & m) = 0 then exec_$mark(33);
  l := m;
  exec_$mark(39);
end;

procedure computed_zero_$tested(a, b: integer; c: byte_t); options(val_param);
begin
  bt := c;
  if exec_$id_w(a) = 0 then exec_$mark(41);
  if (b div 7) = 0 then exec_$mark(51);
  if (b mod 7) <> 0 then exec_$mark(61);
  if bt = 0 then exec_$mark(71);
  exec_$mark(49);
end;

procedure computed_zero_$assigned(a, b: integer); options(val_param);
var x: integer;
begin
  x := exec_$id_w(a);
  x := x - b;
  if x = 0 then exec_$mark(81);
  x := x - 1;
  if x < 0 then exec_$mark(91);
  exec_$mark(89);
end;
