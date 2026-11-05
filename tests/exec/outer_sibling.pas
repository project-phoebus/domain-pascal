{ Execution test: a routine nested two levels in calls a sibling of its
  enclosing routine, which reaches their common parent's local (ref 2.4;
  codegen-patterns 6, "Deeper nesting"; ../source finding 1034, SAU7
  FIM_FP's sp_tiny_result 0x3C46DDCC calling underflow_result 0x3C46D99A
  with the outermost frame as the link).  Best effort: the link is walked
  at the call.

  warns: best effort (not byte-identical): a routine nested in a nested routine
  warns: best effort (not byte-identical): a call of a sibling of an enclosing routine
  call: OUTER_SIBLING_$MAIN w:5
  expect:
    6
    16
}
module outer_sibling;
%include 'exec.ins.pas';

procedure outer_sibling_$main(n: integer); options(val_param);
var x: integer;

  procedure bump(d: integer); options(val_param);
  begin
    x := x + d;
  end;

  procedure mid;
  var y: integer;

    procedure inner;
    begin
      bump(1);
      exec_$log_w(x);
      bump(y);
    end;

  begin
    y := 10;
    inner;
  end;

begin
  x := n;
  mid;
  exec_$log_w(x);
end;
