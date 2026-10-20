{ Execution test (bead domain-pascal-ogf8): `case' forms (ref "Case",
  pp. 4-36..4-37; codegen-patterns 7.3).
  - One label (best effort): tested as an `if' is, `cmpi.w #k,Dn; bne'
    to the `otherwise' part or the join (REM_NAME's 0xE4A89A), with and
    without `otherwise', label 0, a longword selector.
  - A constant selector, dispatched at run time (SMD_$INIT 0xE34D50
    `clr.w D0; cmpi.w #4,D0'): a jump table and a compare chain.
  - A jump table entry to an arm of only `return' (best effort: the
    entry left on the arm's `bra').
  - A `dbf' whose target a `bra' follows (best effort, the same): a
    `for' body that starts with a `while'.

  warns: best effort (not byte-identical): a `case` with one label
  warns: best effort (not byte-identical): a `case` table entry whose target a `bra` follows
  warns: best effort (not byte-identical): a `dbf` whose target a `bra` follows
  call: OGF8_CASES_$ONE w:1 l:5
  call: OGF8_CASES_$ONE w:2 l:0
  call: OGF8_CASES_$ONE w:0 l:-5
  call: OGF8_CASES_$CONST
  call: OGF8_CASES_$RET w:0
  call: OGF8_CASES_$RET w:1
  call: OGF8_CASES_$RET w:2
  call: OGF8_CASES_$RET w:4
  call: OGF8_CASES_$RET w:7
  call: OGF8_CASES_$DBF w:3 w:4
  call: OGF8_CASES_$DBF w:0 w:1
  expect:
    10
    3
    3
    3
    20
    -1
    -1
    20
    3
    1
    1
    3
    6
    7
    2
    mark 0
    mark 2
    mark 4
    mark 9
    0
    5
    0
    2
}
module ogf8_cases;
%include 'exec.ins.pas';

const
  kz = 0;
  kt = 2;

var
  w: integer;

procedure ogf8_cases_$one(x: integer; y: integer32); options(val_param);
begin
  case x of
    1: w := 10;
    otherwise w := 20;
  end;
  exec_$logv_w(w);
  w := 3;
  case x of
    2: w := -1;
  end;
  exec_$logv_w(w);
  case x of
    0: w := 1;
  end;
  exec_$logv_w(w);
  case y of
    0: w := 0;
  otherwise
    exec_$logv_w(w);
  end;
end;

procedure ogf8_cases_$const;
begin
  case kz of
    0: w := 3;
    1: w := 4;
    2: w := 5;
    3: w := 9;
  end;
  exec_$logv_w(w);
  case kt of
    0: w := 1;
    1: w := 2;
    2: w := 6;
  end;
  exec_$logv_w(w);
  case kt of
    0: w := 1;
    1: w := 2;
    otherwise w := 7;
  end;
  exec_$logv_w(w);
  case kz of
    0: w := 2;
  end;
  exec_$logv_w(w);
end;

procedure ogf8_cases_$ret(x: integer); options(val_param);
begin
  w := 9;
  case x of
    0: w := 0;
    1: return;
    2: w := 2;
    3: w := 3;
    4: w := 4;
  end;
  exec_$mark(w);
end;

procedure ogf8_cases_$dbf(a: integer; b: integer); options(val_param);
var
  i: integer;
  x, y: integer;
begin
  x := a;
  y := b - 4;
  for i := 1 to 5 do begin
    while x > 0 do
      x := x - 1;
    y := y + 1;
  end;
  exec_$logv_w(x);
  exec_$logv_w(y);
end;
