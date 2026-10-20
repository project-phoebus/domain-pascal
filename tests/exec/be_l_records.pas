{ Execution test (bead domain-pascal-5kgg): 4-byte record function
  results on the lowering's best-effort path (codegen-patterns 3,
  "Record results"; ref 5.1, a function's result is the value last
  assigned to its name): assigned on two paths of an `if`, assigned
  and followed by `return`, and assigned from a parameter passed by
  value.

  call: BE_L_RECORDS_$MAIN w:1
  call: BE_L_RECORDS_$MAIN w:0
  call: BE_L_RECORDS_$MAIN w:-1
  warns: best effort (not byte-identical): a record function result assigned other than once
  warns: best effort (not byte-identical): a record function result assigned from a parameter passed by value
  expect:
    1
    2
    3
    4
    7
    8
    3
    4
    3
    4
    7
    8
    3
    4
    5
    6
    7
    8
}
module be_l_records;
%include 'exec.ins.pas';
type
  rec4 = record a, b: integer; end;
var
  r1, r2, r3: rec4;

function pick(c: integer): rec4; options(val_param);
begin
  if c > 0 then pick := r1 else pick := r2;
end;

function early(c: integer): rec4; options(val_param);
begin
  early := r3;
  if c < 0 then return;
  early := r2;
end;

function same(r: rec4): rec4; options(val_param);
begin
  same := r;
end;

procedure be_l_records_$main(c: integer); options(val_param);
var r: rec4;
begin
  r1.a := 1; r1.b := 2;
  r2.a := 3; r2.b := 4;
  r3.a := 5; r3.b := 6;
  r := pick(c);
  exec_$log_w(r.a);
  exec_$log_w(r.b);
  r := early(c);
  exec_$log_w(r.a);
  exec_$log_w(r.b);
  r.a := 7; r.b := 8;
  r := same(r);
  exec_$log_w(r.a);
  exec_$log_w(r.b);
end;
