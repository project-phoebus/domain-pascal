{ Execution test (bead domain-pascal-5kgg review, blocker): record function
  results from by-value 4-byte record parameters: the first field stored
  then the whole returned (`r.a := 99; f := r'), a field stored on one
  path, the result assigned twice.

  call: BE_X_RECORD_PARAM_$MAIN w:1
  warns: best effort (not byte-identical): assigning to part of a by-value parameter
  warns: best effort (not byte-identical): a record function result assigned from a parameter passed by value
  expect:
    99
    8
    7
    42
    5
    6
}
module be_x_record_param;
%include 'exec.ins.pas';
type
  rec4 = record a, b: integer; end;

function modfirst(r: rec4): rec4; options(val_param);
begin
  r.a := 99;
  modfirst := r;
end;

function cond(r: rec4; c: integer): rec4; options(val_param);
begin
  if c > 0 then r.b := 42;
  exec_$clobber;
  cond := r;
end;

function twice(r: rec4; s: rec4; c: integer): rec4; options(val_param);
begin
  twice := r;
  if c > 0 then twice := s;
end;

procedure be_x_record_param_$main(c: integer); options(val_param);
var r, s: rec4;
begin
  r.a := 7; r.b := 8;
  s := modfirst(r);
  exec_$log_w(s.a);
  exec_$log_w(s.b);
  s := cond(r, c);
  exec_$log_w(s.a);
  exec_$log_w(s.b);
  s.a := 5; s.b := 6;
  r := twice(r, s, c);
  exec_$log_w(r.a);
  exec_$log_w(r.b);
end;
