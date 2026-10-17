{ Execution test: compares of a variable in memory with a register-held
  parameter, both orders, and of two held values (codegen-patterns 7.1, "A
  variable in memory against a register-held one, and two held values";
  bead domain-pascal-5xnz).  Each test sets a bit of the logged mask:
  1 g = a, 2 g <> a, 4 g < a, 8 g >= a, 16 a > b, 32 b <= a, 64 a = b,
  128 a < g, 256 b <> a.  The unsigned routine does the same on 0..65535
  values (40000 is above every signed word).

  call: HELD_CMP_$W w:5 w:3 w:5
  call: HELD_CMP_$W w:-2 w:7 w:4
  call: HELD_CMP_$W w:7 w:7 w:-9
  call: HELD_CMP_$U w:40000 w:3 w:65535
  call: HELD_CMP_$U w:3 w:40000 w:3
  expect:
    313
    394
    102
    442
    265
}
module held_cmp;
%include 'exec.ins.pas';

var
  g: integer;
  gu: exec_$uword;
  r: integer;

procedure held_cmp_$w(a, b: integer; c: integer); options(val_param);
begin
  g := c;
  r := 0;
  if g = a then r := r + 1;
  if g <> a then r := r + 2;
  if g < a then r := r + 4;
  if g >= a then r := r + 8;
  if a > b then r := r + 16;
  if b <= a then r := r + 32;
  if a = b then r := r + 64;
  if a < g then r := r + 128;
  if b <> a then r := r + 256;
  exec_$log_w(r);
end;

procedure held_cmp_$u(a, b: exec_$uword; c: exec_$uword); options(val_param);
begin
  gu := c;
  r := 0;
  if gu = a then r := r + 1;
  if gu <> a then r := r + 2;
  if gu < a then r := r + 4;
  if gu >= a then r := r + 8;
  if a > b then r := r + 16;
  if b <= a then r := r + 32;
  if a = b then r := r + 64;
  if a < gu then r := r + 128;
  if b <> a then r := r + 256;
  exec_$log_w(r);
end;
