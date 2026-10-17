{ Golden: codegen-patterns 7.1 ("Two computed values").  Bead
  domain-pascal-o1v0; survey `tools/cmpsurvey.py summary', kinds `two'
  and `kcopy'.

  - Both operands computed: the left one first into a data register, the
    right one into another, `cmp.<s> Dr,Dl', the relation as written
    (MSG_$$SEND 0xE0DAE0..0xE0DAF8 `... add.l D1,D0; clr.l D1; move.w
    (2,A0),D1; addi.l #256,D1; cmp.l D1,D0; ble.s', ../source `data_len +
    hdr_len > drv^.max_data_len + 16#100'; the AST_ module-local 0xE00D46
    at 0xE00E74..0xE00E8E, a sum against a widened word).  In `send' the
    parameters arrive in D0 and D1, so the sum's widenings are not
    interleaved as at 0xE0DAE0 (where they are held in D2 and D6).
  - The same mask on both sides: the constant loaded once and copied,
    `move.l #k,D0; move.l D0,D1; and.l x,D0; and.l y,D1; cmp.l D1,D0'
    (HINT_$ADDU 0xE49C26..0xE49C36, ../source `(target_uid.low & 16#FFFFF)
    = (source_uid.low & 16#FFFFF)'; DIR_$OLD_VALIDATE_ROOT_ENTRY 0xE58154,
    with a frame variable).
  - Two bytes widened to words (FILE_$LOCATEI 0xE606E6..0xE606F4 `clr.w
    D0; clr.w D1; move.b (-39,A6),D0; move.b (0xe173f5).l,D1; cmp.w D1,D0;
    bne.s').
  - A variable held in a data register against a computed value: its
    register the destination (PKT_$DUMP_DATA 0xE12808..0xE12812 `clr.l
    D1; move.w D3,D1; subq.l #1,D1; lsl.l #8,D1; lsl.l #2,D1; cmp.l
    D1,D4; ble.s', ../source `len <= (i - 1) * 1024'; there `len''s
    widening is hoisted out of the loop, here the variable is a long). }
module cmpcomp;
type
  pinteger = 0..65535;
  uid_t = record high: integer32; low: integer32; end;
  drv_t = record a: integer; max_data_len: pinteger; end;
  drvp = ^drv_t;
  byte_t = [byte] 0..255;
  bytes_t = record b0: byte_t; b1: byte_t; end;
var
  r: integer;
  w0, w1, w2: integer32;
  thresh: pinteger;
  dl: uid_t;
  other: bytes_t;

procedure send(data_len: pinteger; hdr_len: integer; drv: drvp); options(val_param);
begin
  if data_len + hdr_len > drv^.max_data_len + 16#100 then r := 2;
end;

procedure sum3;
begin
  if w0 + w1 + w2 < thresh then r := 3;
end;

procedure addu(in t: uid_t; in s: uid_t);
begin
  if (t.low & 16#FFFFF) = (s.low & 16#FFFFF) then return;
  r := 1;
end;

procedure frm(in s: uid_t);
var f: uid_t;
begin
  f := s;
  if (dl.low & 16#FFFFF) <> (f.low & 16#FFFFF) then r := 5;
end;

procedure bytes(in mine: bytes_t);
var copy: bytes_t;
begin
  copy := mine;
  if copy.b1 = other.b1 then r := 6;
end;

procedure dump(len: integer32); options(val_param);
var i: pinteger;
begin
  for i := 1 to 4 do begin
    if len <= (i - 1) * 1024 then
      exit;
    r := i;
  end;
end;
