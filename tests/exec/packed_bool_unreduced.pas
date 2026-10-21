{ Execution test (bead domain-pascal-6zdt; codegen-patterns 5.3, "A
  boolean constant"): boolean fields of a packed record set and cleared
  with `bset'/`bclr #k'.  On an indexed or an absolute operand k is the
  bit's number in the word whose byte the field's is, unreduced
  (VTOCE_$TRUNCATE 0xE3A07A `bclr #15,(0xa,A2,D0.l)'): the CPU takes it
  modulo 8 on the byte, so only the field's bit changes.  Booleans in
  each byte of a longword record: byte 0 bits 7 and 6, byte 2 bit 7,
  byte 3 bits 1 and 0; on an array element (indexed), another unit's
  record (absolute) and through a pointer.  The neighbouring words are
  set so that a wrong byte or bit shows.

  link: packed_bool_unreduced_data.mod.pas
  call: PBU_$RUN w:2
  expect:
    mem 5a 5a ff ff ff ff a5 a5
    mem 5a 5a 3f ff 7f fc a5 a5
    mem 5a 5a c0 00 80 03 a5 a5
    mem 5a 5a ff ff ff ff a5 a5
    mem 5a 5a 3f ff 7f fc a5 a5
    mem 5a 5a c0 00 80 03 a5 a5
    mem 5a 5a 3f ff 7f fc a5 a5
}
module pbu;
%include 'exec.ins.pas';

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
  wrap_t = record
    lo: integer;
    hdr: hdr_t;
    hi: integer;
  end;
  all_t = record
    case integer of
      1: (w: wrap_t);
      2: (lo: integer; all: integer32; hi: integer);
  end;

var
  slots: array[1..4] of all_t;
  pbu_$wrap: extern wrap_t;

procedure clear_all(var h: hdr_t);
begin
  h.top := false;
  h.b6 := false;
  h.in_use := false;
  h.read_only := false;
  h.b0 := false;
end;

procedure set_all(var h: hdr_t);
begin
  h.top := true;
  h.b6 := true;
  h.in_use := true;
  h.read_only := true;
  h.b0 := true;
end;

procedure pbu_$run(j: integer); options(val_param);
var
  p: hdr_ptr;
begin
  { an array element: indexed operands }
  slots[j].lo := 16#5A5A;
  slots[j].hi := -16#5A5B;
  slots[j].all := -1;
  exec_$log_mem(slots[j], 8);
  slots[j].w.hdr.top := false;
  slots[j].w.hdr.b6 := false;
  slots[j].w.hdr.in_use := false;
  slots[j].w.hdr.read_only := false;
  slots[j].w.hdr.b0 := false;
  exec_$log_mem(slots[j], 8);
  slots[j].all := 0;
  slots[j].w.hdr.top := true;
  slots[j].w.hdr.b6 := true;
  slots[j].w.hdr.in_use := true;
  slots[j].w.hdr.read_only := true;
  slots[j].w.hdr.b0 := true;
  exec_$log_mem(slots[j], 8);

  { another unit's record: absolute operands }
  pbu_$wrap.lo := 16#5A5A;
  pbu_$wrap.hi := -16#5A5B;
  set_all(pbu_$wrap.hdr);
  pbu_$wrap.hdr.flags := 255;
  pbu_$wrap.hdr.mode := 3;
  pbu_$wrap.hdr.pad0 := 63;
  pbu_$wrap.hdr.pad2 := 31;
  pbu_$wrap.hdr.pad3 := 63;
  exec_$log_mem(pbu_$wrap, 8);
  pbu_$wrap.hdr.top := false;
  pbu_$wrap.hdr.b6 := false;
  pbu_$wrap.hdr.in_use := false;
  pbu_$wrap.hdr.read_only := false;
  pbu_$wrap.hdr.b0 := false;
  exec_$log_mem(pbu_$wrap, 8);
  clear_all(pbu_$wrap.hdr);
  pbu_$wrap.hdr.flags := 0;
  pbu_$wrap.hdr.mode := 0;
  pbu_$wrap.hdr.pad0 := 0;
  pbu_$wrap.hdr.pad2 := 0;
  pbu_$wrap.hdr.pad3 := 0;
  pbu_$wrap.hdr.top := true;
  pbu_$wrap.hdr.b6 := true;
  pbu_$wrap.hdr.in_use := true;
  pbu_$wrap.hdr.read_only := true;
  pbu_$wrap.hdr.b0 := true;
  exec_$log_mem(pbu_$wrap, 8);

  { through a pointer: the bit's number in its byte }
  p := addr(slots[j].w.hdr);
  slots[j].all := -1;
  p^.top := false;
  p^.b6 := false;
  p^.in_use := false;
  p^.read_only := false;
  p^.b0 := false;
  exec_$log_mem(slots[j], 8);
end;
