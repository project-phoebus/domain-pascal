{ Execution test: a boolean function whose whole body is `f := true'
  (`st D0', DUMMY_INTERRUPT_HANDLER 0xE6D46C) or `f := false' (`clr.b
  D0', IIC_$EXISTS 0xE70AB0); codegen-patterns 3 ("Boolean constants").
  The results read from D0.b by the harness, and tested and stored by a
  compiled caller.

  call: BCR_$YES -> b
  call: BCR_$NO -> b
  call: BCR_$MAIN
  expect:
    => true
    => false
    true
    false
    1
}
module bool_const_result;
%include 'exec.ins.pas';

var
  gb: boolean;

function bcr_$yes: boolean;
begin
  bcr_$yes := true;
end;

function bcr_$no: boolean;
begin
  bcr_$no := false;
end;

procedure bcr_$main;
begin
  gb := bcr_$yes;
  exec_$log_bool(gb);
  gb := bcr_$no;
  exec_$log_bool(gb);
  if bcr_$yes and not bcr_$no then exec_$logv_w(1) else exec_$logv_w(0);
end;
