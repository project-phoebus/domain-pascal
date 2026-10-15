{ codegen-patterns 11 ("Byte-sized module variables"); bead
  domain-pascal-yvko.  Every static object is at least word aligned: a
  char or boolean module variable takes the high byte of a word of its
  own, and two of them are never packed into one word.  The layout is
  SIO_IO's WIRED_DATA 0xE2DEB8 from +0x48 on (`70 00 60 00 000E 000E
  0001 0002 10 00 45 00 2A 00 3A 00 0A 00 07 00 0B 00'; the two
  templates are one-char packed records, a word each, ref 3.10.7
  p. 3-35), read with `move.b (0x4a,A5),(5,A0)' / `move.b (0x48,A5),
  (5,A0)' as in SIO2681_$SET_BREAK 0xE1D13E / 0xE1D148; then booleans
  one to a word, as NETWORK_$ACTIVITY_FLAG 0xE24C42 .. NETWORK_$REALLY_
  DISKLESS 0xE24C4A, tested with `tst.b'. }
module bytes, wired_proc, wired_data;
type
  rates_t = record tx, rx: integer end;
  mr_t = packed record
    case integer of
      1: (c: char);
      2: (mode: 0..3; tx_rts: boolean; cts_tx: boolean; stop: 0..15);
  end;
  regs_t = record mr, sr, csr, cr, rhr, cra: char end;
  regs_ptr_t = ^regs_t;
var
  spin_lock: integer32;
  reserved_04: integer32;
  error_table: array[0..15] of integer32;
  cmd_break_stop: char := chr(16#70);
  cmd_break_start: char := chr(16#60);
  default_baud: rates_t := [tx := 14, rx := 14];
  baud_masks: array[boolean] of integer := [1, 2];
  cmd_reset_mr_ptr: char := chr(16#10);
  cmd_reset_err_enable: char := chr(16#45);
  cmd_reset_rx: char := chr(16#2A);
  cmd_reset_tx: char := chr(16#3A);
  cmd_disable_rx_tx: char := chr(16#0A);
  mr2_template: mr_t := [c := chr(16#07)];
  mr1_template: mr_t := [c := chr(16#0B)];
  activity_flag: define boolean;
  loopback_flag: define boolean;
  do_chksum: define boolean := true;

procedure set_break(regs: regs_ptr_t; enable: boolean);
  options(val_param);
begin
  if enable then
    regs^.cra := cmd_break_start
  else
    regs^.cra := cmd_break_stop;
end;

function busy: boolean;
begin
  busy := activity_flag or loopback_flag;
end;
