{ Execution test: a pointer local assigned a pointer function's result
  while a by-address parameter is held across the call (codegen-patterns
  12.4 R1b; bead domain-pascal-ue7).  In CALL_RESULT_FIRST_$DIES the
  status address dies in the assignment's block (`if st <> 0 then
  return'), so R1b gives the local the lower register; in
  CALL_RESULT_FIRST_$KEPT the status is used again after that block, so
  address order holds.  Either way both values must survive the calls
  that follow, and the status cell must get the function's store.

  call: CALL_RESULT_FIRST_$DIES l:3 @l:99!
  call: CALL_RESULT_FIRST_$DIES l:-1 @l:99!
  call: CALL_RESULT_FIRST_$KEPT l:4 @l:99!
  call: CALL_RESULT_FIRST_$KEPT l:-2 @l:99!
  expect:
    5
    8
    out 0
    out 1
    8
    12
    out 13
    out 1
}
module call_result_first;
%include 'exec.ins.pas';

type
  ip = ^integer32;
var
  g: integer32 := 5;
  gp: ip := addr(g);

function getp(in k: integer32; out st: integer32): ip;
begin
  if k < 0 then
    st := 1
  else
    st := 0;
  getp := gp;
end;

procedure call_result_first_$dies(k: integer32; out st: integer32);
  options(val_param);
var
  p: ip;
begin
  p := getp(k, st);
  if st <> 0 then
    return;
  exec_$logv_l(p^);
  p^ := p^ + k;
  exec_$logv_l(p^);
end;

procedure call_result_first_$kept(k: integer32; out st: integer32);
  options(val_param);
var
  p: ip;
begin
  p := getp(k, st);
  if st <> 0 then
    return;
  exec_$logv_l(p^);
  p^ := p^ + k;
  exec_$logv_l(p^);
  st := p^ + 1;
end;
