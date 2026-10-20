{ Execution test (bead domain-pascal-5kgg): best-effort forms of another
  data section's block (codegen-patterns 11): routines in their own
  sections reaching the unit's block with branches (the block's address
  loaded again after each label), and an element of an array in that
  block indexed at run time (the block's base, then the index; bead
  domain-pascal-hxwq).

  call: BX_SECT_$SET @l:40
  call: BX_SECT_$BRANCHY @w:1
  call: BX_SECT_$BRANCHY @w:0
  call: BX_SECT_$ELEM w:3 w:-6
  call: BX_SECT_$ELEM w:1 w:9
  call: BX_SECT_$SHOW
  warns: best effort (not byte-identical): a routine that reaches another data section's block and branches
  warns: best effort (not byte-identical): an element of an array in another data section's block
  expect:
    8
    9
    0
    8
    9
    3
    9
    0
    -6
}
module be_x_sect, bx_sect_proc, bx_sect_data;
%include 'exec.ins.pas';

var
  a: integer32;
  b: integer32;
  c: integer;
  t: array[1..4] of integer;

procedure bx_sect_$set(in x: integer32);
begin
  a := x;
  b := x + 1;
  c := 0;
end;

[section(bx_sect_init_proc, bx_sect_init_data)]
procedure bx_sect_$branchy(in n: integer);
begin
  a := 7;
  if n > 0 then
    b := 9
  else
    c := 3;
  a := a + 1;
  exec_$log_l(a);
  exec_$log_l(b);
  exec_$log_w(c);
end;

[section(bx_sect_init_proc, bx_sect_init_data)]
procedure bx_sect_$elem(i, v: integer); options(val_param);
begin
  t[i] := v;
end;

procedure bx_sect_$show;
begin
  exec_$log_w(t[1]);
  exec_$log_w(t[2]);
  exec_$log_w(t[3]);
end;
