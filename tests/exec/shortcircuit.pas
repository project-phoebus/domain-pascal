{ Execution test: `and then' / `or else' (ref p. 4-18, 4-134) evaluate
  their right operand only when needed; the right operand here is a
  function with a visible side effect (a mark).

  call: SHORTCIRCUIT_$MAIN w:0
  call: SHORTCIRCUIT_$MAIN w:1
  call: SHORTCIRCUIT_$MAIN w:2
  expect:
    false
    mark 50
    false
    mark 50
    false
    true
    mark 50
    true
    true
}
module shortcircuit;
%include 'exec.ins.pas';

var
  g: integer;
  r: boolean;

function probe(v: integer): boolean; options(val_param);
begin
  exec_$mark(50);
  probe := v > 1;
end;

procedure shortcircuit_$main(k: integer); options(val_param);
begin
  g := k;
  if (g <> 0) and then probe(g) then r := true else r := false;
  exec_$log_bool(r);
  if (g <> 0) or else probe(g) then r := true else r := false;
  exec_$log_bool(r);
end;
