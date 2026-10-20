{ Execution test (bead domain-pascal-5kgg): best-effort `for' loops over
  arrays of words through stepped pointers (codegen-patterns 7.4): the
  unit's first array from its first byte (`lea (A5),An', bead
  domain-pascal-s9tx), two arrays of the unit's data (a pointer each),
  two arrays through one pointer variable, another unit's array (its
  pointer formed in place) with an array through a pointer (bead
  domain-pascal-883c); and, for comparison, an array at an offset in a
  `var' parameter (no best effort).

  call: EB_$FILL
  call: BX_FORSTEP_$FIRST
  call: BX_FORSTEP_$TWO
  call: BX_FORSTEP_$PTR
  call: BX_FORSTEP_$MIX
  call: BX_FORSTEP_$OFF
  link: elem_bases_data.mod.pas
  warns: best effort (not byte-identical): a `for` loop stepping a pointer from the start of the unit's data area
  warns: best effort (not byte-identical): an element of a stepped array at displacement 0
  warns: best effort (not byte-identical): a `for` loop over two arrays of one element size in the unit's data or the frame
  warns: best effort (not byte-identical): a `for` loop over two arrays through one pointer variable
  warns: best effort (not byte-identical): a `for` loop stepping a pointer formed in place (`movea.l A5|A6|#sym,An`) and another pointer
  expect:
    0
    3
    6
    9
    6
    7
    8
    5
    -872
    -808
    -744
    10
    20
    30
}
module be_x_forstep;
%include 'exec.ins.pas';
type
  pair = record x: array[1..3] of integer; y: array[1..3] of integer; end;
  pairp = ^pair;
  w8 = array[1..8] of integer;
var
  ga: array[0..3] of integer;
  gb: array[0..3] of integer;
  pr: pair;
  pp: pairp;
  dst: w8;
  dp: ^w8;
  eb_$words: extern array[1..8] of integer;

procedure bx_forstep_$first;
var i: integer;
begin
  for i := 0 to 3 do ga[i] := i * 3;
  exec_$log_w(ga[0]);
  exec_$log_w(ga[1]);
  exec_$log_w(ga[2]);
  exec_$log_w(ga[3]);
end;

procedure bx_forstep_$two;
var i: integer;
begin
  for i := 0 to 3 do gb[i] := ga[i] + 3;
  exec_$log_w(gb[1]);
end;

procedure bx_forstep_$ptr;
var i: integer;
begin
  pp := addr(pr);
  pr.y[1] := 7;
  pr.y[2] := 8;
  pr.y[3] := 5;
  for i := 1 to 3 do pp^.x[i] := pp^.y[i];
  exec_$log_w(pr.x[1]);
  exec_$log_w(pr.x[2]);
  exec_$log_w(pr.x[3]);
end;

procedure bx_forstep_$mix;
var i: integer;
begin
  dp := addr(dst);
  for i := 2 to 4 do dp^[i] := eb_$words[i];
  exec_$log_w(dst[2]);
  exec_$log_w(dst[3]);
  exec_$log_w(dst[4]);
end;

procedure off(var r: pair);
var i: integer;
begin
  for i := 1 to 3 do r.y[i] := i * 10;
end;

procedure bx_forstep_$off;
begin
  off(pr);
  exec_$log_w(pr.y[1]);
  exec_$log_w(pr.y[2]);
  exec_$log_w(pr.y[3]);
end;
