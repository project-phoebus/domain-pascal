{ Execution test (bead domain-pascal-pjh6): one bit of a `[device]'
  record set or cleared with `ori'/`andi' of a mask at the unit's width
  (codegen-patterns 5.5; sau5 sm_$wire 0xE6EFEE `ori.b #-0x80,(0x44,A2)',
  OS_$SHUTDOWN 0xE6ED30 `andi.b #-9,(0xff9431).l'): booleans and
  one-bit fields of packed records, at the top, bottom and middle of
  their units, with every other bit of the record set (or clear), so a
  wrong mask or unit shows (a packed record's unused bits, here its
  second byte, stay 0); and the same fields of a record that is not a
  device (`bset'/`bclr').

  call: DEVICE_ONE_BIT_$RUN
  expect:
    mem 7f ff f7 00
    mem ff ff ff 00
    mem 80 00 08 00
    mem 00 00 00 00
    mem ff fe ef 00
    mem 00 01 10 00
    mem 7f f7
}
module device_one_bit;
%include 'exec.ins.pas';
type
  { SM_$CB's flag word: the wired flag its sign bit }
  flags_t = packed record
    wired: boolean;
    count: 0..16#7FFF;
  end;
  { a control byte with bit 3 in the middle (VME_CTL's byte 0x31) }
  ctl_t = packed record
    hi: 0..15;
    run: 0..1;
    lo: 0..7;
  end;
  w_t = packed record
    a: 0..16#7FFF;
    low: boolean;
  end;
  b_t = packed record
    x: 0..7;
    mid: boolean;
    y: 0..15;
  end;
  dev1_t = [device] record
    flags: flags_t;
    ctl: ctl_t;
  end;
  dev2_t = [device] record
    w: w_t;
    b: b_t;
  end;
  plain_t = record
    flags: flags_t;
  end;

var
  d1: dev1_t;
  d2: dev2_t;
  p: plain_t;

procedure device_one_bit_$run;
begin
  { all other bits set: clear the flag bits }
  d1.flags.wired := true;
  d1.flags.count := 16#7FFF;
  d1.ctl.hi := 15;
  d1.ctl.run := 1;
  d1.ctl.lo := 7;
  d1.flags.wired := false;
  d1.ctl.run := 0;
  exec_$log_mem(d1, 4);
  d1.flags.wired := true;
  d1.ctl.run := 1;
  exec_$log_mem(d1, 4);
  { all other bits clear: set them }
  d1.flags.wired := false;
  d1.flags.count := 0;
  d1.ctl.hi := 0;
  d1.ctl.run := 0;
  d1.ctl.lo := 0;
  d1.flags.wired := true;
  d1.ctl.run := 1;
  exec_$log_mem(d1, 4);
  d1.flags.wired := false;
  d1.ctl.run := 0;
  exec_$log_mem(d1, 4);

  { the low bit of a word unit, the middle of a byte }
  d2.w.a := 16#7FFF;
  d2.w.low := true;
  d2.b.x := 7;
  d2.b.mid := true;
  d2.b.y := 15;
  d2.w.low := false;
  d2.b.mid := false;
  exec_$log_mem(d2, 4);
  d2.w.a := 0;
  d2.b.x := 0;
  d2.b.y := 0;
  d2.w.low := true;
  d2.b.mid := true;
  exec_$log_mem(d2, 4);

  { not a device }
  p.flags.count := 16#7FF7;
  p.flags.wired := true;
  p.flags.wired := false;
  exec_$log_mem(p, 2);
end;
