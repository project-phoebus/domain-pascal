{ Execution test (bead domain-pascal-h4ba): three more forms of the
  link test's long tail.

  - A longword held in a register narrowed to a byte in an assignment
    (DISK_IO's `req^.daddr.head := daddr', a `[byte] 0..255' field from
    a `0..2147483647' parameter; best effort): the low byte kept (ref
    reference-clarifications question 41's narrowing), the neighbours
    left alone.
  - A byte constant argument with all bits set other than `true':
    `st -(SP)' (tty_$i_store_parity's `chr(255)', 0xE1BD1E), the byte in
    the high half of its slot.
  - A constant index one past an array as the operand of `addr' (a
    C-style end pointer, rem_file/server.pas; best effort): the address
    computed from the index, no element accessed.

  call: BH_X_$NARROW l:258
  call: BH_X_$NARROW l:-1
  call: BH_X_$NARROW l:0
  call: BH_X_$NARROW l:2147483647
  call: BH_X_$FF
  call: BH_X_$PAST
  warns: best effort (not byte-identical): a longword held in a register narrowed to a byte
  warns: best effort (not byte-identical): a constant index out of range in the operand of `addr`
  expect:
    287441476
    1
    287506244
    1
    287440964
    1
    287506244
    1
    255
    254
    8
    16
    -4
}
module be_h4ba_misc;
%include 'exec.ins.pas';
type
  ubyte = [byte] 0..255;
  hdr = record
    case integer of
      1: (all: integer32);
      2: (cyl: 0..65535; head: ubyte; sector: ubyte);
  end;
  lint = 0..2147483647;
var
  h: hdr;
  bytes: array[1..8] of char;
  longs: array[1..4] of integer32;

procedure bh_x_$narrow(d: lint); options(val_param);
var k: integer;
begin
  h.all := 16#11223344;
  k := 0;
  if d <> 1 then
    k := 1;
  h.head := d;
  exec_$logv_l(h.all);
  exec_$logv_w(k);
end;

procedure bh_x_$ff;
begin
  exec_$logv_c(chr(255));
  exec_$logv_c(chr(254));
end;

procedure bh_x_$past;
begin
  exec_$logv_l(integer32(addr(bytes[9])) - integer32(addr(bytes[1])));
  exec_$logv_l(integer32(addr(longs[5])) - integer32(addr(longs[1])));
  exec_$logv_l(integer32(addr(longs[0])) - integer32(addr(longs[1])));
end;
