{ Execution test: `for' loops from 0 and from 1 to a run-time upper bound
  with no call in the body (ref 4.2 "For", p. 4-89: no iteration when the
  range is empty; codegen-patterns 7.4, "The count's register"; bead
  domain-pascal-fbah): the bound a value parameter dying at the bound, the
  count copied back into its register (the MAC_OS_ module-local 0xE0B202,
  the RING_ module-local 0xE76422), each for an empty range, one iteration
  and several, with an early `return' out of the loop; and a loop with no
  index register, a frame array through a stepped pointer.

  call: FORNC_$SETUP
  call: FORNC_$FIND w:0 l:23
  call: FORNC_$FIND w:1 l:3
  call: FORNC_$FIND w:1 l:23
  call: FORNC_$FIND w:5 l:43
  call: FORNC_$FIND w:20 l:193
  call: FORNC_$FIND1 w:0 l:23
  call: FORNC_$FIND1 w:1 l:13
  call: FORNC_$FIND1 w:4 l:33
  call: FORNC_$FIND1 w:3 l:33
  call: FORNC_$FILLS w:0 l:7
  call: FORNC_$FILLS w:1 l:7
  call: FORNC_$FILLS w:3 l:7
  expect:
    mark -1
    mark 0
    mark -1
    mark 4
    mark 19
    mark 0
    mark 1
    mark 3
    mark 3
    0
    7
    0
    7
    7
    7
    0
}
module fornc;
%include 'exec.ins.pas';

type
  pair = record
    lo, hi: integer32;
    port: integer32;
  end;
  pairs = array[0..19] of pair;
  pairs1 = array[1..20] of pair;

var
  tab: pairs;
  tab1: pairs1;

{ from 0 to `count - 1' (0xE0B202) }
function fornc_$find_in(key: integer32; in t: pairs; count: integer): integer;
  internal;
var
  i: integer;
begin
  for i := 0 to count - 1 do
    with t[i] do
      if (key >= lo) and then (key <= hi) then begin
        fornc_$find_in := i;
        return;
      end;
  fornc_$find_in := -1;
end;

{ from 1 to `count' (0xE76422) }
function fornc_$find1_in(a, b: integer32; in t: pairs1; count: integer): integer;
  internal;
var
  i: integer;
begin
  for i := 1 to count do
    with t[i] do
      if (b <= hi) and then (a >= lo) then begin
        fornc_$find1_in := i;
        return;
      end;
  fornc_$find1_in := 0;
end;

{ entry j covers 10 j .. 10 j + 5 }
procedure fornc_$setup;
var
  j: integer;
begin
  j := 0;
  while j < 20 do begin
    tab[j].lo := j * 10;
    tab[j].hi := j * 10 + 5;
    tab1[j + 1].lo := (j + 1) * 10;
    tab1[j + 1].hi := (j + 1) * 10 + 5;
    j := j + 1;
  end;
end;

procedure fornc_$find(n: integer; key: integer32); options(val_param);
var
  r: integer;
begin
  r := fornc_$find_in(key, tab, n);
  exec_$mark(r);
end;

{ the range key .. key + 1 }
procedure fornc_$find1(n: integer; key: integer32); options(val_param);
var
  r: integer;
begin
  r := fornc_$find1_in(key, key + 1, tab1, n);
  exec_$mark(r);
end;

{ no index to set up: a frame array through a stepped pointer formed
  after the count's copy }
procedure fornc_$fill_in(n: integer; k: integer32; var t: pairs1); internal;
var
  i: integer;
  u: array[1..4] of integer32;
begin
  u[1] := 0;
  u[2] := 0;
  u[3] := 0;
  u[4] := 0;
  for i := 1 to n do
    u[i] := k;
  t[1].lo := u[1];
  t[2].lo := u[2];
  t[3].lo := u[3];
  t[4].lo := u[4];
end;

{ logs the first n + 1 (at most 4) values the fill left }
procedure fornc_$fills(n: integer; k: integer32); options(val_param);
begin
  fornc_$fill_in(n, k, tab1);
  exec_$logv_l(tab1[1].lo);
  if n >= 1 then
    exec_$logv_l(tab1[2].lo);
  if n >= 2 then
    exec_$logv_l(tab1[3].lo);
  if n >= 3 then
    exec_$logv_l(tab1[4].lo);
end;
