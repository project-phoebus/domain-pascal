{ Execution test: `for' loops over byte and longword indices (ref "For",
  pp. 4-88..4-90: the bounds evaluated once, no pass for an empty range;
  codegen-patterns 7.4, "Byte and longword indices"; bead
  domain-pascal-1mtr): integer32, linteger (0..2147483647, unsigned),
  `[byte]' subranges (0..255, -128..127) and char; `to' and
  `downto'; constant and run-time bounds; more than 65536 passes (the
  longword count); loops to the type's last value, which must end; a
  run-time bound wider than the index and below its type's range, which
  must run no pass (`0 to n - 1' with n = 0: reference-clarifications
  question 47); an unsigned word index to a run-time bound; the
  index passed by address and read after an `exit'.

  call: FORWD_$LCONST
  call: FORWD_$LBIG
  call: FORWD_$LRUN l:1 l:3
  call: FORWD_$LRUN l:5 l:4
  call: FORWD_$LRUN l:-3 l:2
  call: FORWD_$LRUN l:2147483645 l:2147483647
  call: FORWD_$LRUN l:-2147483648 l:-2147483646
  call: FORWD_$LRUN l:1 l:70000
  call: FORWD_$LUPTO l:-1
  call: FORWD_$LUPTO l:0
  call: FORWD_$LUPTO l:21
  call: FORWD_$UUPTO l:0
  call: FORWD_$UUPTO l:3
  call: FORWD_$WIRE l:0 l:0
  call: FORWD_$WIRE l:1024 l:4096
  call: FORWD_$ONCE
  call: FORWD_$BYTES
  call: FORWD_$CRUN b:97 b:99
  call: FORWD_$CRUN b:253 b:255
  call: FORWD_$CRUN b:66 b:65
  call: FORWD_$URUN b:0 b:255
  call: FORWD_$URUN b:10 b:9
  call: FORWD_$URUN b:255 b:255
  call: FORWD_$SRUN b:-128 b:127
  call: FORWD_$SRUN b:5 b:-5
  call: FORWD_$SRUN b:-3 b:-1
  call: FORWD_$AFTER
  call: FORWD_$NEXT
  call: FORWD_$BELOW w:0 l:0
  call: FORWD_$BELOW w:3 l:2
  call: FORWD_$UWORD w:0 l:0
  call: FORWD_$UWORD w:3 l:2
  call: FORWD_$UWORD w:65535 l:65535
  expect:
    10
    3
    2
    1
    0
    -1
    -2
    70000
    3
    1
    3
    3
    0
    0
    0
    0
    6
    -3
    2
    6
    3
    2147483645
    2147483647
    3
    3
    -2147483648
    -2147483646
    3
    70000
    1
    70000
    70000
    0
    0
    0
    0
    0
    0
    1
    0
    0
    1
    21
    22
    41
    41
    22
    0
    1
    0
    3
    4
    6
    1
    1
    2
    3
    4
    3
    256
    mark 32640
    256
    256
    mark -128
    32640
    97
    98
    99
    mark 99
    99
    98
    97
    mark 98
    253
    254
    255
    mark 99
    255
    254
    253
    mark 98
    mark 99
    mark 98
    256
    256
    0
    0
    1
    1
    256
    mark -128
    0
    mark 0
    3
    mark -6
    4
    2
    3
    113
    5
    mark 7
    0
    0
    0
    3
    2
    1
    0
    0
    0
    3
    2
    1
    65535
    65535
    65534
}
module forwd;
%include 'exec.ins.pas';

type
  byte_t = [byte] 0..255;
  sbyte_t = [byte] -128..127;
  lint = 0..2147483647;

var
  cnt: integer32;
  sum: integer32;
  first: integer32;
  last: integer32;
  n: integer32;
  wsum: integer;

{ constant bounds, the index read in the body }
procedure forwd_$lconst;
var
  i: integer32;
begin
  sum := 0;
  for i := 0 to 4 do
    sum := sum + i;
  exec_$logv_l(sum);
  for i := 3 downto -2 do
    exec_$logv_l(i);
end;

{ more than 65536 passes: a longword count }
procedure forwd_$lbig;
var
  i: integer32;
begin
  cnt := 0;
  for i := 1 to 70000 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ run-time bounds both ways, up to the type's limits }
procedure forwd_$lrun(a, b: integer32); options(val_param);
var
  i: integer32;
begin
  cnt := 0;
  first := 0;
  last := 0;
  for i := a to b do begin
    if cnt = 0 then
      first := i;
    cnt := cnt + 1;
    last := i;
  end;
  exec_$logv_l(cnt);
  exec_$logv_l(first);
  exec_$logv_l(last);
  cnt := 0;
  for i := b downto a do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ a constant bound and a run-time one }
procedure forwd_$lupto(k: integer32); options(val_param);
var
  i: integer32;
begin
  cnt := 0;
  for i := 1 to k do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := 0 to k do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := 20 to k do
    cnt := cnt + i;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := k downto 20 do
    cnt := cnt + i;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := k downto 0 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ an unsigned longword index (Apollo's linteger) }
procedure forwd_$uupto(k: lint); options(val_param);
var
  i: lint;
begin
  cnt := 0;
  for i := 1 to k do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := 0 to k do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := k downto 1 do
    cnt := cnt + i;
  exec_$logv_l(cnt);
end;

{ MST_$WIRE_AREA's shape: a computed count, a call in the body }
procedure forwd_$wire(va, end_va: lint); options(val_param);
var
  k, m: lint;
begin
  m := rshft(end_va, 10) - rshft(va, 10) + 1;
  for k := 1 to m do
    exec_$logv_l(k);
end;

{ the bound evaluated once }
procedure forwd_$once;
var
  i: integer32;
begin
  n := 3;
  cnt := 0;
  for i := 1 to n do begin
    n := n + 1;
    cnt := cnt + 1;
  end;
  exec_$logv_l(cnt);
end;

{ byte indices to their type's last value }
procedure forwd_$bytes;
var
  b: byte_t;
  s: sbyte_t;
  c: char;
begin
  cnt := 0;
  wsum := 0;
  for b := 0 to 255 do begin
    cnt := cnt + 1;
    wsum := wsum + b;
  end;
  exec_$logv_l(cnt);
  exec_$mark(wsum);
  cnt := 0;
  for b := 255 downto 0 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  wsum := 0;
  for s := -128 to 127 do begin
    cnt := cnt + 1;
    wsum := wsum + s;
  end;
  exec_$logv_l(cnt);
  exec_$mark(wsum);
  cnt := 0;
  for c := chr(0) to chr(255) do
    cnt := cnt + ord(c);
  exec_$logv_l(cnt);
end;

{ a char index, run-time bounds, a call in the body }
procedure forwd_$crun(lo, hi: char); options(val_param);
var
  c: char;
begin
  for c := lo to hi do
    exec_$logv_c(c);
  exec_$mark(99);
  for c := hi downto lo do
    exec_$logv_c(c);
  exec_$mark(98);
end;

{ an unsigned byte index, run-time bounds }
procedure forwd_$urun(lo, hi: byte_t); options(val_param);
var
  b: byte_t;
begin
  cnt := 0;
  for b := lo to hi do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for b := hi downto lo do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ a signed byte index, run-time bounds }
procedure forwd_$srun(lo, hi: sbyte_t); options(val_param);
var
  s: sbyte_t;
begin
  cnt := 0;
  wsum := 0;
  for s := lo to hi do begin
    cnt := cnt + 1;
    wsum := wsum + s;
  end;
  exec_$logv_l(cnt);
  exec_$mark(wsum);
end;

{ the index passed by address in the body, and read after `exit' }
procedure forwd_$after;
var
  i: integer32;
  c: char;
