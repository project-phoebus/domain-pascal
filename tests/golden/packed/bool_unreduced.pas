{ Golden: codegen-patterns 5.3 "A boolean constant" (bead
  domain-pascal-6zdt): a boolean field of a packed record set or cleared
  with `bset'/`bclr #k' on its byte.  On an absolute or an indexed
  operand k is the bit's number in the word whose byte the field's is,
  unreduced (8 more in the word's high byte, the byte at an even
  displacement or address), as a word set's member (5.1, bead
  domain-pascal-fozq); through an address register the bit's number in
  its byte.  sau2 and sau5 image:
  - entry_clear, entry_set: VTOCE_$TRUNCATE 0xE3A07A `bclr
    #15,(0xa,A2,D0.l)' and 0xE3A0AE `bclr #15,(6,A2,D0.l)' (in_use, bit 7
    of byte 2 of a longword packed record, ../source vtoc/vtoc.ins.pas
    vtoce_$hdr_t), an indexed operand; set, the same number (no site);
  - ptr_clear: through a pointer the bit's in its byte (0xE00E02 `bclr
    #6,(-128,A2)'; PROC2_$INIT 0xE30560 `bclr #5,(42,A2)', 0xE30566 `bset
    #7,(42,A2)');
  - abs_bits: another unit's record, absolute operands (no packed
    boolean site; word sets, sau5 MT_$WAIT 0xE76A1A `bclr
    #13,(0xe90cc0).l'): byte 0 and byte 2 of the longword take 8 more
    (byte 0: the word's number, not the longword's, not seen), byte 3
    the bit's own number;
  - entry_low: a boolean in the low byte (byte 3) on an indexed operand
    keeps the bit's number in its byte, `bclr #1' on the byte.  Best
    effort (not byte-identical): VTOCE_$WRITE 0xE39786 `andi.w
    #-3,(0xa,A2,D0.l)' and RINGLOG_$CNTL 0xE722A6 `andi.w
    #-5,(0xa,A0,D1.l)' clear such a boolean word-wide on the even word. }
module bool_unreduced;
type
  hdr_t = packed record
    top: boolean;                    { byte 0, bit 7 }
    b6: boolean;                     { byte 0, bit 6 }
    pad0: 0..63;
    flags: 0..255;                   { byte 1 }
    in_use: boolean;                 { byte 2, bit 7 }
    mode: 0..3;
    pad2: 0..31;
    pad3: 0..63;
    read_only: boolean;              { byte 3, bit 1 }
    b0: boolean;                     { byte 3, bit 0 }
  end;
  hdr_ptr = ^hdr_t;
  entry_t = record
    uid: array[1..2] of integer32;   { 0x00 }
    hdr: hdr_t;                      { 0x08 }
    rest: array[1..4] of integer32;  { 0x0c }
  end;

var
  x_$hdr: extern hdr_t;
  entries: array[0..3] of entry_t;

procedure entry_clear(i: integer); options(val_param);
begin
  entries[i].hdr.in_use := false;
end;

procedure entry_set(i: integer); options(val_param);
begin
  entries[i].hdr.in_use := true;
end;

procedure entry_low(i: integer); options(val_param);
begin
  entries[i].hdr.read_only := false;
end;

procedure ptr_clear(in p: hdr_ptr);
begin
  p^.in_use := false;
  p^.top := true;
  p^.read_only := true;
end;

procedure abs_bits;
begin
  x_$hdr.in_use := true;
  x_$hdr.top := false;
  x_$hdr.b6 := true;
  x_$hdr.read_only := false;
  x_$hdr.b0 := true;
end;
