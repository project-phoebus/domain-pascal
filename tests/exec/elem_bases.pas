{ Execution test: elements of other units' arrays (codegen-patterns 5.2,
  "Another unit's array": `movea.l #sym,An', the lower bound and field
  offsets in the displacement, the base loaded after the index's value;
  bead domain-pascal-rlnm) and 1-byte elements through a pointer loaded
  from memory ("Arrays through a pointer").  Indices from a by-value
  parameter, from another unit's variable, from a frame local; element
  sizes 2 (1-based), 12, 56 (a scaling with `neg') and 1 (a `char'
  index); loads, stores and an element passed by address.  Review of the
  bead: a negative lower bound (a positive displacement), a longword
  index (`.l'), an index computed (`i + 1'), an element in a condition,
  a byte element by a word index, the same array in two statements with
  a call between (the base loaded again), a store of a computed value.  Element k of
  each array holds a value made from k (elem_bases_data.mod.pas), so a
  wrong index, scale or displacement reads another value.  Not expressed
  (refused): such elements in loops, two in one statement, the same
  array in a later statement with no call between, an index read from
  the same variable, displacements beyond a byte, stores whose value
  needs an address register, a padded call's first argument, and their
  addresses as values.

  link: elem_bases_data.mod.pas
  call: EB_$FILL
  call: EB_$WORD w:1 -> w
  call: EB_$WORD w:8 -> w
  call: EB_$REC_B w:4 -> l
  call: EB_$REC_C_K -> l
  call: EB_$BIG_Z w:3 -> l
  call: EB_$BIG_A w:2 -> l
  call: EB_$BYTE b:5 -> c
  call: EB_$THROUGH b:14 -> c
  call: EB_$SET w:2
  call: EB_$WORD w:2 -> w
  call: EB_$WORD w:3 -> w
  call: EB_$LOG_REC w:5
  call: EB_$NEGEL w:-5 -> w
  call: EB_$NEGEL w:0 -> w
  call: EB_$NEGEL w:5 -> w
  call: EB_$LONG l:7 -> l
  call: EB_$NEXT w:3 -> w
  call: EB_$COND w:1 -> w
  call: EB_$COND w:8 -> w
  call: EB_$BYTE_W w:15 -> c
  call: EB_$TWO w:1 w:8 -> w
  call: EB_$PUT w:4 w:21
  call: EB_$WORD w:4 -> w
  expect:
    => -936
    => -488
    => 42
    => 33
    => -3
    => 2000
    => 95
    => 239
    => -7
    => -808
    51
    52
    53
    => -16
    => -1
    => 14
    => 700000
    => -744
    => 1
    => 2
    => 255
    => -1424
    => 172
}
module elem_bases;
%include 'exec.ins.pas';

type
  rec12 = record a, b, c: integer32; end;
  rec56 = record a: integer32; z: array[1..13] of integer32; end;
  bytes = array[0..15] of char;
  bytes_ptr = ^bytes;
var
  eb_$words: extern array[1..8] of integer;
  eb_$recs: extern array[0..5] of rec12;
  eb_$big: extern array[1..4] of rec56;
  eb_$bytes: extern bytes;
  eb_$ptr: extern bytes_ptr;
  eb_$k: extern integer;
  eb_$neg: extern array[-5..5] of integer;
  eb_$longs: extern array[0..9] of integer32;

function eb_$word(i: integer): integer;
  options(val_param);
begin
  eb_$word := eb_$words[i];
end;

function eb_$rec_b(i: integer): integer32;
  options(val_param);
begin
  eb_$rec_b := eb_$recs[i].b;
end;

function eb_$rec_c_k: integer32;
begin
  eb_$rec_c_k := eb_$recs[eb_$k].c;
end;

function eb_$big_z(i: integer): integer32;
  options(val_param);
begin
  eb_$big_z := eb_$big[i].z[13];
end;

function eb_$big_a(i: integer): integer32;
  options(val_param);
var
  j: integer;
begin
  j := i;
  exec_$clobber;
  eb_$big_a := eb_$big[j].a;
end;

function eb_$byte(c: char): char;
  options(val_param);
begin
  eb_$byte := eb_$bytes[ord(c)];
end;

function eb_$through(c: char): char;
  options(val_param);
begin
  eb_$through := eb_$ptr^[ord(c)];
end;

procedure eb_$set(i: integer);
  options(val_param);
begin
  eb_$words[i] := -7;
end;

procedure eb_$log_rec(i: integer);
  options(val_param);
begin
  exec_$log_l(eb_$recs[i].a);
  exec_$log_l(eb_$recs[i].b);
  exec_$log_l(eb_$recs[i].c);
end;

function eb_$negel(i: integer): integer;
  options(val_param);
begin
  eb_$negel := eb_$neg[i];
end;

function eb_$long(i: integer32): integer32;
  options(val_param);
begin
  eb_$long := eb_$longs[i];
end;

function eb_$next(i: integer): integer;
  options(val_param);
begin
  eb_$next := eb_$words[i + 1];
end;

function eb_$cond(i: integer): integer;
  options(val_param);
begin
  if eb_$words[i] < -500 then eb_$cond := 1
  else eb_$cond := 2;
end;

function eb_$byte_w(i: integer): char;
  options(val_param);
begin
  eb_$byte_w := eb_$bytes[i];
end;

function eb_$two(i, j: integer): integer;
  options(val_param);
var
  x: integer;
begin
  x := eb_$words[i];
  exec_$clobber;
  eb_$two := x + eb_$words[j];
end;

procedure eb_$put(i, j: integer);
  options(val_param);
begin
  eb_$words[i] := (j * 2 + 1) * 4;
end;
