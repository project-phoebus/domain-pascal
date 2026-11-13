{ Execution test (bead domain-pascal-338r, domain-c's gap 4): write and
  writeln to the standard output ("Write, Writeln", pp. 4-215..4-220),
  calls of the run time's PAS_$WRITE (one per item: the format word, the
  file, the item's address, width, digits, the name table) and
  PAS_$WRITELN; the runner's stubs print them.  Strings, character arrays
  with and without a field width (`:-1' trims trailing blanks), chars,
  integers and integer32s with widths, an expression item, booleans, an
  enumeration, a varying string, a double with digits and a pointer (as an
  integer32).  Best effort,
  no kernel site.

  call: WRITES_$RUN w:42
  expect:
    text
    Zonker Harris       F
    Zonker HarrisF
    ZonkF
            42  -7000000|  43
                TRUE|GREEN
    abc|   3.14
      0
}
module writes;
%include 'exec.ins.pas';

type
  colour = (red, green, blue);
var
  name: array [1..20] of char;
  grade: char;
  c: colour;
  v: varying [10] of char;
  d: double;
  b: boolean;
  l: integer32;
  ptr: ^integer;

procedure writes_$run(n: integer); options(val_param);
begin
  writeln('text');
  name := 'Zonker Harris';
  grade := 'F';
  writeln(name, grade);
  writeln(name:-1, grade);
  writeln(name:4, grade);
  l := -7000000;
  writeln(n, l:10, '|', n + 1:4);
  b := n > 3;
  c := green;
  writeln(b:20, '|', c:5);
  v := 'abc';
  d := 3.14159;
  write(v, '|');
  writeln(d:7:2);
  ptr := nil;
  writeln(ptr:3);
end;
