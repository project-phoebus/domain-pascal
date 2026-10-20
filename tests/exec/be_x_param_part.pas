{ Execution test (bead domain-pascal-5kgg review, blocker): a store of the
  first field of a by-value 4-byte record parameter (`r.a := 99', a word
  at the parameter's own offset) is a part of it, not the whole: the
  parameter stays in its slot and its other field keeps its value.
  Earlier the store was taken for the whole parameter held in D0
  (`moveq #0x63,D0'), clearing `r.b'.

  call: BE_X_PARAM_PART_$P l:0x00070008
  call: BE_X_PARAM_PART_$Q l:0x00070008
  warns: best effort (not byte-identical): assigning to part of a by-value parameter
  expect:
    99
    8
    99
    8
}
module be_x_param_part;
%include 'exec.ins.pas';
type
  rec4 = record a, b: integer; end;
var g: rec4;

procedure be_x_param_part_$p(r: rec4); options(val_param);
var i: integer;
begin
  r.a := 99;
  g := r;
  i := g.a;
  exec_$log_w(i);
  i := g.b;
  exec_$log_w(i);
end;

procedure be_x_param_part_$q(r: rec4); options(val_param);
var i: integer;
begin
  r.a := 99;
  i := r.a;
  exec_$log_w(i);
  i := r.b;
  exec_$log_w(i);
end;