begin
  for i := 1 to 10 do
    if i = 4 then
      exit;
  last := i;
  exec_$logv_l(last);
  for i := 2 to 3 do
    exec_$log_l(i);
  for c := 'a' to 'z' do
    if c = 'q' then
      exit;
  last := ord(c);
  exec_$logv_l(last);
end;

{ `next' in the body still steps the index }
procedure forwd_$next;
var
  i: integer32;
  c: char;
begin
  cnt := 0;
  for i := 1 to 4 do begin
    if i = 3 then
      next;
    cnt := cnt + i;
  end;
  exec_$logv_l(cnt - 2);
  wsum := 0;
  for c := 'a' to 'e' do begin
    if c = 'c' then
      next;
    wsum := wsum + 1;
  end;
  exec_$mark(wsum + 3);
end;

type
  uword_t = 0..65535;

{ a wider run-time bound below the index type's range: no pass }
procedure forwd_$below(n: integer; k: integer32); options(val_param);
var
  b: byte_t;
  i: lint;
  u: byte_t;
begin
  cnt := 0;
  for b := 0 to n - 1 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := 0 to k - 1 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for u := n downto k + 1 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ an unsigned word index (in a register) to a run-time bound, and to a
  wider longword bound }
procedure forwd_$uword(n: uword_t; k: integer32); options(val_param);
var
  p: uword_t;
begin
  cnt := 0;
  for p := 1 to n do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for p := 1 to k do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  last := 0;
  for p := 0 to k - 1 do
    last := p;
  exec_$logv_l(cnt + last);
end;
