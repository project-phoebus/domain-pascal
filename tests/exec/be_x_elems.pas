{ Execution test (bead domain-pascal-5kgg): best-effort forms of array
  elements (codegen-patterns 5.2, 8): another unit's array in a loop
  (the base loaded in the loop, bead domain-pascal-dw25), an element of
  more than a byte through a pointer loaded from memory (the index scaled
  before the pointer's load, bead domain-pascal-nqg1), two elements of
  other units' arrays in one statement, an index read from the same
  array, an element of another unit's array reached by an earlier
  statement (a base of its own, bead domain-pascal-lnzy), and element
  updates in place (`x[i] := x[i] op c` for c other than 1 and for `&`,
  `!`, of the unit's data, the frame and through a pointer: one
  instruction to the element, bead domain-pascal-9pkq).  Values of
  elem_bases_data.mod.pas: eb_$words[i] = 64i - 1000, eb_$neg[j] =
  3j - 1, eb_$big[i].a = 1000i, eb_$big[i].z[13] = -i.

  call: EB_$FILL
  call: BX_ELEMS_$SUM -> w
  call: BX_ELEMS_$SET
  call: BX_ELEMS_$THROUGH w:3 -> w
  call: BX_ELEMS_$THROUGH w:8 -> w
  call: BX_ELEMS_$SELF w:3 -> w
  call: BX_ELEMS_$KEPT w:2 -> l
  call: BX_ELEMS_$TWO w:5 w:-2
  call: BX_ELEMS_$WORD w:5 -> w
  call: BX_ELEMS_$UPD w:3
  call: BX_ELEMS_$UPDF w:2
  call: BX_ELEMS_$UPDP w:4
  link: elem_bases_data.mod.pas
  warns: best effort (not byte-identical): an element of another unit's array in a loop
  warns: best effort (not byte-identical): an element of more than 1 byte of an array through a pointer loaded from memory
  warns: best effort (not byte-identical): updating an array element other than by adding or subtracting 1
  warns: best effort (not byte-identical): a statement with two elements of other units' arrays
  warns: best effort (not byte-identical): an element of another unit's array whose index is read from the same variable
  expect:
    => -5696
    => -808
    => -488
    => -808
    => 1998
    => -7
    15
    -5
    6
    20
    12
    1007
    -94
    3
}
module be_x_elems;
%include 'exec.ins.pas';

type
  rec12 = record a, b, c: integer32; end;
  rec56 = record a: integer32; z: array[1..13] of integer32; end;
  words8 = array[1..8] of integer;
  words_ptr = ^words8;
var
  eb_$words: extern array[1..8] of integer;
  eb_$big: extern array[1..4] of rec56;
  eb_$neg: extern array[-5..5] of integer;
  wp: words_ptr;
  mine: array[1..8] of integer;
  pa: ^words8;

function bx_elems_$sum: integer;
var i, t: integer;
begin
  t := 0;
  i := 1;
  while i <= 8 do begin
    t := t + eb_$words[i];
    i := i + 1;
  end;
  bx_elems_$sum := t;
end;

procedure bx_elems_$set;
begin
  wp := addr(eb_$words);
end;

function bx_elems_$through(i: integer): integer;
  options(val_param);
begin
  bx_elems_$through := wp^[i];
end;

function bx_elems_$self(i: integer): integer;
  options(val_param);
begin
  bx_elems_$self := eb_$words[(eb_$words[i] + 1000) div 64];
end;

function bx_elems_$kept(i: integer): integer32;
  options(val_param);
var x: integer32;
begin
  x := eb_$big[i].a;
  bx_elems_$kept := x + eb_$big[i].z[13];
end;

procedure bx_elems_$two(i, j: integer);
  options(val_param);
begin
  eb_$words[i] := eb_$neg[j];
end;

function bx_elems_$word(i: integer): integer;
  options(val_param);
begin
  bx_elems_$word := eb_$words[i];
end;

{ the unit's array: + 5, - 7, & 6, ! 16 }
procedure bx_elems_$upd(i: integer);
  options(val_param);
begin
  mine[i] := 10;
  mine[i + 1] := 2;
  mine[i + 2] := 7;
  mine[i + 3] := 4;
  mine[i] := mine[i] + 5;
  mine[i + 1] := mine[i + 1] - 7;
  mine[i + 2] := mine[i + 2] & 6;
  mine[i + 3] := mine[i + 3] ! 16;
  exec_$log_w(mine[i]);
  exec_$log_w(mine[i + 1]);
  exec_$log_w(mine[i + 2]);
  exec_$log_w(mine[i + 3]);
end;

{ a frame array: + 2 }
procedure bx_elems_$updf(i: integer);
  options(val_param);
var t: array[1..4] of integer;
begin
  t[i] := 10;
  exec_$clobber;
  t[i] := t[i] + 2;
  exec_$log_w(t[i]);
end;

{ through a pointer: + 1000, - 100, xor-free `&` }
procedure bx_elems_$updp(i: integer);
  options(val_param);
begin
  pa := addr(mine);
  pa^[i] := 7;
  pa^[i + 1] := 6;
  pa^[i + 2] := 7;
  pa^[i] := pa^[i] + 1000;
  pa^[i + 1] := pa^[i + 1] - 100;
  pa^[i + 2] := pa^[i + 2] & 3;
  exec_$log_w(pa^[i]);
  exec_$log_w(pa^[i + 1]);
  exec_$log_w(pa^[i + 2]);
end;
