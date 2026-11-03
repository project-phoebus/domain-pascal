{ Execution test: best-effort paths (bead domain-pascal-5kgg) of
  comparisons, entry loads, parameter copies and register-held locals:
  a variable in memory compared with a computed value (codegen-patterns
  7.1; the left one loaded, then compared with the computed right one);
  a pointer parameter compared with `nil' and dereferenced (loaded at
  entry into an address register; 12.2, bead domain-pascal-nys); a
  by-value parameter assigned only other values (held in its register,
  bead domain-pascal-r0ag); `addr' of a by-address parameter stored
  besides one dereference (loaded at entry, bead domain-pascal-m2w3); a
  value parameter passed by address to a nested routine and one never
  used (copied; codegen-patterns 1.2); copies of 3 and 6 characters (a
  `move.b' `dbf' loop; 5.2 "Arrays of char"); a negative constant added
  to a register-held local; 65535 into a register-held `0..65535' local.

  call: BP_$CMPMEM w:3 w:2 -> w
  call: BP_$CMPMEM w:2 w:2 -> w
  call: BP_$CMPMEM w:-9 w:-11 -> w
  call: BP_$NYS_T w:41 -> w
  call: BP_$R0AG w:1 w:5 -> w
  call: BP_$R0AG w:1 w:2 -> w
  call: BP_$M2W3_T w:6 -> l
  call: BP_$NEST @l:30 -> l
  call: BP_$UNUSED_T -> l
  call: BP_$CHARS_T
  call: BP_$NEG w:7 -> w
  call: BP_$NEG w:0 -> w
  call: BP_$NEG w:9 -> w
  call: BP_$CWEB w:0 -> uw
  call: BP_$CWEB w:1 -> uw
  warns: best effort (not byte-identical): comparing a variable in memory with a computed value
  warns: best effort (not byte-identical): a pointer parameter compared with `nil` or 0 and dereferenced
  warns: best effort (not byte-identical): assigning to a by-value parameter loaded at entry none of whose assignments
  warns: best effort (not byte-identical): `addr` of a parameter passed by address stored
  warns: best effort (not byte-identical): a value parameter passed by address to a nested routine
  warns: best effort (not byte-identical): a value parameter passed by address that the routine never uses
  warns: best effort (not byte-identical): a copy of an array of 6 char
  warns: best effort (not byte-identical): a copy of an array of 3 char
  warns: best effort (not byte-identical): adding or subtracting a negative constant
  warns: best effort (not byte-identical): a constant outside -128..127 that `moveq` could load
  expect:
    => 0
    => 1
    => 0
    => 42
    => 12
    => 3
    6
    => 7
    => 35
    => 9
    97
    102
    120
    122
    => 6
    => 0
    => 6
    => 65535
    => 3
}
module be_params;
%include 'exec.ins.pas';

type
  wp = ^integer;
  wpp = ^wp;
  c3 = array[1..3] of char;
  c6 = array[1..6] of char;
  rec = record a, b: integer32; end;

var
  gw: integer;
  gp: wp;

{ `gw < integer16(b + 1)': gw in memory, b + 1 computed (a word: without
  the transfer both sides are longwords, reference-clarifications 31) }
function bp_$cmpmem(a, b: integer): integer; options(val_param);
begin
  gw := a;
  if gw < integer16(b + 1) then bp_$cmpmem := 1 else bp_$cmpmem := 0;
end;

function bp_$nys(p: wp): integer; options(val_param);
begin
  bp_$nys := 0;
  if p <> nil then bp_$nys := p^ + 1;
end;

function bp_$nys_t(k: integer): integer; options(val_param);
var w, a, b: integer;
begin
  w := k;
  a := bp_$nys(addr(w));
  b := bp_$nys(nil);
  bp_$nys_t := a + b;
end;

function bp_$r0ag(n: integer; m: integer): integer; options(val_param);
begin
  if m > 3 then n := 7;
  bp_$r0ag := n + m;
end;

{ `addr(w)' stored and `w' read once besides }
function bp_$m2w3(var w: integer): integer32;
begin
  gp := addr(w);
  bp_$m2w3 := w;
end;

function bp_$m2w3_t(k: integer): integer32; options(val_param);
var w: integer;
begin
  w := k;
  bp_$m2w3_t := bp_$m2w3(w) + 1;
  exec_$log_w(gp^);
end;

function bp_$nest(in k: integer32): integer32;
  function inner(r: rec): integer32;
  begin
    inner := r.a + r.b;
  end;
var x: rec;
begin
  x.a := k;
  x.b := 5;
  bp_$nest := inner(x);
end;

procedure bp_$unused(r: rec; out z: integer32);
begin
  z := 9;
end;

function bp_$unused_t: integer32;
var r: rec; z: integer32;
begin
  r.a := 300;
  r.b := 400;
  bp_$unused(r, z);
  bp_$unused_t := z;
end;

procedure bp_$chars6(out d: c6; in s: c6);
begin
  d := s;
end;

procedure bp_$chars3(out d: c3; in s: c3);
begin
  d := s;
end;

procedure bp_$chars_t;
var s, d: c6; t, u: c3;
begin
  s[1] := 'a'; s[2] := 'b'; s[3] := 'c'; s[4] := 'd'; s[5] := 'e'; s[6] := 'f';
  bp_$chars6(d, s);
  exec_$log_c(d[1]);
  exec_$log_c(d[6]);
  t[1] := 'x'; t[2] := 'y'; t[3] := 'z';
  bp_$chars3(u, t);
  exec_$log_c(u[1]);
  exec_$log_c(u[3]);
end;

{ i := i + (-3) on a register-held local }
function bp_$neg(n: integer): integer; options(val_param);
var i, j: integer;
begin
  i := n;
  j := 0;
  while i > 0 do begin
    j := j + 2;
    i := i + (-3);
  end;
  bp_$neg := j;
end;

function bp_$cweb(n: integer): exec_$uword; options(val_param);
var u: exec_$uword;
begin
  u := 65535;
  if n > 0 then u := 3;
  exec_$clobber;
  bp_$cweb := u;
end;
