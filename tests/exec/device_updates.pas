{ Execution test (bead domain-pascal-eaw7): in-place updates of
  `[device]' objects (ref 3.15.3 pp. 3-57..3-59; codegen-patterns 5.5):
  the shapes of the sau5 MBUS_ and VME_ routines on PBU_CTL (`[n]' or-ed
  into a word set, `- [n]' and-ed, `& ~16#4000' and `! 16#40' on a
  word, MBUS_$ENABLE_DEV 0xE0AD5E, MBUS_$DISABLE_DEV 0xE0AD76, VME_$INT
  0xE0ABAC..0xE0ABFC), counters `+ 1' and `- 3', set members added and
  removed, and SM_$CB's packed command word (sm_$command 0xE6F1A2: the
  unit's 2-bit field from a variable, the command's byte) with a
  boolean and a wider field.  Every word starts with its neighbouring
  bits set (or clear), so a store that touched them would show; the
  registers here are RAM.

  call: DEVICE_UPDATES_$RUN
  expect:
    mem 12 34 00 21 56 78
    mem 12 34 00 01 56 78
    mem 12 34 80 01 56 78
    mem 12 34 80 00 56 78
    mem bf ff 00 40
    mem 00 01 ff fd
    mem ff f7 00 18
    mem ff ff 00 00
    mem a5 fe 7f ff
    mem a5 fd 80 00
    mem 5a 01 7f fe
    mem 5a 03 7f fe
    mem 00 fc 00 00
}
module device_updates;
%include 'exec.ins.pas';
type
  io_$int_id_t = record vector: integer; end;
  { MBUS_'s PBU_CTL (../source mbus/mbus.ins.pas) }
  ctl_t = [device] record
    word_00: integer;
    int_enable: set of 0..15;
    word_04: integer;
  end;
  { SM_$CB's command word and flags (../source sm/sm.ins.pas) }
  cmd_word_t = packed record
    cmd: 0..255;
    unused_7_2: 0..63;
    unit: 0..3;
  end;
  flags_t = packed record
    wired: boolean;
    count: 0..16#7FFF;
  end;
  cb_t = [device] record
    cmd_word: cmd_word_t;
    flags: flags_t;
  end;
  w2_t = [device] record
    a: integer;
    b: integer;
  end;
  s2_t = [device] record
    a: set of 0..15;
    b: set of 0..15;
  end;

var
  pbu_ctl: ctl_t;
  w2: w2_t;
  s2: s2_t;
  cb: cb_t;

procedure enable_dev(id: io_$int_id_t); options(val_param);
begin
  pbu_ctl.int_enable := pbu_ctl.int_enable + [id.vector];
end;

procedure disable_dev(id: io_$int_id_t); options(val_param);
begin
  pbu_ctl.int_enable := pbu_ctl.int_enable - [id.vector];
end;

procedure command(unit: integer; cmd: integer); options(val_param);
begin
  cb.cmd_word.unit := unit;
  cb.cmd_word.cmd := cmd;
end;

procedure device_updates_$run;
var
  id: io_$int_id_t;
begin
  { the MBUS_ register record: neighbours 16#1234, 16#5678 }
  pbu_ctl.word_00 := 16#1234;
  pbu_ctl.int_enable := [0, 5];
  pbu_ctl.word_04 := 16#5678;
  exec_$log_mem(pbu_ctl, 6);
  id.vector := 5;
  disable_dev(id);
  exec_$log_mem(pbu_ctl, 6);
  id.vector := 15;
  enable_dev(id);
  exec_$log_mem(pbu_ctl, 6);
  id.vector := 0;
  disable_dev(id);
  exec_$log_mem(pbu_ctl, 6);

  { VME_$INT's `& ~16#4000', and `! 16#40' }
  w2.a := -1;
  w2.b := 0;
  w2.a := w2.a & ~16#4000;
  w2.b := w2.b ! 16#40;
  exec_$log_mem(w2, 4);
  { counters }
  w2.a := 0;
  w2.b := 0;
  w2.a := w2.a + 1;
  w2.b := w2.b - 3;
  exec_$log_mem(w2, 4);

  { constant members of device sets: one, then two }
  s2.a := [0..15];
  s2.b := [];
  s2.a := s2.a - [3];
  s2.b := s2.b + [3, 4];
  exec_$log_mem(s2, 4);
  s2.a := s2.a + [3];
  s2.b := s2.b - [3, 4];
  exec_$log_mem(s2, 4);

  { SM_$CB: unit and command with the other bits set, then clear }
  cb.cmd_word.cmd := 16#FF;
  cb.cmd_word.unused_7_2 := 63;
  cb.cmd_word.unit := 3;
  cb.flags.wired := false;
  cb.flags.count := 16#7FFF;
  command(2, 16#A5);
  exec_$log_mem(cb, 4);
  cb.flags.wired := true;
  cb.flags.count := 0;
  cb.cmd_word.unit := 1;
  exec_$log_mem(cb, 4);
  cb.cmd_word.cmd := 0;
  cb.cmd_word.unused_7_2 := 0;
  cb.cmd_word.unit := 0;
  cb.flags.wired := false;
  cb.flags.count := 16#7FFF;
  command(1, 16#5A);
  cb.flags.count := cb.flags.count - 1;
  exec_$log_mem(cb, 4);
  cb.cmd_word.unit := cb.cmd_word.unit + 2;
  exec_$log_mem(cb, 4);
  cb.cmd_word.cmd := 0;
  cb.cmd_word.unit := 0;
  cb.cmd_word.unused_7_2 := 63;
  cb.flags.count := 0;
  exec_$log_mem(cb, 4);
end;
