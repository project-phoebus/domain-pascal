{ Execution test, bead domain-pascal-trdz: the long tail of `for' loops
  dpc refused, now compiled by best-effort paths (ref "For", pp.
  4-88..4-90; codegen-patterns 7.4).  Covered:

  - a row of another array `g[j][i]' (the unit's and the frame's, with
    and without a call in the body) and an array of the enclosing
    routine's frame or through its pointer (from a nested routine): the
    elements indexed, no pointer stepped;
  - `downto' with a run-time bound (from, to or both; a count of 65536,
    the index from 32767 down to -32768);
  - from 0 to a bound through a pointer (an array through a pointer
    stepped in the body; 32768 passes);
  - from a constant other than 0 or 1 to a run-time bound (13, 2 to `k -
    1' with `tab[i + 1]', -1, -100, -1 to `k - 1'; counts above 32767; a
    bound of -32768 below a negative `from');
  - a bound wider than the index (`w - 1' of an `integer32', a longword
    function's result), compared at its own width (-40000: no pass);
  - word arrays reached only by the index of a `downto' loop with a
    run-time bound and of an unsigned index up to one (the index's
    register set up though no other read needs it);
  - from 13 to -32768 (no pass: `blt', not the image's `bmi');
  - an index assigned, addressed (a `var' argument) or read before its
    loop outside its loops, or its own bound: kept in memory;
  - the index read after a loop with a run-time bound (a word index in a
    register, a longword one), after normal termination (one past the
    last value) and after `exit' or `return'; a loop the guard skips
    (Pascal leaves the index undefined: only run, not printed);
  - a register-held local read before the code first assigns it, in a
    loop (its value is not used before the assignment).

  Expected values are the Pascal semantics: an empty range runs no pass
  (p. 4-90); the index after normal termination is one past the last
  value (codegen-patterns 7.4, "The index read after its loop").

  call: FORTL_$ROW w:2
  call: FORTL_$FROW w:3
  call: FORTL_$ENCL w:3
  call: FORTL_$DOWN w:0
  call: FORTL_$DOWN w:1
  call: FORTL_$DOWN w:3
  call: FORTL_$DOWNTO w:6
  call: FORTL_$DOWNTO w:5
  call: FORTL_$DOWNTO w:3
  call: FORTL_$DOWNBIG w:-1 w:0
  call: FORTL_$DOWNBIG w:20000 w:-20000
  call: FORTL_$DOWNBIG w:32767 w:-32768
  call: FORTL_$PTR w:-1
  call: FORTL_$PTR w:0
  call: FORTL_$PTR w:3
  call: FORTL_$PTRBIG w:32767
  call: FORTL_$FROM13 w:12
  call: FORTL_$FROM13 w:13
  call: FORTL_$FROM13 w:15
  call: FORTL_$FROM13 w:-32768
  call: FORTL_$FROM2 w:2
  call: FORTL_$FROM2 w:5
  call: FORTL_$FROMNEG w:-2
  call: FORTL_$FROMNEG w:-1
  call: FORTL_$FROMNEG w:32767
  call: FORTL_$FROMNEG w:-32768
  call: FORTL_$FROM100 w:-101
  call: FORTL_$FROM100 w:5
  call: FORTL_$FROM100 w:32767
  call: FORTL_$FROMFOLD w:-1
  call: FORTL_$FROMFOLD w:0
  call: FORTL_$FROMFOLD w:3
  call: FORTL_$WIDE l:0
  call: FORTL_$WIDE l:3
  call: FORTL_$WIDE l:-40000
  call: FORTL_$REHOME w:10
  call: FORTL_$ADDR
  call: FORTL_$FIND w:0
  call: FORTL_$FIND w:12
  call: FORTL_$FIND w:13
  call: FORTL_$AFTER w:3 w:6
  call: FORTL_$AFTER w:3 w:7
  call: FORTL_$AFTER w:2 w:7
  call: FORTL_$AFTERSKIP w:-1
  call: FORTL_$AFTERSKIP w:2
  call: FORTL_$AFTERL l:69999
  call: FORTL_$AFTERL l:70005
  call: FORTL_$UNDEF w:2
  call: FORTL_$BITS w:2 @w:0
  call: FORTL_$DOWNARR w:3
  call: FORTL_$UPARR w:3
  warns: best effort (not byte-identical): a `for` loop over a row of another array
  warns: best effort (not byte-identical): a `for` loop over an array of the enclosing routine's frame
  warns: best effort (not byte-identical): a `for` loop counting down (`downto`) with a run-time bound
  warns: best effort (not byte-identical): a `for` loop from 0 to a bound reached through a pointer
  warns: best effort (not byte-identical): a `for` loop from a constant other than 0 or 1 to a run-time bound
  warns: best effort (not byte-identical): a `for` bound of another width than the index
  warns: best effort (not byte-identical): the index variable of a `for` loop assigned, addressed or read in part outside its loops
  warns: best effort (not byte-identical): the index variable of a `for` loop with a run-time bound read after the loop
  warns: best effort (not byte-identical): a register-held local read before the code first assigns it
  expect:
    21
    22
    23
    24
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 03 00 00 00 06 00 00 00 09
    3
    6
    9
    12
    15
    mem 00 03 00 04 00 05 00 06 00 07 00 08 00 09 00 0a 00 0b 00 0c
    mark 99
    mark 1
    mark 99
    mark 3
    mark 2
    mark 1
    mark 99
    mark 99
    mark 5
    mark 99
    mark 5
    mark 4
    mark 3
    mark 99
    0
    0
    40001
    0
    65536
    -32768
    mark 99
    mark 0
    mark 99
    mark 0
    mark 1
    mark 2
    mark 3
    mark 99
    mem 00 00 00 02 00 04 00 06 00 00 00 00 00 00 00 00 00 00 00 00
    32768
    536854528
    mark 99
    mark 13
    mark 99
    mark 13
    mark 14
    mark 15
    mark 99
    mark 99
    mem 00 00 00 01 00 02 00 03 00 04 00 05 00 06 00 07 00 08 00 09
    mem 00 00 00 01 00 1e 00 28 00 32 00 05 00 06 00 07 00 08 00 09
    0
    0
    1
    -1
    32769
    536854527
    0
    0
    0
    0
    106
    -5035
    32868
    536849478
    0
    1
    4
    0
    0
    3
    3
    0
    0
    mark 1
    mark 2
    mark 3
    mark 10
    mark 11
    mark 12
    mark 1
    mark 2
    mark 103
    mark 5
    mark 6
    mark 7
    mark 0
    mark 4
    mark 10
    mark 102
    mark 2
    mark 4
    mark 1
    mark 3
    mark 99
    mark 99
    70000
    70001
    mark 4
    mark 2
    mark 1
    mark 4
    mem 00 00 00 07 00 07 00 07 00 00 00 00 00 00 00 00 00 00 00 00
    mem 00 00 00 07 00 07 00 07 00 00 00 00 00 00 00 00 00 00 00 00
}
module fortl;
%include 'exec.ins.pas';

