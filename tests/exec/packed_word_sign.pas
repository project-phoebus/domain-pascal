{ Execution test: a boolean in bit 15 of a word of a packed record is
  tested as the word's sign, `tst.w' (codegen-patterns 5.3, "Boolean
  field, test"; bead domain-pascal-g9kx; sau5 MT_$ACQUIRE 0xE75CFE
  `tst.w (0x00e91970).l; bmi', PBU_$UNWIRE_USER_PAGES 0xE59D90 `smi',
  MSG_$$SEND 0xE0DBBA `tst.w (0x2,A3)').  The rest of the word (the low
  byte's sign among it) must not change the result; the top bit of the
  low byte stays the byte's sign.  An absolute (another unit's) record,
  one in the unit's data, one through a pointer, as a branch and as a
  value; bit 14 of the absolute record, loaded and tested in a register
  (sau5 PBU_$WIRE_PBU 0xE5A436 `move.w (0x00e91970).l,D0; btst #14,D0').

  link: packed_word_sign_data.mod.pas
  call: PACKED_WORD_SIGN_$RUN w:0x8000
  call: PACKED_WORD_SIGN_$RUN w:0x7fff
  call: PACKED_WORD_SIGN_$RUN w:0x00ff
  call: PACKED_WORD_SIGN_$RUN w:-1
  call: PACKED_WORD_SIGN_$RUN w:0x4000
  call: PACKED_WORD_SIGN_$STORE
  expect:
    true
    1
    true
    true
    1
    false
    false
    3
    false
    2
    false
    false
    0
    true
    true
    2
    false
    2
    false
    false
    0
    true
    false
    1
    true
    1
    true
    true
    1
    true
    true
    2
    false
    2
    false
    false
    2
    false
    true
    2
    0x00ff1234
    0x80ff1234
    true
}
module packed_word_sign;
%include 'exec.ins.pas';

type
  mcb_t = packed record case integer of
      1:  (present: boolean;
           wired: boolean;
           unused: 0..16383;
           setups: integer);
      2:  (word0: integer;
           word1: integer);
      3:  (all: integer32);
      end;
  mcb_ptr_t = ^mcb_t;
  low_t = packed record case integer of
      1:  (high: 0..255;
           flag: boolean;
           rest: 0..127);
      2:  (word0: integer);
      end;
var
  ext_mcb: extern mcb_t;
  mine: mcb_t;
  low: low_t;
  p: mcb_ptr_t;
  ok: boolean;
  n: integer;

function present_of(q: mcb_ptr_t): boolean; options(val_param);
begin
  present_of := q^.present;
end;

procedure packed_word_sign_$run(v: integer); options(val_param);
begin
  ext_mcb.word0 := v;
  mine.word0 := v;
  low.word0 := v;
  p := addr(mine);
  ok := ext_mcb.present;
  exec_$log_bool(ok);
  n := 0;
  if ext_mcb.present then
    n := 1;
  if not mine.present then
    n := n + 2;
  exec_$logv_w(n);
  ok := mine.present;
  exec_$log_bool(ok);
  ok := present_of(p);
  exec_$log_bool(ok);
  n := 0;
  if p^.present then
    n := 1
  else if not low.flag then
    n := 2;
  exec_$logv_w(n);
  ok := low.flag;
  exec_$log_bool(ok);
  ok := ext_mcb.wired;
  exec_$log_bool(ok);
  n := 0;
  if not ext_mcb.wired then
    n := 1;
  if ext_mcb.present or ext_mcb.wired then
    n := n + 2;
  exec_$logv_w(n);
end;

procedure packed_word_sign_$store;
begin
  ext_mcb.all := 16#80FF1234;
  ok := false;
  ext_mcb.present := ok;
  exec_$log_x(ext_mcb.all);
  ok := true;
  ext_mcb.present := ok;
  exec_$log_x(ext_mcb.all);
  ok := ext_mcb.present;
  exec_$log_bool(ok);
end;
