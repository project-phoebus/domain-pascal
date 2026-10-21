{ Golden: in-place updates of `[device]' objects (ref 3.15.3 pp.
  3-57..3-59; codegen-patterns 5.5; bead domain-pascal-eaw7).  The
  statement reads the object, so the read of a read-modify-write
  instruction is not "gratuitous": the forms are those of any object.
  The types are ../source's mbus/mbus.ins.pas and sm/sm.ins.pas
  (reconstructed); SM_$CB is `[device]' here, as its flag stores (`move.b
  #0', sm_$command 0xE6F1B6) suggest.
  - enable_dev, disable_dev: MBUS_$ENABLE_DEV 0xE0AD52 and
    MBUS_$DISABLE_DEV 0xE0AD68 (sau5), `clr.w D0; move.w (8,A6),D1;
    bset D1,D0; or.w D0,(0xff7c02).l' and the same with `not.w D0; and.w
    D0,(0xff7c02).l': byte-identical but for the relocation;
  - vme_int: VME_$INT's updates of PBU_CTL, `andi.w #-0x4001,
    (0xff7c00).l' (0xE0ABCC) and the run-time member removed and added
    again (0xE0ABAC, 0xE0ABFC; the image builds `[mlevel]' once in a
    frame word, range-checked, which dpc does not); `! 16#40' as
    `ori.w' (the form of PARITY_$CHK 0xE0B2C8 on a register that is not
    a device);
  - command: sm_$command 0xE6F1A2..0xE6F1B2, the unit's 2-bit field from
    a variable, `andi.b #-4,(0x31,A0); move.b D1,D3; andi.b #3,D3; or.b
    D3,(0x31,A0)', then the command's byte `move.b D0,(0x30,A0)' (the
    image reaches SM_$CB through A0; here by its absolute address);
  - counters and flags: sm_$check_status's `addq.w #1,(0x82,A2)';
    sm_$wire's flag set in place (0xE6EFEE `ori.b #-0x80,(0x44,A2)'; dpc
    gives a boolean field `bset #7', the original's declaration is not
    known); a constant into the unit field (`andi.b', `ori.b'). }
module device_updates;
type
  io_$int_id_t = record vector: integer; end;
  mbus_$ctl_t = [device] record
    word_00: integer;
    int_enable: set of 0..15;
    word_04: integer;
  end;
  sm_$cmd_word_t = packed record
    cmd: 0..255;
    unused_7_2: 0..63;
    unit: 0..3;
  end;
  sm_$cb_flags_t = packed record
    wired: boolean;
    unused_14_0: 0..16#7FFF;
  end;
  sm_$cb_t = [device] record
    unused_00: array[1..24] of integer;
    cmd_word: sm_$cmd_word_t;             { 0x30 }
    unused_32: array[1..9] of integer;
    flags: sm_$cb_flags_t;                { 0x44 }
    count: integer;                       { 0x46 }
  end;

var
  pbu_ctl: extern mbus_$ctl_t;
  sm_$cb: extern sm_$cb_t;

procedure enable_dev(id: io_$int_id_t); options(val_param);
begin
  pbu_ctl.int_enable := pbu_ctl.int_enable + [id.vector];
end;

procedure disable_dev(id: io_$int_id_t); options(val_param);
begin
  pbu_ctl.int_enable := pbu_ctl.int_enable - [id.vector];
end;

procedure vme_int(mlevel: integer); options(val_param);
begin
  pbu_ctl.int_enable := pbu_ctl.int_enable - [mlevel];
  pbu_ctl.word_00 := pbu_ctl.word_00 & ~16#4000;
  pbu_ctl.word_04 := pbu_ctl.word_04 ! 16#40;
  pbu_ctl.int_enable := pbu_ctl.int_enable + [mlevel];
end;

procedure command(unit: integer; cmd: integer); options(val_param);
begin
  sm_$cb.cmd_word.unit := unit;
  sm_$cb.cmd_word.cmd := cmd;
end;

procedure counters;
begin
  sm_$cb.count := sm_$cb.count + 1;
  sm_$cb.flags.wired := true;
  sm_$cb.cmd_word.unit := 2;
end;