type
  row = array[1..4] of integer32;
  grid = array[1..3] of row;
  rec = record
    pad: integer;
    n: integer;
  end;
  recp = ^rec;
  warr = array[0..9] of integer;
  wp = ^warr;
  uw = 0..65535;

var
  g: grid;
  r: rec;
  tab: warr;
  cnt: integer;

{ a row of the unit's array, set and then logged (a call in the body) }
procedure fortl_$row(j: integer); options(val_param);
var
  i: integer;
begin
  for i := 1 to 4 do
    g[j][i] := j * 10 + i;
  for i := 1 to 4 do
    exec_$logv_l(g[j][i]);
end;

{ rows of a frame array: each cleared, the outer index the row's }
procedure fortl_$frow(j: integer); options(val_param);
var
  fg: grid;
  i, k: integer;
begin
  for k := 1 to 3 do
    for i := 1 to 4 do
      fg[k][i] := 0;
  for i := 1 to 4 do
    fg[j][i] := (i - 1) * j;
  exec_$log_mem(fg, 48);
end;

{ the parent's frame array and an array through the parent's pointer,
  from nested routines }
procedure fortl_$encl(k: integer); options(val_param);
var
  a: array[1..5] of integer32;
  p: wp;
  w: warr;

  procedure fill;
  var
    i: integer;
  begin
    for i := 1 to 5 do
      a[i] := i * k;
    for i := 0 to 9 do
      p^[i] := i + k;
  end;

  procedure show;
  var
    i: integer;
  begin
    for i := 1 to 5 do
      exec_$logv_l(a[i]);
  end;

begin
  p := addr(w);
  fill;
  show;
  exec_$log_mem(w, 20);
end;

{ `downto' from a run-time value to 1 }
procedure fortl_$down(k: integer); options(val_param);
var
  i: integer;
begin
  for i := k downto 1 do
    exec_$mark(i);
  exec_$mark(99);
end;

{ `downto' from 5 to a run-time value }
procedure fortl_$downto(k: integer); options(val_param);
var
  i: integer;
begin
  for i := 5 downto k do
    exec_$mark(i);
  exec_$mark(99);
end;

{ `downto' between two run-time values, no call: the passes counted }
procedure fortl_$downbig(a, b: integer); options(val_param);
var
  i: integer;
  n, s: integer32;
begin
  n := 0;
  s := 0;
  for i := a downto b do begin
    n := n + 1;
    s := s + i;
  end;
  exec_$logv_l(n);
  exec_$logv_l(s);
end;

{ from 0 to a bound through a pointer: with a call, then an array
  through a pointer }
procedure fortl_$ptr(k: integer); options(val_param);
var
  i: integer;
  p: recp;
  q: wp;
begin
  r.n := k;
  p := addr(r);
  for i := 0 to p^.n do
    exec_$mark(i);
  exec_$mark(99);
  for i := 0 to 9 do
    tab[i] := 0;
  q := addr(tab);
  for i := 0 to p^.n do
    q^[i] := i * 2;
  if k = 3 then
    exec_$log_mem(tab, 20);
end;

{ from 0 to a bound through a pointer, 32768 passes }
procedure fortl_$ptrbig(k: integer); options(val_param);
var
  i: integer;
  p: recp;
  n, s: integer32;
begin
  r.n := k;
  p := addr(r);
  n := 0;
  s := 0;
  for i := 0 to p^.n do begin
    n := n + 1;
    s := s + i;
  end;
  exec_$logv_l(n);
  exec_$logv_l(s);
end;

{ from 13 to a run-time value }
procedure fortl_$from13(k: integer); options(val_param);
var
  i: integer;
begin
  for i := 13 to k do
    exec_$mark(i);
  exec_$mark(99);
end;

{ from 2 to `k - 1', the next element read (MMAP's
  mmap_$add_pages_to_wsl) }
procedure fortl_$from2(k: integer); options(val_param);
var
  i: integer;
begin
  for i := 0 to 9 do
    tab[i] := i;
  for i := 2 to k - 1 do
    tab[i] := tab[i + 1] * 10;
  exec_$log_mem(tab, 20);
end;

{ from -1: up to 32769 passes }
procedure fortl_$fromneg(k: integer); options(val_param);
var
  i: integer;
  n, s: integer32;
begin
  n := 0;
  s := 0;
  for i := -1 to k do begin
    n := n + 1;
    s := s + i;
  end;
  exec_$logv_l(n);
  exec_$logv_l(s);
end;

{ from -100 }
procedure fortl_$from100(k: integer); options(val_param);
var
  i: integer;
  n, s: integer32;
begin
  n := 0;
  s := 0;
  for i := -100 to k do begin
    n := n + 1;
    s := s + i;
  end;
  exec_$logv_l(n);
  exec_$logv_l(s);
end;

{ from -1 to `k - 1': the constants folded to 0 }
procedure fortl_$fromfold(k: integer); options(val_param);
var
  i: integer;
  n: integer32;
begin
  n := 0;
  for i := -1 to k - 1 do
    n := n + 1;
  exec_$logv_l(n);
end;

{ a longword bound for a word index: `w - 1' and a longword function's
  result }
procedure fortl_$wide(w: integer32); options(val_param);
var
  i: integer;
  n: integer32;
begin
  n := 0;
  for i := 0 to w - 1 do
    n := n + 1;
  exec_$logv_l(n);
  n := 0;
  for i := 1 to exec_$id_l(w) do
    n := n + 1;
  exec_$logv_l(n);
end;

{ an index assigned after its loop and stepped in another loop
  (AREA_$FREE_SEGMENTS's `n := first_n') }
procedure fortl_$rehome(first: integer); options(val_param);
var
  n, k: integer;
begin
  for n := 1 to 3 do
    exec_$mark(n);
  n := first;
  for k := 1 to 2 do begin
    exec_$mark(n);
    n := n + 1;
  end;
  exec_$mark(n);
end;

procedure fortl_bump(var x: integer); internal;
begin
  x := x + 100;
end;

{ an index passed to a `var' parameter after its loop, then a loop from
  its own value }
procedure fortl_$addr;
var
  n: integer;
begin
  for n := 1 to 2 do
    exec_$mark(n);
  fortl_bump(n);
  exec_$mark(n);
  n := 5;
  for n := n to 6 do
    exec_$mark(n);
  exec_$mark(n);
end;

{ an index assigned a call's result before its loop, read after it
  (PROC2_$PGROUP_SET's new_idx) }
procedure fortl_$find(key: integer); options(val_param);
var
  idx: integer;
begin
  for idx := 0 to 9 do
    tab[idx] := idx * 3;
  idx := exec_$id_w(key);
  if idx <> 0 then begin
    for idx := 1 to 9 do
      if tab[idx] = key then
        exit;
  end;
  exec_$mark(idx);
end;

{ the index read after a loop to a unit variable (XNS_IDP_$REGISTER_ADDR) }
procedure fortl_$after(c: integer; key: integer); options(val_param);
var
  i, j: integer;
begin
  for j := 0 to 9 do
    tab[j] := j * 3;
  cnt := c;
  for i := 0 to cnt do
    if tab[i] = key then begin
      exec_$mark(100 + i);
      return;
    end;
  if i < 4 then
    exec_$mark(1)
  else
    exec_$mark(2);
  exec_$mark(i);
end;

{ the index read after a loop the guard may skip: run, not printed }
procedure fortl_$afterskip(c: integer); options(val_param);
var
  i, t: integer;
begin
  for i := 1 to c do
    tab[i] := 0;
  t := i;
  tab[0] := t;
  exec_$mark(99);
end;

{ a longword index read after its loop: one past the last value, or the
  value at `exit' }
procedure fortl_$afterl(c: integer32); options(val_param);
var
  i: integer32;
begin
  for i := 69998 to c do
    if i = 70001 then
      exit;
  exec_$logv_l(i);
end;

{ a local read in a loop before the statement that assigns it, only
  once that statement has run }
procedure fortl_$undef(k: integer); options(val_param);
var
  x: integer;
  first: boolean;
begin
  first := true;
  repeat
    if not first then
      exec_$mark(x);
    x := k * 2;
    first := false;
    k := k - 1;
  until k < 0;
end;

{ a word set given members one at a time, never cleared (TERM_$INQUIRE's
  flow, 0xE6713C): only the members set are tested }
procedure fortl_$bits(k: integer; out v: integer); options(val_param);
type
  bits = set of 0..15;
var
  flow: bits;
begin
  if k > 0 then
    flow := flow + [0];
  if k > 1 then
    flow := flow + [3];
  if 0 in flow then
    exec_$mark(1);
  if 3 in flow then
    exec_$mark(4);
  v := k;
end;

{ a word array reached only by the index of a `downto' loop with a
  run-time bound }
procedure fortl_$downarr(k: integer); options(val_param);
var
  i: integer;
begin
  for i := 0 to 9 do
    tab[i] := 0;
  for i := k downto 1 do
    tab[i] := 7;
  exec_$log_mem(tab, 20);
end;

{ the same, an unsigned index up to a run-time bound (bead
  domain-pascal-1mtr's path) }
procedure fortl_$uparr(k: uw); options(val_param);
var
  i: uw;
begin
  for i := 0 to 9 do
    tab[i] := 0;
  for i := 1 to k do
    tab[i] := 7;
  exec_$log_mem(tab, 20);
end;
