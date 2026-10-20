{ Execution test (bead domain-pascal-5kgg): best-effort comparisons and
  `for' loops (codegen-patterns 7.1, 7.4): two pointer variables
  compared (`cmp.l' through a data register), an ordered test of a
  parameter right after its entry load (on the load's flags, which `move'
  sets as `tst' does), a plain `or' whose right operand calls and whose
  left operand is not a comparison, a `for' index whose initial value
  equals the `dbf' count (count and index loaded apart), and a `for' loop
  with a run-time bound read from memory and no call in its body.

  call: BX_CMPFOR_$PTRS w:3 w:3
  call: BX_CMPFOR_$PTRS w:3 w:4
  call: BX_CMPFOR_$ENTRYORD w:5 w:11
  call: BX_CMPFOR_$ENTRYORD w:0 w:12
  call: BX_CMPFOR_$ENTRYORD w:-5 w:13
  call: BX_CMPFOR_$ANDCALL w:5
  call: BX_CMPFOR_$ANDCALL w:7
  call: BX_CMPFOR_$ANDCALL w:12
  call: BX_CMPFOR_$ANDCALL w:1
  call: BX_CMPFOR_$EQCOUNT
  call: BX_CMPFOR_$NOCALL w:6
  call: BX_CMPFOR_$NOCALL w:0
  warns: best effort (not byte-identical): comparing two pointer variables
  warns: best effort (not byte-identical): an ordered test of a parameter right after its entry load
  warns: best effort (not byte-identical): a plain `and`/`or` with a call in its right operand
  warns: best effort (not byte-identical): a `for` index whose initial value equals the `dbf` count
  warns: best effort (not byte-identical): a `for` loop with a run-time bound and no call in its body
  expect:
    mark 1
    mark 0
    mark 1
    mark 11
    mark 0
    mark 12
    mark 0
    mark 13
    mark 1
    mark 1
    mark 0
    mark 0
    mark 4
    mark 5
    mark 6
    mark 7
    mark 8
    mark 21
    mark 0
}
module be_x_cmpfor;
%include 'exec.ins.pas';
type
  pint = ^integer;
var
  gn, gr: integer;
  gp, gq: pint;

procedure bx_cmpfor_$ptrs(a, b: integer); options(val_param);
var x, y: integer;
begin
  x := a; y := b;
  gp := addr(x);
  if a = b then gq := addr(x) else gq := addr(y);
  if gp = gq then gr := 1 else gr := 0; exec_$mark(gr);
end;

procedure bx_cmpfor_$entryord(k: integer; j: integer); options(val_param);
begin
  if k > 0 then gr := 1 else gr := 0; exec_$mark(gr);
  exec_$mark(j);
end;

procedure bx_cmpfor_$andcall(k: integer); options(val_param);
var b: boolean;
begin
  b := k > 2;
  if (b and (k < 10)) or (exec_$id_w(k) = 7) then gr := 1 else gr := 0; exec_$mark(gr);
end;

procedure bx_cmpfor_$eqcount; options(val_param);
var i: integer;
begin
  for i := 4 to 8 do exec_$mark(i);
end;

procedure bx_cmpfor_$nocall(n: integer); options(val_param);
var i, s: integer;
begin
  s := 0;
  gn := n;
  for i := 1 to gn do s := s + i;
  exec_$mark(s);
end;
