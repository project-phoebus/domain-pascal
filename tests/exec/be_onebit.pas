{ Execution test (bead domain-pascal-5kgg, best effort): one-bit tests
  of `&' whose image form is not settled (codegen-patterns 7.1, "One
  bit"): bit 15 and bit 7 (`btst #15'/`#7': no such `btst' in compiled
  code), a function's result (`btst' on D0 after the call), a byte
  widened for the `&', a word variable in memory as a boolean value.

  warns: best effort (not byte-identical): testing bit 7, 15 or 31 alone
  warns: best effort (not byte-identical): testing one bit of a function's result
  warns: best effort (not byte-identical): testing one bit of a byte widened
  warns: best effort (not byte-identical): one bit of a variable in memory as a boolean value
  call: BE_ONEBIT_$BIT15 w:-32768
  call: BE_ONEBIT_$BIT15 w:16384
  call: BE_ONEBIT_$BIT7 w:251
  call: BE_ONEBIT_$BIT7 w:4
  call: BE_ONEBIT_$CALL w:4
  call: BE_ONEBIT_$CALL w:251
  call: BE_ONEBIT_$BYTE b:4
  call: BE_ONEBIT_$BYTE b:251
  call: BE_ONEBIT_$VALUE w:4
  call: BE_ONEBIT_$VALUE w:251
  expect:
    1
    0
    1
    0
    1
    0
    1
    0
    true
    false
}
module be_onebit;
%include 'exec.ins.pas';

type
  ub = [byte] 0..255;
var
  u: exec_$uword;
  c: ub;
  g: integer;
  bo: boolean;
  w: integer;

procedure be_onebit_$bit15(x: exec_$uword); options(val_param);
begin
  u := x;
  if (u & 16#8000) <> 0 then w := 1 else w := 0;
  exec_$log_w(w);
end;

procedure be_onebit_$bit7(x: integer); options(val_param);
begin
  g := x;
  if (g & 128) <> 0 then w := 1 else w := 0;
  exec_$log_w(w);
end;

procedure be_onebit_$call(x: integer); options(val_param);
begin
  g := x;
  if (exec_$id_w(g) & 4) <> 0 then w := 1 else w := 0;
  exec_$log_w(w);
end;

procedure be_onebit_$byte(b: ub); options(val_param);
begin
  c := b;
  if (c & 4) <> 0 then w := 1 else w := 0;
  exec_$log_w(w);
end;

procedure be_onebit_$value(x: integer); options(val_param);
begin
  g := x;
  bo := (g & 4) <> 0;
  exec_$log_bool(bo);
end;
