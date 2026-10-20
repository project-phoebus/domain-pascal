{ Execution test (bead domain-pascal-61fd): register-sized set operations
  as values and the set comparisons beyond `=' / `<>' (ref "Set
  Operations", pp. 4-173..4-176; codegen-patterns 5.1, "Values", "Set
  comparisons").  Image forms: `s - [c] + [d]' as `moveq #~c; and; ori'
  (PROC1_$TRY_TO_SUSPEND 0xE1474C..0xE14756), `(s - a) + b' as `move.l;
  not.l; and.l; or.l' (PROC2_$SET_SIG_MASK 0xE3F850..0xE3F85E), an array
  element updated in place (DIR_$OLD_FIND_OVERFLOW_SLOT 0xE5500A `bclr
  #7,(868,A4)'), `k in h' with h held in a register as a value
  (PROC1_$SUSPEND 0xE14842 `btst #1,D1; sne D0').  Best effort (no site):
  `x + y' and `x + c' of variables, `c - x', `x * y' of computed
  operands, `s := s * y' (not in place), subset `<=' and superset `>='
  (as `s - t = []'), a constant left operand of `=' / `<>', and one-byte
  sets compared with a set other than `[]' (widened to words).  The
  expected log is the bit model of ref 3.9.3 (member k is bit k),
  computed apart from the compiler.

  call: SET_VALUES_$RUN w:5 w:7 l:2 l:6
  call: SET_VALUES_$RUN w:4 w:4 l:2147483649 l:2147483649
  call: SET_VALUES_$RUN w:-32760 w:32514 l:-1 l:15728655
  call: SET_VALUES_$RUN w:258 w:770 l:0 l:305419896
  call: SET_VALUES_$RUN w:18440 w:1 l:6 l:-2
  expect:
    3
    5
    7
    0x0000000a
    0x00000000
    4
    32
    0
    0x00000002
    7
    0
    7
    21
    false
    false
    true
    false
    false
    true
    true
    mark 1
    mark 2
    true
    false
    true
    true
    false
    true
    false
    true
    false
    2
    6
    4
    0x80000009
    0x80000001
    4
    33
    0
    0x80000001
    4
    0
    4
    20
    false
    false
    true
    true
    true
    false
    true
    mark 1
    mark 2
    true
    true
    false
    false
    true
    true
    true
    true
    false
    -32758
    -32760
    -246
    0xffffffff
    0xff0ffff1
    -32760
    33
    -32760
    0x00f0000f
    -246
    -32760
    32522
    -32744
    true
    false
    false
    false
    false
    true
    true
    false
    true
    true
    false
    true
    true
    false
    false
    false
    258
    258
    770
    0x00000008
    0x00000000
    256
    33
    0
    0x00000000
    770
    0
    770
    274
    false
    false
    true
    false
    false
    true
    true
    mark 1
    mark 2
    true
    false
    false
    true
    false
    true
    true
    true
    true
    18442
    18442
    18441
    0x0000000e
    0x80000000
    18440
    33
    18440
    0x00000006
    18441
    18440
    18441
    18456
    true
    true
    false
    false
    false
    true
    true
    true
    false
    true
    true
    false
    true
    false
    false
    false
}
module set_values;
%include 'exec.ins.pas';

type
  w_t = set of 0..15;
  l_t = set of 0..31;
  b_t = [byte] set of 0..7;
  rec = record
    pad: integer;
    flags: w_t;
    lf: l_t;
    b1: b_t;
    b2: b_t;
  end;
var
  r: rec;
  a, gw, c: w_t;
  x, gl, y: l_t;
  arr: array[1..4] of w_t;
  b: boolean;

procedure logs(s: w_t); options(val_param); internal;
begin
  exec_$logv_w(integer(s));
end;

procedure unions;
begin
  r.flags := a;
  r.flags := r.flags - [2] + [1];
  exec_$log_w(integer(r.flags));
  r.flags := (r.flags - c) + a;
  exec_$log_w(integer(r.flags));
  gw := a + c;
  exec_$log_w(integer(gw));
  gl := x + [3];
  exec_$log_x(integer32(gl));
  gl := (x - y) + (y * [0, 31]);
  exec_$log_x(integer32(gl));
end;

procedure diffs;
begin
  gw := a - [0, 1];
  exec_$log_w(integer(gw));
  gw := [0, 5] - a;
  exec_$log_w(integer(gw));
  gw := (a + c) * (a - c);
  exec_$log_w(integer(gw));
  r.lf := x;
  r.lf := r.lf * y;
  exec_$log_x(integer32(r.lf));
  logs(a + c);
  logs(a - c);
end;

procedure elems;
var
  i, j: integer;
begin
  for i := 1 to 4 do
    arr[i] := a;
  j := exec_$id_w(2);
  arr[j] := arr[j] - [15];
  arr[j + 1] := arr[j + 1] + [4];
  arr[j] := arr[j] + c;
  exec_$log_w(integer(arr[2]));
  exec_$log_w(integer(arr[3]));
end;

procedure held(in s: w_t);
var
  h: w_t;
begin
  h := s;
  exec_$clobber;
  b := 3 in h;
  exec_$log_bool(b);
  b := 14 in h;
  exec_$log_bool(b);
end;

procedure cmp_w;
begin
  exec_$log_bool(a <= c);
  exec_$log_bool(a >= c);
  exec_$log_bool([2] = a);
  exec_$log_bool([2] <> a);
  exec_$log_bool(a * c <= a);
  if a <= c then
    exec_$mark(1);
  if (a * c) <> [] then
    exec_$mark(2);
end;

procedure cmp_l;
begin
  exec_$log_bool(x <= y);
  exec_$log_bool(x >= y);
  exec_$log_bool([1] <= x);
  exec_$log_bool(x <= [1, 2]);
  exec_$log_bool(x >= [0, 31]);
  exec_$log_bool((x + y) = (y + x));
end;

procedure cmp_b;
begin
  r.b1 := a;
  r.b2 := c;
  exec_$log_bool(r.b1 = r.b2);
  exec_$log_bool(r.b1 <= r.b2);
  exec_$log_bool(r.b1 = [1]);
end;

procedure set_values_$run(ia: integer; ic: integer; ix: integer32; iy: integer32);
  options(val_param);
begin
  a := w_t(ia);
  c := w_t(ic);
  x := l_t(ix);
  y := l_t(iy);
  unions;
  diffs;
  elems;
  held(a);
  cmp_w;
  cmp_l;
  cmp_b;
end;
