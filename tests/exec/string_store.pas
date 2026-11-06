{ Execution test (bead domain-pascal-vq0h; ref "Variable-Length String
  Operations", p. 4-204; codegen-patterns 5.2, "String constants"): a string
  constant assigned to a longer array of char, every form: immediates, the
  cell, a repeated character, all blanks, by longwords and by bytes, at an odd
  count, through a pointer, and checked to the last byte so that no store runs
  past the array.

  call: STRS_$RUN
  expect:
    1
    1
    1
    1
    1
    1
    1
    1
    1
    1
    1
    1
    1
}
module strs;
%include 'exec.ins.pas';

const n = 40;
type buf = array[1..n] of char;
     holder = record b: buf; guard: integer32; end;

var g: holder;

function ok(in b: buf; in want: buf): integer32;
var i: integer;
begin
  ok := 1;
  for i := 1 to n do
    if b[i] <> want[i] then ok := 0;
end;

procedure check(in b: buf; in want: buf; guard: integer32);
var v: integer32;
begin
  v := ok(b, want);
  if guard <> 16#1234 then v := 0;
  exec_$log_l(v);
end;

procedure by_pointer(out d: buf; which: integer);
begin
  case which of
    1: d := 'x';
    2: d := 'xy';
    3: d := 'abc';
    4: d := 'abcdef';
    5: d := '-----';
    otherwise d := '     ';
  end;
end;

procedure exec_all;
var want: buf;
    f: holder;
begin
  g.guard := 16#1234;
  f.guard := 16#1234;

  g.b := 'x';
  want := 'x                                       ';
  check(g.b, want, g.guard);
  g.b := 'xy';
  want := 'xy                                      ';
  check(g.b, want, g.guard);
  f.b := 'abc';
  want := 'abc                                     ';
  check(f.b, want, f.guard);
  f.b := 'abcde';
  want := 'abcde                                   ';
  check(f.b, want, f.guard);
  g.b := 'abcdefg';
  want := 'abcdefg                                 ';
  check(g.b, want, g.guard);
  g.b := 'abcdefghijklmnopqrstuvw';
  want := 'abcdefghijklmnopqrstuvw                 ';
  check(g.b, want, g.guard);
  f.b := '        ';
  want := '                                        ';
  check(f.b, want, f.guard);

  by_pointer(g.b, 1);
  want := 'x                                       ';
  check(g.b, want, g.guard);
  by_pointer(g.b, 2);
  want := 'xy                                      ';
  check(g.b, want, g.guard);
  by_pointer(g.b, 3);
  want := 'abc                                     ';
  check(g.b, want, g.guard);
  by_pointer(g.b, 4);
  want := 'abcdef                                  ';
  check(g.b, want, g.guard);
  by_pointer(g.b, 5);
  want := '-----                                   ';
  check(g.b, want, g.guard);
  by_pointer(g.b, 6);
  want := '                                        ';
  check(g.b, want, g.guard);
end;

procedure strs_$run;
begin
  exec_all;
end;
