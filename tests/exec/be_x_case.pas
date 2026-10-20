{ Execution test (bead domain-pascal-5kgg): best-effort `case' forms
  (codegen-patterns 6): a function call as the selector (its D0 copied),
  a compare chain whose labelled arms are all empty, a chain whose last
  label belongs to its first arm (`beq' to the arm, `bra' over it), and
  label 0 tested first on a computed selector (an explicit `tst').  Only
  selectors with a matching label or an `otherwise' part are used
  (ref 4.x, `case').

  call: BX_CASE_$CALL w:1
  call: BX_CASE_$CALL w:2
  call: BX_CASE_$CALL w:7
  call: BX_CASE_$EMPTY w:1
  call: BX_CASE_$EMPTY w:5
  call: BX_CASE_$EMPTY w:3
  call: BX_CASE_$LASTFIRST w:1
  call: BX_CASE_$LASTFIRST w:9
  call: BX_CASE_$ZERO w:-1
  call: BX_CASE_$ZERO w:4
  warns: best effort (not byte-identical): a function call in a `case` selector
  warns: best effort (not byte-identical): a compare-chain `case` whose arms are all empty
  warns: best effort (not byte-identical): a compare-chain `case` whose last label belongs to its first arm
  warns: best effort (not byte-identical): label 0 tested first on a computed `case` selector
  expect:
    mark 1
    mark 2
    mark 9
    mark 8
    mark 8
    mark 9
    mark 8
    mark 1
    mark 8
    mark 1
    mark 8
    mark 0
    mark 8
    mark 8
}
module be_x_case;
%include 'exec.ins.pas';
var
  gk: integer;

procedure bx_case_$call(k: integer); options(val_param);
begin
  case exec_$id_w(k) of
    1: exec_$mark(1);
    2: exec_$mark(2);
    otherwise exec_$mark(9);
  end;
end;

procedure bx_case_$empty(k: integer); options(val_param);
begin
  gk := k;
  case gk of
    1, 5: ;
    otherwise exec_$mark(9);
  end;
  exec_$mark(8);
end;

procedure bx_case_$lastfirst(k: integer); options(val_param);
begin
  gk := k;
  case gk of
    1, 9: exec_$mark(1);
  end;
  exec_$mark(8);
end;

procedure bx_case_$zero(k: integer); options(val_param);
begin
  case k + 1 of
    0: exec_$mark(0);
    5: ;
  end;
  exec_$mark(8);
end;
