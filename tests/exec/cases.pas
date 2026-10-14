{ Execution test: `case' (ref 4.2 "Case", p. 4-36; codegen-patterns 8):
  a dense label set as a jump table (with a gap, `otherwise', values
  below and above the range), a sparse one as a compare chain, negative
  labels, char selectors.

  call: CASES_$TABLE w:-1
  call: CASES_$TABLE w:0
  call: CASES_$TABLE w:1
  call: CASES_$TABLE w:2
  call: CASES_$TABLE w:3
  call: CASES_$TABLE w:4
  call: CASES_$TABLE w:5
  call: CASES_$TABLE w:6
  call: CASES_$TABLE w:-32768
  call: CASES_$CHAIN w:3
  call: CASES_$CHAIN w:8
  call: CASES_$CHAIN w:9
  call: CASES_$CHAIN w:10
  call: CASES_$CHAIN w:11
  call: CASES_$NEG w:-3
  call: CASES_$NEG w:-5
  call: CASES_$NEG w:3
  call: CASES_$CHAR b:100
  call: CASES_$CHAR b:104
  call: CASES_$CHAR b:108
  call: CASES_$CHAR b:122
  expect:
    mark 99
    mark 10
    mark 11
    mark 12
    mark 99
    mark 14
    mark 15
    mark 99
    mark 99
    mark 5
    mark 6
    mark 0
    mark 5
    mark 7
    mark 3
    mark 0
    mark 0
    mark 1
    mark 2
    mark 3
    mark 9
}
module cases;
%include 'exec.ins.pas';

var
  gk: integer;
  gc: char;

procedure cases_$table(k: integer); options(val_param);
begin
  gk := k;
  case gk of
    0: exec_$mark(10);
    1: exec_$mark(11);
    2: exec_$mark(12);
    4: exec_$mark(14);
    5: exec_$mark(15);
    otherwise exec_$mark(99);
  end;
end;

procedure cases_$chain(k: integer); options(val_param);
begin
  gk := k;
  case gk of
    3, 10: exec_$mark(5);
    8: exec_$mark(6);
    9: exec_$mark(0);
    otherwise exec_$mark(7);
  end;
end;

procedure cases_$neg(k: integer); options(val_param);
begin
  gk := k;
  case gk of
    -1: exec_$mark(1);
    -2: exec_$mark(2);
    -3: exec_$mark(3);
    -4: exec_$mark(4);
    otherwise exec_$mark(0);
  end;
end;

procedure cases_$char(c: char); options(val_param);
begin
  gc := c;
  case gc of
    'd': exec_$mark(1);
    'f', 'h': exec_$mark(2);
    'l': exec_$mark(3);
    otherwise exec_$mark(9);
  end;
end;
