{ Execution test (bead domain-pascal-338r, domain-c's gap 40): a local
  of 32 KB or more, out of the `link.w' frame (codegen-patterns 2,
  "Locals beyond `link.w`"; best effort, no kernel site): reserved by
  `suba.l' below the frame, the registers saved below it, reached through
  a pointer slot the prologue sets, and the saves restored through A1
  (the DOMAIN C compiler's ra_$collapse_conflict_graph, reported).  The
  array is passed by address, indexed at both ends, and the routine keeps
  values in callee-saved registers across calls; a nested routine
  reaches it through the parent's slot.

  call: BIG_LOCAL_$RUN w:3
  expect:
    7
    9999
    3
    49995000
    10998
}
module big_local;
%include 'exec.ins.pas';

type
  t_t = array [0..9999] of integer32;

procedure fill(var t: t_t; n: integer32);
var
  i: integer;
begin
  for i := 0 to 9999 do t[i] := i;
  t[0] := n;
end;

procedure big_local_$run(k: integer); options(val_param);
var
  t: t_t;
  s: integer32;
  i: integer;

  procedure poke(j: integer);
  begin
    t[j] := t[j] + 1000;
  end;

begin
  fill(t, 7);
  exec_$logv_l(t[0]);
  exec_$logv_l(t[9999]);
  exec_$logv_l(t[k]);
  t[0] := 0;
  s := 0;
  for i := 0 to 9999 do s := s + t[i];
  exec_$logv_l(s);
  poke(9998);
  exec_$logv_l(t[9998]);
end;
