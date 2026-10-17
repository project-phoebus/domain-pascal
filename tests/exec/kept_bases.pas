{ Execution test: the base of another unit's array kept for a later
  statement (codegen-patterns 5.2, "Base kept across statements"; bead
  domain-pascal-lnzy): after an element operand `(d,An,Dx)' of an earlier
  statement, no call since and the base's load dominating, the later
  statement's element of the same array reads the same register (module-
  local 0xE5B704 at 0xE5B756; KBD_$GET_DESC 0xE1AA74).  Two statements
  in a row, a store then another store, past an `if' and its join, past
  an `if' that returns, with a scaling with `neg' (×56).  The arrays are
  elem_bases_data.mod.pas's: element k holds a value made from k, so a
  base or an index of the wrong element reads another value.

  link: elem_bases_data.mod.pas
  call: EB_$FILL
  call: KB_$TWO w:1 w:8 -> w
  call: KB_$SET2 w:2 w:3
  call: KB_$WORD w:2 -> w
  call: KB_$WORD w:3 -> w
  call: KB_$COND w:1 -> l
  call: KB_$COND w:3 -> l
  call: KB_$BIG w:2 -> l
  call: KB_$BIG w:4 -> l
  expect:
    => -1424
    => 5
    => 6
    => 14
    => 33
    => -2
    => -4
}
module kept_bases;
%include 'exec.ins.pas';

type
  rec12 = record a, b, c: integer32; end;
  rec56 = record a: integer32; z: array[1..13] of integer32; end;
var
  eb_$words: extern array[1..8] of integer;
  eb_$recs: extern array[0..5] of rec12;
  eb_$big: extern array[1..4] of rec56;

function kb_$two(i, j: integer): integer;
  options(val_param);
var
  x: integer;
begin
  x := eb_$words[i];
  kb_$two := x + eb_$words[j];
end;

function kb_$word(i: integer): integer;
  options(val_param);
begin
  kb_$word := eb_$words[i];
end;

procedure kb_$set2(i, j: integer);
  options(val_param);
begin
  eb_$words[i] := 5;
  eb_$words[j] := 6;
end;

function kb_$cond(i: integer): integer32;
  options(val_param);
var
  x: integer32;
begin
  if eb_$recs[i].a > 20 then x := 1
  else x := 2;
  kb_$cond := x + eb_$recs[i].b;
end;

function kb_$big(i: integer): integer32;
  options(val_param);
begin
  kb_$big := 0;
  if eb_$big[i].a = 0 then return;
  kb_$big := eb_$big[i].z[13];
end;
