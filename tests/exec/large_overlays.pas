{ Execution test: an overlay section of more than 0x8000 bytes (ref
  3.14, p. 3-52; 7.2.4, Method 4, p. 7-7), shared with
  large_overlays_peer.mod.pas.  Its arrays are reached from the section's
  start at offset 0 and from the start plus 60000 at 0x7C00 and beyond,
  the displacements less 60000 (codegen-patterns 5.2, "Overlay sections
  of more than 0x8000 bytes": ACL_$DATA, FILE_$LOT_DATA); the peer reads
  and writes the same words absolutely, so the two addressings must name
  the same memory.  This unit: an element of the array at the start
  (`movea.l #sym,An'), `for' loops over the arrays at 0x7C00 and 0x8480
  (a pointer stepped from `movea.l #sym+60000,An'), words beyond 0x8000
  passed by address (`pea (sym+d).l').

  link: large_overlays_peer.mod.pas
  call: LARGE_OVERLAYS_$RUN w:3
  expect:
    3
    5
    5
    6
    6
    -1
    -2
    -1
    -2
}
module large_overlays;
%include 'exec.ins.pas';

var (lov_$data)
  first: array[1..16#3E00] of integer;     { +0x0000 }
  table: array[1..64] of integer;          { +0x7C00 }
  big: array[1..16#400] of integer;        { +0x7C80 }
  last: array[1..64] of integer;           { +0x8480 }

procedure lov_$show; extern;
procedure lov_$set; extern;

procedure large_overlays_$run(in k: integer);
options(val_param);
var
  i: integer;
  n: integer;
begin
  first[k] := k;
  n := 5;
  for i := 1 to 64 do
    table[i] := n;
  n := 6;
  for i := 1 to 64 do
    last[i] := n;
  lov_$show;
  lov_$set;
  exec_$log_w(big[16#400]);
  exec_$log_w(last[64]);
end;
