{ Execution test: `for' indices beyond the loop body's own reads
  (ref "For", pp. 4-89..4-90; codegen-patterns 7.4; bead domain-pascal-h1io).
  - A longword bound read as its low word (as an assignment narrows,
    reference-clarifications Q41): a value parameter's 65538 counts 2.
  - The index passed by address (`exec_$log_w' takes `in v' by address,
    ref 5.2): its register stored to its slot before each call.
  - The index read after its loop: after `exit' the value at the exit (ref
    p. 4-90 lists `exit' among the ways out); after normal termination the
    reference "forbids the use of the value" (message 235, Diagnostic
    Messages p. 9-31), and the value below
    (one past the upper bound, `find' of 99) is the image's, not the
    reference's: ACL_$CHECK_DEBUG_RIGHTS 0xE48BBC `cmpi.w #8,D1' and
    PROC2_$PGROUP_SET 0xE41F0E `cmpi.w #70,D0' after their loops.
  - Two loops over one index in a row, read after the second.
  - An element at an index read through another element,
    `cls[codes[f]]'.
  The index is stored to a variable before the call that logs it: a call
  whose argument is the last use of an earlier statement's value, after
  branches, is refused (codegen-patterns 12.4 R4(c)).

  call: FORIX_$SETUP l:2
  call: FORIX_$UPTO l:0
  call: FORIX_$UPTO l:3
  call: FORIX_$UPTO l:65538
  call: FORIX_$ALL
  call: FORIX_$FIND l:30
  call: FORIX_$FIND l:100
  call: FORIX_$FIND l:99
  call: FORIX_$TWICE
  call: FORIX_$CLASSES
  expect:
    1
    2
    3
    1
    2
    1
    2
    mark 3
    mark 10
    mark 11
    mark 4
    1
    2
    3
    4
}
module forix;
%include 'exec.ins.pas';

type
  ent = record
    ref: integer32;
    tag: integer;
  end;

var
  tab: array[1..10] of ent;
  count: integer32;
  codes: array[1..4] of integer;
  hold: integer;
  cls: array[0..127] of integer;

procedure forix_$setup(n: integer32); options(val_param);
var
  i: integer;
begin
  count := n;
  for i := 1 to 10 do begin
    tab[i].ref := i * 10;
    tab[i].tag := 0;
  end;
  for i := 1 to 4 do
    codes[i] := i + 8;
end;

{ a longword value parameter as the bound; the index by address }
procedure forix_$upto(n: integer32); options(val_param);
var
  i: integer;
begin
  for i := 1 to n do
    exec_$log_w(i);
end;

{ a longword of the unit's data as the bound }
procedure forix_$all;
var
  i: integer;
begin
  for i := 1 to count do
    exec_$log_w(i);
end;

{ a search: the index after the loop }
procedure forix_$find(key: integer32); options(val_param);
var
  i: integer;
begin
  for i := 1 to 10 do
    if tab[i].ref = key then
      exit;
  hold := i;
  exec_$mark(hold);
end;

procedure forix_$twice;
var
  i: integer;
begin
  for i := 1 to 3 do
    tab[i].tag := i;
  for i := 1 to 5 do
    if tab[i].tag = 0 then
      exit;
  hold := i;
  exec_$mark(hold);
end;

procedure forix_$classes;
var
  f: integer;
begin
  for f := 1 to 4 do
    cls[codes[f]] := f;
  exec_$logv_w(cls[9]);
  exec_$logv_w(cls[10]);
  exec_$logv_w(cls[11]);
  exec_$logv_w(cls[12]);
end;
