{ codegen-patterns 5.1 ("MC68020: `in' on a set of more than a longword")
  and 5.2 ("Packed arrays of booleans", MC68020); bead domain-pascal-bkjm.
  With `-cpu mathchip' a run-time element of a set larger than a longword
  is tested with one `bftst' whose offset register holds `not k', counted
  back from the byte after the set; a packed array of booleans with the
  element's position from the array's first bit:

  - alloc: a word local in a register against an 8-byte set through a
    pointer, `lea (A1),A0' first, `clr.l D1; move.w D2,D1; not.l D1;
    bftst (8,A0)<D1:1>' (SAU9 MST_$ALLOC_ASID 0x3C44A0EC..0x3C44A0F6);
  - drained: a char from an array, `clr.l D1; move.b (d,An,D0.w),D1;
    not.l D1; bftst (0x32,A5)<D1:1>' (TTY_$I_OUTPUT_BUFFER_DRAINED
    0x3C41E2B8..0x3C41E2C2);
  - hpipc: a word from memory against a set in an array of records
    (MSG_$SET_HPIPC 0x3C46050E..0x3C46051A `clr.l D1; lsl.w #3,D0; move.w
    abs.l,D1; not.l D1; bftst (0x1e0,A5,D0.w)<D1:1>'; dpc folds the scale
    into the operand and keeps the `clr.l' with the element, bead
    domain-pascal-9jvz);
  - init: a constant set in the code, the element guarded, `not.l D4;
    cmpi.l #-96,D4; bcs.s L; bftst (L+12,PC)<D4:1>; beq.s L' (NAME_$INIT
    0x3C437920..0x3C437936, a 12-byte set);
  - count, used: a packed array of booleans, `move.w D2,D0; clr.l D1;
    subq.w #1,D0; move.w D0,D1; bftst (0x18,A5)<D1:1>' (PCHIST_$COUNT
    0x3C41CCFE..0x3C41CD06) and, as a function result, `bftst
    abs.l<D1:1>; sne D0' (ACL_$USED_SUSER 0x3C450994..0x3C4509A2). }
module set_member;
type
  asid_t = 0..63;
  asid_set_t = set of asid_t;
  pasid_set_t = ^asid_set_t;
  line_t = array [0..255] of char;
  rec_t = record
    a: integer32;
    flags: set of asid_t;
  end;
var
  seps: static set of char;
  recs: array [0..9] of rec_t;
  bitmap: packed array [1..64] of boolean;
  proc1_$current: extern integer;
  g: extern integer;
  key: extern asid_t;

function alloc(list: pasid_set_t): integer; options(val_param);
var asid: asid_t;
begin
  for asid := 0 to 57 do
    if not (asid in list^) then begin
      alloc := asid;
      return;
    end;
  alloc := -1;
end;

procedure drained(in line: line_t; n: integer);
begin
  if line[n] in seps then
    g := 0;
end;

procedure hpipc(in idx: integer);
begin
  if key in recs[idx].flags then
    g := 1;
end;

procedure init(c: char); options(val_param);
var k: 0..95;
begin
  k := ord(c);
  if k in [0..10, 20, 30..95] then
    g := 2;
end;

procedure count(in mode: integer);
var pid: integer;
begin
  pid := proc1_$current;
  if mode = 1 then
    if bitmap[pid] then
      g := 0;
end;

function used: boolean;
begin
  used := bitmap[proc1_$current];
end;
