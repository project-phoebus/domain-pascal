{ Execution test (bead domain-pascal-e07e): assignments of records and
  arrays with device or volatile parts (ref 3.15.1-3.15.3, 3.15.7 pp.
  3-76..3-77), in both directions: whole records, sub-records and
  arrays of 2, 6, 8, 10, 12 and 22 bytes, an array of 3 bytes (an odd
  size: the byte loop's best-effort path, as for any array of `[byte]'
  elements), a 4-byte record with a volatile field, and the WIN_
  descriptor's save and restore through a pointer to a `[device]'
  window (the sau5 RESTORE 0xE19A1E, SEEK 0xE19B14: three `move.l' each
  way; codegen-patterns 5.5).  The copies are the ordinary ones (5.2),
  so every byte arrives; the window here is RAM.

  call: DEVICE_COPIES_$RUN
  warns: best effort (not byte-identical): assigning an array of one-byte elements other than char
  expect:
    mem 01 02
    mem 01 02 03 04 05 06
    mem 01 02 03 04 05 06 07 08
    mem 01 02 03 04 05 06 07 08 09 0a
    mem 01 02 03 04 05 06 07 08 09 0a 0b 0c
    mem 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f 10 11 12 13 14 15 16
    mem 01 02 03
    mem 01 02 03 04
    mem 11 12 13 14 15 16 17 18 19 1a 1b 1c
    mem 00 00 00 00 00 00 00 00 00 00 00 00 21 22 23 24 25 26 27 28 29 2a 2b 2c
    mem 21 22 23 24 25 26 27 28 29 2a 2b 2c
    mem 00 05 33 34 35 36 37 38 39 3a 3b 3c
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 41 42 43 44 45 46 47 48 49 4a 4b 4c
}
module device_copies;
%include 'exec.ins.pas';
type
  ub = [byte] 0..255;
  pinteger = 0..65535;
  b2 = array[1..2] of ub;
  b3 = array[1..3] of ub;
  w3 = record a, b, c: integer end;
  l2 = record a, b: integer32 end;
  w5 = array[1..5] of integer;
  { the WIN_ descriptor, 12 bytes (../source win/win.ins.pas) }
  desc_t = record
    cyl: pinteger;
    head: ub;
    sector: ub;
    phys: integer32;
    page: pinteger;
    drive: pinteger;
  end;
  w11 = array[1..11] of integer;
  { a device record with parts of each size }
  dev_t = [device] record
    p2: b2;
    p6: w3;
    p8: l2;
    p10: w5;
    p12: desc_t;
    p22: w11;
    p3: b3;
  end;
  { a volatile field: the record is not volatile (ref 3.15.7), but its
    copy references the field }
  vf_t = record a: [volatile] integer; b: integer end;
  { the window: two descriptors from +0x10 }
  regs_t = [device] record
    go: ub;
    command: ub;
    status: pinteger;
    count: ub;
    error: ub;
    pad: array[1..5] of integer;
    desc: array[1..2] of desc_t;
  end;
  regs_ptr_t = ^regs_t;

var
  dev, dev2: dev_t;
  s2: b2; s6: w3; s8: l2; s10: w5; s12: desc_t; s22: w11; s3: b3;
  vs, vd: vf_t;
  window: regs_t;
  regs: regs_ptr_t;
  saved: desc_t;
  zero: desc_t;

procedure fill(var v: univ exec_$bytes; n: integer; first: integer);
var i: integer;
begin
  for i := 1 to n do v[i] := chr(first + i - 1);
end;

procedure device_copies_$run;
begin
  { into the device record, then out of it into plain variables }
  fill(s2, 2, 1); fill(s6, 6, 1); fill(s8, 8, 1); fill(s10, 10, 1);
  fill(s12, 12, 1); fill(s22, 22, 1); fill(s3, 3, 1);
  dev.p2 := s2; dev.p6 := s6; dev.p8 := s8; dev.p10 := s10;
  dev.p12 := s12; dev.p22 := s22; dev.p3 := s3;
  dev2 := dev;
  fill(s2, 2, 0); fill(s6, 6, 0); fill(s8, 8, 0); fill(s10, 10, 0);
  fill(s12, 12, 0); fill(s22, 22, 0); fill(s3, 3, 0);
  s2 := dev2.p2; s6 := dev2.p6; s8 := dev2.p8; s10 := dev2.p10;
  s12 := dev2.p12; s22 := dev2.p22; s3 := dev2.p3;
  exec_$log_mem(s2, 2);
  exec_$log_mem(s6, 6);
  exec_$log_mem(s8, 8);
  exec_$log_mem(s10, 10);
  exec_$log_mem(s12, 12);
  exec_$log_mem(s22, 22);
  exec_$log_mem(s3, 3);
  fill(vs, 4, 1);
  vd := vs;
  exec_$log_mem(vd, 4);

  { RESTORE's shape through a pointer to the window }
  regs := addr(window);
  fill(window.desc[1], 12, 16#11);
  fill(window.desc[2], 12, 16#21);
  saved := regs^.desc[1];
  exec_$log_mem(saved, 12);
  regs^.desc[1] := zero;
  exec_$log_mem(window.desc, 24);
  regs^.desc[1] := regs^.desc[2];
  exec_$log_mem(regs^.desc[1], 12);
  fill(saved, 12, 16#31);
  regs^.desc[2] := saved;
  regs^.desc[2].cyl := 5;
  exec_$log_mem(regs^.desc[2], 12);
  regs^.desc[1] := zero;
  regs^.desc[2] := zero;
  exec_$log_mem(window.desc, 24);
  fill(saved, 12, 16#41);
  window.desc[1] := saved;
  saved := zero;
  saved := window.desc[1];
  exec_$log_mem(saved, 12);
end;
