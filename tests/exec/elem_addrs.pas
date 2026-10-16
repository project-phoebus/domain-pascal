{ Execution test: addresses of elements of other units' arrays as `with'
  records (codegen-patterns 5.2, "The element's address as a value";
  bead domain-pascal-o1kl): `lea (0,An,Dx),Ar' into the base's own
  register when the record is read twice or more and no call intervenes
  (XPD_$FIND_INDEX 0xE5AF60), into a callee-saved register when it lives
  across a call (MMAP_$AVAIL 0xE0CC80), read and written through,
  passed by address.  Element sizes 12 (0-based) and 56 (1-based, a
  scaling with `neg'), a word and a longword index, an index computed.
  Element k of each array holds a value made from k (elem_bases_data.
  mod.pas), so a wrong index, scale or register reads another value.
  Review of the bead: a negative lower bound (a positive displacement),
  the index's variable changed in the body and by a call in it (the
  address formed once, ref "With", p. 4-211), a longword index by
  address with a 56-byte element, the reads in both arms of an `if', a
  call that clobbers the scratch registers between two reads.
  A record read once and not across a call, an address stored: the
  exec test elem_once (bead domain-pascal-1ibr).

  link: elem_bases_data.mod.pas
  call: EB_$FILL
  call: EA_$SUM w:2 -> l
  call: EA_$SUM w:5 -> l
  call: EA_$BUMP w:3 -> l
  call: EA_$SUM w:3 -> l
  call: EA_$BIG w:4 -> l
  call: EA_$LONG l:1 -> l
  call: EA_$NEXT w:0 -> l
  call: EA_$NEG w:-3 -> l
  call: EA_$NEG w:0 -> l
  call: EA_$NEG w:3 -> l
  call: EA_$MOVED -> l
  call: EA_$MOVED -> l
  call: EA_$KCALL -> l
  call: EA_$VIDX @l:3 -> l
  call: EA_$BRANCH w:2 -> l
  call: EA_$BRANCH w:4 -> l
  call: EA_$NEGCALL w:-2 -> l
  call: EA_$NEGCALL w:2 -> l
  expect:
    => 66
    => 156
    41
    => 65
    => 106
    => 3996
    => 23
    => 23
    => -84
    => 6
    => 96
    => 73
    => 83
    => 104
    => 2997
    => 23
    => 42
    => -36
    => 44
}
module elem_addrs;
%include 'exec.ins.pas';

type
  rec12 = record a, b, c: integer32; end;
  rec56 = record a: integer32; z: array[1..13] of integer32; end;
var
  eb_$recs: extern array[0..5] of rec12;
  eb_$big: extern array[1..4] of rec56;
  eb_$negr: extern array[-3..3] of rec12;
  eb_$k: extern integer;

procedure bump(var x: integer32);
begin
  x := x + 10;
end;

{ read twice: a + b + c }
function ea_$sum(i: integer): integer32;
  options(val_param);
begin
  with eb_$recs[i] do
    ea_$sum := a + b + c;
end;

{ across a call: a := a + 10 through the call, then a is logged and
  b + c returned }
function ea_$bump(i: integer): integer32;
  options(val_param);
begin
  with eb_$recs[i] do begin
    bump(a);
    exec_$logv_l(a);
    ea_$bump := b + c;
  end;
end;

{ a 56-byte element: a + z[13] }
function ea_$big(i: integer): integer32;
  options(val_param);
begin
  with eb_$big[i] do
    ea_$big := a + z[13];
end;

{ a longword index }
function ea_$long(i: integer32): integer32;
  options(val_param);
begin
  with eb_$recs[i] do
    ea_$long := a + b;
end;

{ an index computed }
function ea_$next(i: integer): integer32;
  options(val_param);
begin
  with eb_$recs[i + 1] do
    ea_$next := a + b;
end;

{ a negative lower bound: a + b + c }
function ea_$neg(i: integer): integer32;
  options(val_param);
begin
  with eb_$negr[i] do
    ea_$neg := a + b + c;
end;

{ the index's variable changed in the body: the element is the one of
  its value on entry }
function ea_$moved: integer32;
begin
  with eb_$recs[eb_$k] do begin
    eb_$k := eb_$k + 1;
    ea_$moved := a + b;
  end;
end;

procedure bumpk;
begin
  eb_$k := eb_$k + 1;
end;

{ and by a call in the body }
function ea_$kcall: integer32;
begin
  with eb_$recs[eb_$k] do begin
    bumpk;
    ea_$kcall := a + c;
  end;
end;

{ a longword index by address, a 56-byte element: a + z[13] }
function ea_$vidx(var i: integer32): integer32;
begin
  with eb_$big[i] do
    ea_$vidx := a + z[13];
end;

{ the reads in both arms }
function ea_$branch(i: integer): integer32;
  options(val_param);
begin
  with eb_$recs[i] do
    if a > 30 then ea_$branch := b else ea_$branch := c;
end;

{ a call that clobbers the scratch registers between two reads }
function ea_$negcall(i: integer): integer32;
  options(val_param);
var
  x: integer32;
begin
  with eb_$negr[i] do begin
    x := a;
    exec_$clobber;
    ea_$negcall := x + c;
  end;
end;
