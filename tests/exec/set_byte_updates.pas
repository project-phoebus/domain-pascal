{ Execution test (beads domain-pascal-wrzd, domain-pascal-fozq;
  codegen-patterns 5.1): a word set's constant members added or removed
  in place.  Members all in the high byte: `ori.b'/`andi.b' of that
  byte (sau5 mt_$queue_request 0xE759CE `ori.b #-0x60,(0x4,A3)'),
  through a pointer and on an absolute operand; members in the low byte
  or in both bytes: word-wide; one member on an absolute or an indexed
  operand: `bset'/`bclr' with the member number unreduced (sau5 MT_$WAIT
  0xE76A1A `bclr #13,(0xe90cc0).l'; sau2 MMAP_$FREE_WSL 0xE0D1B2 `bclr
  #15,(0x2c,A5,D0.w)').  The neighbouring words are set so that a wrong
  byte or width shows.

  link: set_byte_updates_data.mod.pas
  call: SBU_$RUN w:3
  expect:
    mem 5a 5a a0 00 a5 a5
    mem 5a 5a c7 ff a5 a5
    mem 5a 5a c7 1f a5 a5
    mem 5a 5a 01 80 a5 a5
    mem 5a 5a 9d f7 a5 a5
    mem 5a 5a 62 08 a5 a5
    mem 7f ff 5a 5a
    mem 04 00 5a 5a
}
module sbu;
%include 'exec.ins.pas';

type
  w_t = record
    lo: integer;
    s: set of 0..15;
    hi: integer;
  end;
  w_ptr = ^w_t;
  scb_t = record
    lo: integer;
    flags: set of 0..15;
    hi: integer;
  end;
  slot_t = record
    flags: set of 0..15;
    pad: integer;
  end;

var
  w: w_t;
  slots: array[1..4] of slot_t;
  sbu_$scb: extern scb_t;

procedure through(in pw: w_ptr);
begin
  pw^.lo := 16#5A5A;
  pw^.hi := -16#5A5B;
  pw^.s := [];
  pw^.s := pw^.s + [15, 13];
  exec_$log_mem(pw^, 6);
  pw^.s := [0..15];
  pw^.s := pw^.s - [13, 12, 11];
  exec_$log_mem(pw^, 6);
  pw^.s := pw^.s - [5, 6, 7];
  exec_$log_mem(pw^, 6);
  pw^.s := [];
  pw^.s := pw^.s + [7, 8];
  exec_$log_mem(pw^, 6);
end;

procedure sbu_$run(j: integer); options(val_param);
begin
  through(addr(w));

  { another unit's set: absolute operands }
  sbu_$scb.lo := 16#5A5A;
  sbu_$scb.hi := -16#5A5B;
  sbu_$scb.flags := [0..15];
  sbu_$scb.flags := sbu_$scb.flags - [13];
  sbu_$scb.flags := sbu_$scb.flags - [3];
  sbu_$scb.flags := sbu_$scb.flags - [14, 9];
  exec_$log_mem(sbu_$scb, 6);
  sbu_$scb.flags := [];
  sbu_$scb.flags := sbu_$scb.flags + [14, 13];
  sbu_$scb.flags := sbu_$scb.flags + [3];
  sbu_$scb.flags := sbu_$scb.flags + [9];
  exec_$log_mem(sbu_$scb, 6);

  { an array element: indexed operands }
  slots[j].flags := [0..15];
  slots[j].pad := 16#5A5A;
  slots[j].flags := slots[j].flags - [15];
  exec_$log_mem(slots[j], 4);
  slots[j].flags := [];
  slots[j].flags := slots[j].flags + [10];
  exec_$log_mem(slots[j], 4);
end;
