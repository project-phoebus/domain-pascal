{ Execution test (bead domain-pascal-h4ba): memory forms of the link
  test's long tail (codegen-patterns 5.2, 7.1).

  - Elements of another unit's packed array of booleans (ACL_'s bitmaps,
    ACL_$ALLOC_ASID 0xE73CAE: the base `movea.l #sym,An', then `lea
    (0,An,Do.w),Aa'; best effort), tested, set, cleared and stored from a
    computed value (`if v then bset else bclr', best effort; the image
    keeps the bit number in a frame byte, ACL_$INHERIT_SUBSYS 0xE4914C).
    Element k is bit 7 - (k - 1) mod 8 of byte (k - 1) div 8 (ref
    3.11.4.2, Table 3-5).
  - `=`/`<>` of records of 4, 10, 2 and 12 bytes (a `cmpm' loop of
    longwords, words or bytes; best effort), equal and differing in the
    first or the last unit.
  - The address of an element through a pointer at a displacement beyond
    8 bits (`lea (0,An,Dx),Ar; lea (d,Ar),Ar', best effort) and an
    element beyond 16 bits (`lea (0,An,Dx),Am; adda.l #d,Am',
    SMD_$RESET_UNIT_DISPLAY 0xE6D7B4), the pointer made by an address
    minus a constant (best effort).
  - An address plus a constant, on either side (best effort).
  - A computed word index into arrays larger than 32K bytes (widened by
    its sign: zero-extended for a `0..65535' sum, `ext.l' for an
    `integer' one; best effort), wide_index_data.mod.pas's arrays, whose
    element j holds j + 100000 (228 bytes) and j + 300000 (24 bytes).
  - A `with' record of another unit's array an earlier statement reached
    (the `lea' off the kept base, best effort).

  link: be_h4ba_data.mod.pas wide_index_data.mod.pas
  call: BH_M_$BITS w:1 w:0
  call: BH_M_$BITS w:9 w:1
  call: BH_M_$BITS w:64 w:-1
  call: BH_M_$BITS w:8 w:0
  call: BH_$DUMP
  call: BH_M_$SET w:1
  call: BH_M_$SET w:2
  call: BH_M_$SET w:9
  call: BH_$DUMP
  call: BH_M_$CLR w:9
  call: BH_M_$CLR w:2
  call: BH_$DUMP
  call: BH_M_$CMP4 w:1 w:1
  call: BH_M_$CMP4 w:2 w:-1
  call: BH_M_$CMP4 w:1 w:0
  call: BH_M_$CMP10 w:3 w:3
  call: BH_M_$CMP10 w:5 w:6
  call: BH_M_$CMP10 w:1 w:-1
  call: BH_M_$CMP3 w:1 w:65
  call: BH_M_$CMP3 w:2 w:255
  call: BH_M_$CMP3 w:1 w:0
  call: BH_M_$CMP12 w:1 l:100000
  call: BH_M_$CMP12 w:3 l:-2147483648
  call: BH_M_$CMP12 w:1 l:2147483647
  call: BH_M_$LEA8 w:1 l:-5
  call: BH_M_$LEA8 w:4 l:2147483647
  call: BH_M_$FAR w:0 w:1234
  call: BH_M_$FAR w:3 w:-1
  call: BH_M_$ADDC l:7
  call: WI_$FILL
  call: BH_M_$WIDE w:149 w:-999
  call: BH_M_$WIDE w:0 w:1399
  call: BH_M_$WIDE w:299 w:-1
  call: BH_M_$KEPT w:1
  call: BH_M_$KEPT w:40
  warns: best effort (not byte-identical): an element of a packed array of booleans that is not in the unit's data
  warns: best effort (not byte-identical): storing a value other than the constant `true` or `false`
  warns: best effort (not byte-identical): comparing records of 4 bytes
  warns: best effort (not byte-identical): comparing records of 10 bytes
  warns: best effort (not byte-identical): comparing records of 2 bytes
  warns: best effort (not byte-identical): comparing records of 12 bytes
  warns: best effort (not byte-identical): the address of an element through a pointer at a displacement beyond 8 bits
  warns: best effort (not byte-identical): an address and a constant in a sum or difference
  warns: best effort (not byte-identical): a computed word index
  warns: best effort (not byte-identical): an element of another unit's array that an earlier statement since the last call reached too, at a displacement beyond a byte or as an address
  expect:
    mark 0
    mark 1
    mark 0
    mark 1
    mark 1
    mark 0
    mark 1
    mark 1
    mark 1
    mark 0
    mark 1
    mark 0
    mem 00 00 00 00 00 00 00 01
    mem c0 80 00 00 00 00 00 01
    mem 80 00 00 00 00 00 00 01
    mark 1
    mark 0
    mark 0
    mark 1
    mark 0
    mark 1
    mark 1
    mark 0
    mark 0
    mark 1
    mark 0
    mark 1
    mark 1
    mark 0
    mark 0
    mark 1
    mark 0
    mark 1
    mark 1
    mark 0
    mark 0
    mark 1
    mark 0
    mark 1
    -5
    2147483647
    7
    1234
    7
    65535
    7
    8
    7
    100150
    299000
    100001
    301398
    100300
    299998
    6
    7
    6
    7
}
module be_h4ba_mem;
%include 'exec.ins.pas';

type
  pinteger = 0..65535;
  rec6 = record a, b, c: integer; end;
  r4 = record a, b: integer; end;
  r10 = record a: array[1..5] of integer; end;
  r3 = record c: array[1..2] of char; end;
  r12 = record a: array[1..3] of integer32; end;
  pair = record x, y: integer32; end;
  big = record pad: array[1..100] of integer32; e: array[1..4] of pair; end;
  bigp = ^big;
  lp = ^integer32;
  mem_t = array[0..1023] of array[0..63] of pinteger;
  memp = ^mem_t;
  r228 = record z: integer32; pad: array[1..112] of integer; end;
  r24 = record z: integer32; pad: array[1..10] of integer; end;

var
  bh_$bits: extern packed array[1..64] of boolean;
  bh_$recs: extern array[1..40] of rec6;
  wi_$r228: extern array[1..300] of r228;
  wi_$r24: extern array[-1000..1400] of r24;
  a4, b4: r4;
  a10, b10: r10;
  a3, b3: r3;
  a12, b12: r12;
  bigv: big;
  gp: bigp;
  words: array[0..7] of pinteger;

{ Element i stored from computed values and the constant `true', and
  tested after each store. }
procedure bh_m_$bits(i: integer; v: integer); options(val_param);
begin
  bh_$bits[i] := v <> 0;
  if bh_$bits[i] then exec_$mark(1) else exec_$mark(0);
  bh_$bits[i] := true;
  if bh_$bits[i] then exec_$mark(1) else exec_$mark(0);
  bh_$bits[i] := v < 0;
  if bh_$bits[i] then exec_$mark(1) else exec_$mark(0);
end;

procedure bh_m_$set(i: integer); options(val_param);
begin
  bh_$bits[i] := true;
end;

procedure bh_m_$clr(i: integer); options(val_param);
begin
  bh_$bits[i] := false;
end;

procedure bh_m_$cmp4(k: integer; v: integer); options(val_param);
begin
  a4.a := 1; a4.b := 2;
  b4.a := 1; b4.b := 2;
  if k = 1 then a4.a := v else a4.b := v;
  if a4 = b4 then exec_$mark(1) else exec_$mark(0);
  if a4 <> b4 then exec_$mark(1) else exec_$mark(0);
end;

procedure bh_m_$cmp10(k: integer; v: integer); options(val_param);
begin
  a10.a[1] := 1; a10.a[2] := 2; a10.a[3] := 3; a10.a[4] := 4; a10.a[5] := 5;
  b10.a[1] := 1; b10.a[2] := 2; b10.a[3] := 3; b10.a[4] := 4; b10.a[5] := 5;
  a10.a[k] := v;
  if a10 = b10 then exec_$mark(1) else exec_$mark(0);
  if a10 <> b10 then exec_$mark(1) else exec_$mark(0);
end;

procedure bh_m_$cmp3(k: integer; v: integer); options(val_param);
begin
  a3.c[1] := 'A'; a3.c[2] := chr(0);
  b3.c[1] := 'A'; b3.c[2] := chr(0);
  a3.c[k] := chr(v);
  if a3 = b3 then exec_$mark(1) else exec_$mark(0);
  if a3 <> b3 then exec_$mark(1) else exec_$mark(0);
end;

procedure bh_m_$cmp12(k: integer; v: integer32); options(val_param);
begin
  a12.a[1] := 100000; a12.a[2] := -2147483648; a12.a[3] := 2147483647;
  b12.a[1] := 100000; b12.a[2] := -2147483648; b12.a[3] := 2147483647;
  a12.a[k] := v;
  if a12 = b12 then exec_$mark(1) else exec_$mark(0);
  if a12 <> b12 then exec_$mark(1) else exec_$mark(0);
end;

{ `addr(p^.e[i].y)', e at 400 bytes into the record. }
procedure bh_m_$lea8(i: integer; v: integer32); options(val_param);
var q: lp;
begin
  gp := addr(bigv);
  exec_$clobber;
  q := addr(gp^.e[i].y);
  q^ := v;
  exec_$logv_l(bigv.e[i].y);
end;

{ Row 1023 of a 128-byte-row array through a pointer 130944 bytes before
  `words': element [1023, i] is words[i]. }
procedure bh_m_$far(i: integer; v: integer); options(val_param);
var p: memp;
begin
  words[i] := 7;
  p := univ_ptr(integer32(addr(words)) - 130944);
  exec_$logv_l(p^[1023, i]);
  p^[1023, i] := v;
  exec_$logv_l(words[i]);
end;

{ An address plus a constant: the pointer to pad[3] from pad[1]'s
  address, the constant on either side; minus a constant. }
procedure bh_m_$addc(v: integer32); options(val_param);
var q: lp;
begin
  bigv.pad[3] := v;
  bigv.pad[5] := v + 1;
  q := univ_ptr(integer32(addr(bigv.pad[1])) + 8);
  exec_$logv_l(q^);
  q := univ_ptr(16 + integer32(addr(bigv.pad[1])));
  exec_$logv_l(q^);
  q := univ_ptr(integer32(addr(bigv.pad[7])) - 16);
  exec_$logv_l(q^);
end;

{ Computed word indices into arrays of more than 32K bytes. }
procedure bh_m_$wide(i: pinteger; k: integer); options(val_param);
begin
  exec_$logv_l(wi_$r228[i + 1].z);
  exec_$logv_l(wi_$r24[k - 1].z);
end;

{ Another unit's element reached, then a `with' of it. }
procedure bh_m_$kept(i: integer); options(val_param);
begin
  bh_$recs[i].a := 5;
  with bh_$recs[i] do begin
    b := a + 1;
    c := a + 2;
  end;
  exec_$logv_w(bh_$recs[i].b);
  exec_$logv_w(bh_$recs[i].c);
end;
