{ Golden: one-byte integers, `[byte]' subranges (ref 3.15.5, pp. 3-60..
  3-62: "[byte] 0..255 corresponds to the unsigned char data type in
  Domain/C"; Table 3-7 p. 3-61).  Modelled on TPAD_$DATA 0xE691BC, whose
  packet bytes `../source' declares as `tpad_$byte_t = [byte] 0..255'
  and `tpad_$signed_byte_t = [byte] -128..127' (tpad/tpad.ins.pas; the
  Pascal types are an inference):
  - an unsigned byte against an integer constant, widened to a word first:
    0xE691D8..0xE691E0 `clr.w D0; move.b (10,A1),D0; cmpi.w #0xdf,D0';
  - `&' with a constant of 0..255: the constant loaded as a word, `and.b'
    of the byte (0xE691EE `moveq #0x70,D0; and.b (11,A1),D0; lsr.w #4,D0';
    0xE693EC `move.w #240,D3; ... and.b (12,A1),D3');
  - a type transfer to the signed byte type (p. 4-197) sign-extended for
    a word product: 0xE69228..0xE69232 `move.b (12,A1),D1; ext.w D1;
    move.w D1,D5; muls.w (-38,A2),D5';
  - `lshft' of a byte and a sum with a byte: 0xE6936A..0xE69378 `clr.w
    D2; clr.w D3; move.b (13,A1),D2; move.b (12,A1),D3; lsl.w #6,D2;
    add.w D3,D2' (the `clr.w's scheduled together, codegen-patterns 12.4);
  - byte stores, a byte argument by value in the high byte of a word slot
    (codegen-patterns 1.3), a byte `case' selector (7.3) and index (5.2)
    zero-extended. }
module byte_ints;
type
  ubyte = [byte] 0..255;
  sbyte = [byte] -128..127;
  packet_t = record
    elapsed: integer32;
    clock: integer32;
    w: integer;
    dev_id: ubyte;
    flags: ubyte;
    b0: ubyte;
    b1: ubyte;
    b2: ubyte;
    b3: ubyte;
  end;
var
  bits: integer;
  scale: integer;
  raw: integer;
  last: ubyte;
  table: array[0..255] of integer;
  delta: integer;

procedure take_byte(b: ubyte; s: sbyte); options(val_param); extern;

procedure decode(in out sample: packet_t);
begin
  if sample.dev_id = 16#DF then
    bits := rshft(sample.flags & 16#70, 4);
  delta := sbyte(sample.b0) * scale;
  raw := lshft(sample.b1, 6) + sample.b0;
  bits := sample.b2 & 16#F0;
  last := sample.b3;
  sample.b0 := 0;
  sample.b2 := 200;
  take_byte(sample.b2, sbyte(sample.b3));
  delta := table[sample.b0];
  case sample.flags of
    1: scale := 2;
    2: scale := 4;
    otherwise scale := 1;
  end;
end;
