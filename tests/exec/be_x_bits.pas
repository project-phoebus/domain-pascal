{ Execution test (bead domain-pascal-5kgg): best-effort bit tests
  (codegen-patterns 5.1, 7.1): members 7, 15 and 31 of word and
  longword sets in memory tested (the set loaded, `btst #k'), after `[k]'
  with a run-time member assigned to them.  Sets are big-endian bit
  strings, member k is bit k of the whole (ref 3.9.3).

  call: BX_BITS_$SETS w:15
  call: BX_BITS_$SETS w:7
  call: BX_BITS_$LSETS w:31
  call: BX_BITS_$LSETS w:15
  call: BX_BITS_$LSETS w:3
  warns: best effort (not byte-identical): `in` testing member 7, 15 or 31 of a set in a register
  warns: best effort (not byte-identical): assigning `[x]` with a run-time member to a set
  expect:
    mark 1
    mark 0
    mark 0
    mark 1
    mark 1
    mark 0
    mark 0
    mark 1
    mark 0
    mark 0
}
module be_x_bits;
%include 'exec.ins.pas';
type
  wset = set of 0..15;
  lset = set of 0..31;
var
  ws: array[1..2] of wset;
  ls: array[1..2] of lset;
  gk: integer;

procedure bx_bits_$sets(k: integer); options(val_param);
begin
  gk := 2;
  ws[gk] := [k];
  if 15 in ws[gk] then exec_$mark(1) else exec_$mark(0);
  if 7 in ws[gk] then exec_$mark(1) else exec_$mark(0);
end;

procedure bx_bits_$lsets(k: integer); options(val_param);
begin
  gk := 1;
  ls[gk] := [k];
  if 31 in ls[gk] then exec_$mark(1) else exec_$mark(0);
  if 15 in ls[gk] then exec_$mark(1) else exec_$mark(0);
end;
