{ Execution test (bead domain-pascal-338r, domain-c's gap 9): nested
  routines that call each other, one declared `forward' (ref 5.5.1),
  both reaching the parent's local (codegen-patterns 6; best effort, no
  kernel site): the forward routine emitted at its body, as in the
  DOMAIN C compiler's il_utils_$search_tree_for_node (reported: search_point
  0x07E206 before search_walk 0x07E252).

  call: NESTED_FORWARD_$RUN w:5
  expect:
    3
    15
}
module nested_forward;
%include 'exec.ins.pas';

procedure nested_forward_$run(n: integer); options(val_param);
var
  calls, sum: integer;

  procedure b(k: integer); forward;

  procedure a(k: integer);
  begin
    calls := calls + 1;
    sum := sum + k;
    if k > 0 then b(k - 1);
  end;

  procedure b;
  begin
    sum := sum + k;
    if k > 0 then a(k - 1);
  end;

begin
  calls := 0;
  sum := 0;
  a(n);
  exec_$logv_w(calls);
  exec_$logv_w(sum);
end;
