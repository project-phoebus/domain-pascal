{ Execution test (bead domain-pascal-338r, domain-c's gaps 43, 44 and
  45): a varying string's initial value (the length word, the text,
  blanks); a short string and a character initialising arrays of char,
  blank-padded (as the DOMAIN C compiler's data has them, reported); and
  a `for' loop over an array whose index starts below zero, stepping a
  pointer formed in place (best effort, no site).

  call: INIT_PAD_NEGLOOP_$RUN
  expect:
    5
    97
    32
    32
    10
    32
    -1
    32
    34
}
module init_pad_negloop;
%include 'exec.ins.pas';

var
  name: varying [8] of char := 'ass_$';
  s: array [1..6] of char := 'ab';
  c: array [1..4] of char := chr(10);
  a: array [-1..32] of integer;

procedure init_pad_negloop_$run;
var
  k: integer;
  n: integer;
begin
  exec_$logv_w(name.length);
  exec_$logv_w(ord(name.body[1]));
  exec_$logv_w(ord(name.body[8]));
  exec_$logv_w(ord(s[6]));
  exec_$logv_w(ord(c[1]));
  exec_$logv_w(ord(c[4]));
  for k := -1 to 32 do
    a[k] := k;
  exec_$logv_w(a[-1]);
  exec_$logv_w(a[32]);
  n := 0;
  for k := -1 to 32 do
    if a[k] >= -1 then n := n + 1;
  exec_$logv_w(n);
end;
