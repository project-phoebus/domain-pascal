{ Execution test (bead domain-pascal-bzkf): `min' and `max' of run-time
  integers ("Min", p. 4-113; "Max", p. 4-111) and `in_range' of a word
  subrange ("In_range", p. 4-104); codegen-patterns 10.1.  The
  compare-and-select keeps the first operand when it is not beyond the
  second (`ble'/`bls' for min, `bge' for max); the comparison is unsigned
  only when both operands are unsigned (or one is and the other a
  non-negative constant), so 40000 in a 0..65535 word is above 1, and
  against a signed operand or a negative constant the unsigned word is
  widened with zeros and compared in 32 bits.  A word difference is
  computed in 32 bits (NETBUF_$ADD_PAGES 0xE0E95A): 176 - (-32768) is
  32944, not the wrapped -32592.  `max(x - y, 0)' reads the subtraction's
  N flag (0xE0EB64 `bpl; clr'), so a wrapped difference counts as what it
  wrapped to.  Equal operands, the bounds of the types and of the
  subranges.

  call: MIN_MAX_$MIN_SW w:3 w:5 -> w
  call: MIN_MAX_$MIN_SW w:5 w:3 -> w
  call: MIN_MAX_$MIN_SW w:-1 w:1 -> w
  call: MIN_MAX_$MIN_SW w:4 w:4 -> w
  call: MIN_MAX_$MIN_SW w:32767 w:-32768 -> w
  call: MIN_MAX_$MAX_SW w:-1 w:1 -> w
  call: MIN_MAX_$MAX_SW w:1 w:-1 -> w
  call: MIN_MAX_$MAX_SW w:-32768 w:-32768 -> w
  call: MIN_MAX_$MIN_UW w:40000 w:1 -> uw
  call: MIN_MAX_$MIN_UW w:1 w:40000 -> uw
  call: MIN_MAX_$MIN_UW w:65535 w:65535 -> uw
  call: MIN_MAX_$MIN_UL w:65535 l:0x7fffffff -> l
  call: MIN_MAX_$MIN_UL w:5 l:3 -> l
  call: MIN_MAX_$MIN_UL w:0 l:0 -> l
  call: MIN_MAX_$MIN_DIFF w:100 w:-32768 -> w
  call: MIN_MAX_$MIN_DIFF w:100 w:100 -> w
  call: MIN_MAX_$MIN_DIFF w:76 w:100 -> w
  call: MIN_MAX_$MAX0 l:3 l:5 -> l
  call: MIN_MAX_$MAX0 l:5 l:3 -> l
  call: MIN_MAX_$MAX0 l:5 l:5 -> l
  call: MIN_MAX_$MAX0 l:0x80000000 l:1 -> l
  call: MIN_MAX_$MIN_CONST w:2000 -> w
  call: MIN_MAX_$MIN_CONST w:1024 -> w
  call: MIN_MAX_$MIN_CONST w:-5 -> w
  call: MIN_MAX_$MIN_TWO w:3 -> w
  call: MIN_MAX_$MIN_TWO w:-3 -> w
  call: MIN_MAX_$MAX_MIXED w:-1 w:40000 -> l
  call: MIN_MAX_$MAX_MIXED w:-1 w:0 -> l
  call: MIN_MAX_$MAX_NEG w:40000 -> l
  call: MIN_MAX_$MAX_NEG w:0 -> l
  call: MIN_MAX_$IN_PLACE w:7 w:9
  call: MIN_MAX_$IN_PLACE w:9 w:7
  call: MIN_MAX_$RANGE0 w:0
  call: MIN_MAX_$RANGE0 w:3
  call: MIN_MAX_$RANGE0 w:4
  call: MIN_MAX_$RANGE0 w:-1
  call: MIN_MAX_$RANGE1 w:0
  call: MIN_MAX_$RANGE1 w:1
  call: MIN_MAX_$RANGE1 w:3
  call: MIN_MAX_$RANGE1 w:4
  call: MIN_MAX_$RANGE1 w:-1
  expect:
    => 3
    => 3
    => -1
    => 4
    => -32768
    => 1
    => 1
    => -32768
    => 1
    => 1
    => 65535
    => 65535
    => 3
    => 0
    => 100
    => 76
    => 76
    => 0
    => 2
    => 0
    => 2147483647
    => 1024
    => 1024
    => -5
    => 2
    => -3
    => 40000
    => 0
    => 40000
    => 0
    9
    9
    mark 1
    mark 1
    mark 0
    mark 0
    mark 0
    mark 1
    mark 1
    mark 0
    mark 0
}
module min_max;
%include 'exec.ins.pas';

type
  pinteger = 0..65535;
  linteger = 0..2147483647;
  two = 0..3;
  three = 1..3;

var
  t: two;
  s: three;
  g: integer;

function min_max_$min_sw(a, b: integer): integer; options(val_param);
begin
  min_max_$min_sw := min(a, b);
end;

function min_max_$max_sw(a, b: integer): integer; options(val_param);
begin
  min_max_$max_sw := max(a, b);
end;

function min_max_$min_uw(a, b: pinteger): pinteger; options(val_param);
begin
  min_max_$min_uw := min(a, b);
end;

function min_max_$min_ul(a: pinteger; b: linteger): linteger; options(val_param);
begin
  min_max_$min_ul := min(a, b);
end;

function min_max_$min_diff(a, b: integer): integer; options(val_param);
begin
  min_max_$min_diff := min(a, 176 - b);
end;

function min_max_$max0(a, b: integer32): integer32; options(val_param);
begin
  min_max_$max0 := max(a - b, 0);
end;

function min_max_$min_const(a: integer): integer; options(val_param);
begin
  min_max_$min_const := min(a, 1024);
end;

function min_max_$min_two(a: integer): integer; options(val_param);
begin
  min_max_$min_two := min(a, 2);
end;

function min_max_$max_mixed(a: integer; b: pinteger): integer32; options(val_param);
begin
  min_max_$max_mixed := max(a, b);
end;

function min_max_$max_neg(b: pinteger): integer32; options(val_param);
begin
  min_max_$max_neg := max(-1, b);
end;

procedure min_max_$in_place(a, b: integer); options(val_param);
var
  x: integer;
begin
  x := a;
  x := max(x, b);
  g := x;
  exec_$logv_w(g);
end;

procedure min_max_$range0(v: integer); options(val_param);
begin
  t := two(v);
  if in_range(t) then exec_$mark(1) else exec_$mark(0);
end;

procedure min_max_$range1(v: integer); options(val_param);
begin
  s := three(v);
  if in_range(s) then exec_$mark(1) else exec_$mark(0);
end;
