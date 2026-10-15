{ Execution test: packed arrays of booleans (ref 3.11.4.2 Table 3-5
  p. 3-47, "Boolean 1 bit; bit aligned"; 3.15.5 p. 3-62, "The order of
  elements within words is from most significant bit to least
  significant bit"; codegen-patterns 5.2, "Packed arrays of booleans";
  bead domain-pascal-51e).  Element k of a 1-based array is bit 7 -
  (k - 1) mod 8 of byte (k - 1) div 8.  Every element of a 64-element
  array is set and cleared alone (each bit of each byte, across the byte
  boundaries), the others staying as they were, and read back through
  conditions over the whole array and a function result.  The index
  forms: an `in' parameter by address (read through its address,
  PCHIST_$UNIX_PROFIL_FORK's form), a by-value (`val_param')
  parameter, a `for' index, a frame local, a module variable.  A
  16-element array from 2 (`subq #2') with an initial value.

  Not expressible (refused by dpc): constant indices, a computed value
  stored into an element, an element as a value other than a one-statement
  function's result (codegen-patterns 5.2).

  call: PACKED_BOOLS_$FILL w:0
  call: PACKED_BOOLS_$FILL w:1
  call: PACKED_BOOLS_$ONE w:1
  call: PACKED_BOOLS_$ONE w:2
  call: PACKED_BOOLS_$ONE w:7
  call: PACKED_BOOLS_$ONE w:8
  call: PACKED_BOOLS_$ONE w:9
  call: PACKED_BOOLS_$ONE w:15
  call: PACKED_BOOLS_$ONE w:16
  call: PACKED_BOOLS_$ONE w:17
  call: PACKED_BOOLS_$ONE w:32
  call: PACKED_BOOLS_$ONE w:33
  call: PACKED_BOOLS_$ONE w:56
  call: PACKED_BOOLS_$ONE w:57
  call: PACKED_BOOLS_$ONE w:63
  call: PACKED_BOOLS_$ONE w:64
  call: PACKED_BOOLS_$GET @w:64 -> b
  call: PACKED_BOOLS_$GET @w:63 -> b
  call: PACKED_BOOLS_$GET @w:1 -> b
  call: PACKED_BOOLS_$ZERO w:1
  call: PACKED_BOOLS_$ZERO w:8
  call: PACKED_BOOLS_$ZERO w:9
  call: PACKED_BOOLS_$ZERO w:24
  call: PACKED_BOOLS_$ZERO w:25
  call: PACKED_BOOLS_$ZERO w:40
  call: PACKED_BOOLS_$ZERO w:64
  call: PACKED_BOOLS_$GET @w:40 -> b
  call: PACKED_BOOLS_$GET @w:64 -> b
  call: PACKED_BOOLS_$SETK @w:64
  call: PACKED_BOOLS_$GET @w:64 -> b
  call: PACKED_BOOLS_$LO2 @w:9
  call: PACKED_BOOLS_$LO2 @w:10
  call: PACKED_BOOLS_$LO2 @w:17
  call: PACKED_BOOLS_$CLR2 w:3
  call: PACKED_BOOLS_$CLR2 w:9
  call: PACKED_BOOLS_$CLR2 w:17
  expect:
    mem 00 00 00 00 00 00 00 00
    mem ff ff ff ff ff ff ff ff
    mem 80 00 00 00 00 00 00 00
    1
    mem 40 00 00 00 00 00 00 00
    2
    mem 02 00 00 00 00 00 00 00
    7
    mem 01 00 00 00 00 00 00 00
    8
    mem 00 80 00 00 00 00 00 00
    9
    mem 00 02 00 00 00 00 00 00
    15
    mem 00 01 00 00 00 00 00 00
    16
    mem 00 00 80 00 00 00 00 00
    17
    mem 00 00 00 01 00 00 00 00
    32
    mem 00 00 00 00 80 00 00 00
    33
    mem 00 00 00 00 00 00 01 00
    56
    mem 00 00 00 00 00 00 00 80
    57
    mem 00 00 00 00 00 00 00 02
    63
    mem 00 00 00 00 00 00 00 01
    64
    => true
    => false
    => false
    mem 7f ff ff ff ff ff ff ff
    1
    mem fe ff ff ff ff ff ff ff
    8
    mem ff 7f ff ff ff ff ff ff
    9
    mem ff ff fe ff ff ff ff ff
    24
    mem ff ff ff 7f ff ff ff ff
    25
    mem ff ff ff ff fe ff ff ff
    40
    mem ff ff ff ff ff ff ff fe
    64
    => true
    => false
    mem ff ff ff ff ff ff ff ff
    => true
    mem 80 00
    mem 81 00
    mark 1
    mem 81 00
    mem 81 80
    mark 1
    mem 81 80
    mem 81 81
    mark 1
    mem 81 81
    mem 80 81
    mem 80 80
}
module packed_bools;
%include 'exec.ins.pas';

var
  bm: packed array[1..64] of boolean;
  b16: packed array[2..17] of boolean := [true, * of false];
  k1: integer;
  n: integer;

procedure packed_bools_$fill(v: integer); options(val_param);
var i: integer;
begin
  if v = 0 then
    for i := 1 to 64 do bm[i] := false
  else
    for i := 1 to 64 do bm[i] := true;
  exec_$log_mem(bm, 8);
end;

{ Only element k set; the sum of the indices of the set elements. }
procedure packed_bools_$one(k: integer); options(val_param);
var i: integer;
begin
  for i := 1 to 64 do bm[i] := false;
  bm[k] := true;
  exec_$log_mem(bm, 8);
  n := 0;
  for i := 1 to 64 do
    if bm[i] then n := n + i;
  exec_$log_w(n);
end;

{ Only element k clear; the sum of the indices of the clear elements. }
procedure packed_bools_$zero(k: integer); options(val_param);
var i: integer;
begin
  for i := 1 to 64 do bm[i] := true;
  bm[k] := false;
  exec_$log_mem(bm, 8);
  n := 0;
  for i := 1 to 64 do
    if not bm[i] then n := n + i;
  exec_$log_w(n);
end;

procedure packed_bools_$setk(in k: integer);
begin
  bm[k] := true;
  exec_$log_mem(bm, 8);
end;

function packed_bools_$get(in k: integer): boolean;
begin
  packed_bools_$get := bm[k];
end;

procedure packed_bools_$lo2(in k: integer);
var j: integer;
begin
  exec_$log_mem(b16, 2);
  j := k;
  exec_$clobber;
  b16[j] := true;
  exec_$log_mem(b16, 2);
  k1 := k;
  if b16[k1] then exec_$mark(1);
  if not b16[k1] then exec_$mark(0);
end;

procedure packed_bools_$clr2(k: integer); options(val_param);
begin
  b16[k] := false;
  exec_$log_mem(b16, 2);
end;
