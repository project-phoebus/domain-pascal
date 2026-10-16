{ Golden: codegen-patterns 5.2 ("The element's address as a value", read
  once), 12.4 R2.  Bead domain-pascal-1ibr; survey `tools/regalloc.py
  elemonce'.

  An element of another unit's array whose address `lea (d,An,Dx),Am'
  is read once, by the next instruction, in no call's range, the base in
  A0 and read by the `lea' alone:
  - A1 free over the address's range and clean on every path (never
    written, or last written by a call or used up by `(A1)+'): the
    address in A1, 56 of 57 sites.  stash: an address stored (the
    module-local 0xE1C204 at 0xE1C230..0xE1C246 `movea.l
    #FIM_$QUIT_EC,A0; lsl.w #2,D0; ...; lea (0,A0,D0.w),A1; move.l
    A1,(-80,A6)'); note: an element at a displacement beyond a byte,
    `lea (0,A0,Dx),A1' then `(d,A1)' (NETBUF_$RTN_DAT 0xE0F07A..0xE0F086
    `movea.l #MMAP,A0; lsl.l #4,D3; lea (0,A0,D3.l),A1; move.w
    (806,A5),(-8186,A1)'); subsys: an array of an overlay section of
    more than 0x8000 bytes from its base 60000 bytes in (ACL_$IN_SUBSYS
    0xE470AA..0xE470B6 `movea.l #ACL_$DATA+60000,A0; add.w D0,D0; lea
    (0,A0,D0.w),A1; tst.w (-15706,A1)'; tests/conformance/acl_in_subsys.pas);
    after: A1 last written by a call (0xE01244 `lea (0,A0,D5.l),A1').
  - A1 dirty on every path: A0 itself, 6 of 6.  both: the second
    address after the first left one in A1 (SIO_$INIT 0xE32CC4..0xE32CDA
    `lea (-900,A2),A1; move.l A1,(-12,A6); ...; movea.l #0xE2C9F0,A0;
    lea (78,A0,D3.l),A0; move.l A0,(-8,A6)'). }
module elon;
type
  page_t = record
    link: integer32;
    count: integer;
    pad: array[1..4092] of char;
  end;
  ec_t = record
    a, b, c: integer32;
  end;
var (elon_$big)
  low: array[0..16#3DFF] of integer;          { +0x0000 }
  level: array[1..64] of integer;             { +0x7C00 }
  high: array[1..16#200] of integer;          { +0x7C80, to 0x8080 }
var
  pages: extern array[1..8] of page_t;
  ecs: extern array[1..64] of ec_t;
  quits: extern array[1..64] of ec_t;
  as_id: extern integer;
  saved: extern ^ec_t;
  other: extern ^ec_t;
  cnt: integer;

function f(x: integer): integer; options(val_param, extern);

procedure stash; options(val_param);
begin
  saved := addr(ecs[as_id]);
end;

procedure note(in page: integer32); options(val_param);
begin
  pages[page].count := cnt;
end;

function subsys: boolean;
begin
  subsys := level[as_id] > 0;
end;

procedure after;
begin
  as_id := f(as_id);
  saved := addr(ecs[as_id]);
end;

procedure both;
begin
  saved := addr(ecs[as_id]);
  other := addr(quits[cnt]);
end;
