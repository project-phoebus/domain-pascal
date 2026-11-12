{ Execution test (bead domain-pascal-338r, domain-c's gap 22): `ord' of
  a boolean ("Ord", p. 4-137: false 0, true 1), a variable, a packed
  record's field and a comparison (best effort, no kernel site): the
  byte `and' 1.

  call: ORD_BOOL_$RUN w:3
  expect:
    1
    0
    1
    0
    1
    2
}
module ord_bool;
%include 'exec.ins.pas';

var
  r: packed record a: boolean; b: boolean; end;
  b: boolean;

procedure ord_bool_$run(k: integer); options(val_param);
begin
  b := true;
  exec_$logv_w(ord(b));
  b := false;
  exec_$logv_w(ord(b));
  r.a := false;
  r.b := true;
  exec_$logv_w(ord(r.b));
  exec_$logv_w(ord(r.a));
  exec_$logv_w(ord(k > 2));
  exec_$logv_w(ord(k > 2) + ord(k = 3));
end;
