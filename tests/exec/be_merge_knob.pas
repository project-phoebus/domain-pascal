{ Execution test (bead domain-pascal-5kgg): the best-effort paths of the
  store-merge refusals in a unit that merges (`dpc
  --merge-constant-stores`; codegen-patterns 13, "Store merging"):

  - four word constants through a pointer: the first two merged into one
    longword (big-endian: `a` at the lower address), the rest apart
    (runs of four pair from the first at TTY_$K_RESET 0xE67322, not at
    KBD_$INIT 0xE33378);
  - three word locals stored in ascending order, the first two kept
    apart (frame words), so all three apart;
  - two word locals in descending order;
  - two word constants to another unit's adjacent words (no such pair in
    a merging unit): apart.

  call: BE_MERGE_KNOB_$FOUR
  call: BE_MERGE_KNOB_$THREE
  call: BE_MERGE_KNOB_$DOWN
  call: BE_MERGE_KNOB_$EXT
  link: be_merge_data.mod.pas
  dpc: --merge-constant-stores
  warns: best effort (not byte-identical): under --merge-constant-stores, a fourth word constant
  warns: best effort (not byte-identical): under --merge-constant-stores, two word constants stored to adjacent words
  expect:
    1
    2
    3
    4
    1
    2
    3
    10
    20
    7
    8
}
module be_merge_knob;
%include 'exec.ins.pas';
type
  four_t = record a, b, c, d: integer; end;
  four_ptr = ^four_t;
  pair_t = record a: integer; b: integer; end;
var
  f: four_t;
  bm_$ext: extern pair_t;

procedure four(p: four_ptr); options(val_param, internal);
begin
  p^.a := 1;
  p^.b := 2;
  p^.c := 3;
  p^.d := 4;
end;

procedure be_merge_knob_$four;
begin
  f.a := 0;
  f.b := 0;
  exec_$clobber;
  f.c := 0;
  exec_$clobber;
  f.d := 0;
  four(addr(f));
  exec_$log_w(f.a);
  exec_$log_w(f.b);
  exec_$log_w(f.c);
  exec_$log_w(f.d);
end;

procedure be_merge_knob_$three;
var x, y, z: integer;
begin
  z := 3;
  y := 2;
  x := 1;
  exec_$log_w(x);
  exec_$log_w(y);
  exec_$log_w(z);
end;

procedure be_merge_knob_$down;
var x, y: integer;
begin
  x := 10;
  y := 20;
  exec_$log_w(x);
  exec_$log_w(y);
end;

procedure be_merge_knob_$ext;
begin
  bm_$ext.a := 7;
  bm_$ext.b := 8;
  exec_$log_w(bm_$ext.a);
  exec_$log_w(bm_$ext.b);
end;
