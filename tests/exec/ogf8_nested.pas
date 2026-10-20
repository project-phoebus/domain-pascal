{ Execution test (bead domain-pascal-ogf8, best effort): routines nested
  in nested routines (ref 2.4, pp. 2-19..2-21; codegen-patterns 6,
  "Deeper nesting").  Each nested routine whose nested routines reach
  past it stores its static link in its link slot `(-4,A6)' (DIR's
  0xE4F3BA `move.l A1,(-4,A6)'), and the deeper routines walk the chain
  (0xE4EF8C `movea.l (A6),A0; movea.l (-4,A0),A2'): reads and writes of
  the top routine's by-value and `var' parameters, a local only a
  level-2 routine reaches (kept in memory in the top routine), an array
  of the top frame indexed at run time, the middle routine's own local;
  a level-2 routine called by its sibling (the link passed in A1) and
  recursively; a level-2 function; four levels.

  warns: best effort (not byte-identical): a routine nested in a nested routine
  call: OGF8_NESTED_$THREE w:5 @l:7!
  call: OGF8_NESTED_$THREE w:-2 @l:0!
  call: OGF8_NESTED_$FOUR w:3
  expect:
    28
    5
    9
    10
    11
    3
    13
    out 43
    21
    -2
    2
    3
    4
    3
    6
    out 22
    111
    1234
}
module ogf8_nested;
%include 'exec.ins.pas';

procedure ogf8_nested_$three(y: integer; var x: integer32);
  options(val_param);
var
  t: integer32;
  u: integer;
  arr: array[1..4] of integer;
  i: integer;

  procedure mid(z: integer);
  var
    m: integer;

    function inner(k: integer): integer32;
    begin
      inner := t + u + m + k;
    end;

    procedure count(n: integer);
    begin
      if n > 0 then begin
        arr[n] := n + y;
        count(n - 1);
      end;
    end;

    procedure both;
    begin
      count(3);
      x := inner(0) + x;
    end;

  begin
    m := z;
    t := inner(10);
    both;
  end;

begin
  t := 10;
  u := y;
  for i := 1 to 4 do
    arr[i] := 0;
  mid(3);
  exec_$log_l(t);
  exec_$logv_w(u);
  for i := 1 to 4 do
    exec_$logv_w(arr[i] + 3);
  exec_$logv_w(u + 8);
end;

procedure ogf8_nested_$four(a: integer); options(val_param);
var
  outer: integer32;

  procedure l1;
  var
    v1: integer;

    procedure l2;
    var
      v2: integer;

      procedure l3;
      begin
        outer := outer + a * 100 + v1 * 10 + v2;
      end;

    begin
      v2 := a - 2;
      l3;
    end;

  begin
    v1 := a - 2;
    l2;
  end;

begin
  outer := -200;
  l1;
  exec_$log_l(outer);
  outer := 1234;
  exec_$log_l(outer);
end;
