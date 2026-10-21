{ codegen-patterns 5.3 ("Boolean field, test"; bead domain-pascal-g9kx).
  A boolean in the top bit of a byte at an even offset of its packed
  record, bit 15 of the word there, is tested as the word's sign:

  - not_present: `if not pbu_$mcb.present then', an absolute operand,
    `tst.w (PBU_$MCB).l; bmi' (sau5 MT_$ACQUIRE 0xE75CFE, PBU_$SETDOWN
    0xE5950C, PBU_$SETUP 0xE5976C; pbu_$mcb_t as ../source's
    pbu/pbu.ins.pas declares it);
  - present: the value, `tst.w (PBU_$MCB).l; smi D0' (sau5
    PBU_$UNWIRE_USER_PAGES 0xE59D90);
  - not_wired: another bit of the high byte of another unit's record,
    the word loaded and tested in a register, `move.w (PBU_$MCB).l,D0;
    btst #14,D0; bne' (sau5 PBU_$WIRE_PBU 0xE5A436..0xE5A440);
  - either: `if pbu_$mcb.present or pbu_$mcb.vme_set_up then', `tst.w
    (PBU_$MCB).l; smi D0; move.w (PBU_$MCB).l,D1; btst #12,D1; sne D1;
    or.b D1,D0; bpl' (sau5 PBU_$UNWIRE_USER_PAGES 0xE59D90..0xE59DA6);
  - set_present: a store keeps the byte, `andi.b #127,(PBU_$MCB).l; ...;
    or.b D0,(PBU_$MCB).l' (sau5 PBU_$INIT 0xE5A680..0xE5A692);
  - queued: through an address register, the field at bit 15 of the word
    at 2, `tst.w (2,A0); bmi' (MSG_$$SEND 0xE0DBBA `tst.w (0x2,A3); bmi.w',
    whose value store of the same bit is 0xE0DBAC `andi.b #127,(0x2,A3)');
  - failed: `status_$t' as Apollo's base.ins.pas declares it, `if
    status.fail then', `tst.w (A0); bpl' (vtoc_$search_volumes, AST_'s
    module-local 0xE01BEE, at 0xE01C2C `tst.w (A3); bpl.s');
  - low_flag: the top bit of the low byte stays the byte's sign, `tst.b
    (1,A0); smi D0' (AST_$INVALIDATE 0xE066B2 `tst.b (185,A0); smi D0'). }
module packed_word_sign;
type
  status_$t = packed record case integer of
      1:  (fail:  boolean;
           subsys: 0..127;
           modc: 0..255;
           code: integer);
      2:  (all: integer32);
      end;
  mcb_t = packed record
    present: boolean;
    wired: boolean;
    high_ok: boolean;
    vme_set_up: boolean;
    owners_mapped: boolean;
    high_window: boolean;
    unused_9_0: 0..1023;
    setups: integer;
  end;
  xmit_t = packed record
    count: integer;
    queued: boolean;
    rest: 0..32767;
  end;
  low_t = packed record
    high: 0..255;
    flag: boolean;
    rest: 0..127;
  end;
var
  pbu_$mcb: extern mcb_t;
  mbus_$present: extern boolean;

procedure ext_$fail; extern;

procedure not_present;
begin
  if not pbu_$mcb.present then
    ext_$fail;
end;

function present: boolean;
begin
  present := pbu_$mcb.present;
end;

procedure not_wired;
begin
  if not pbu_$mcb.wired then
    ext_$fail;
end;

procedure either;
begin
  if pbu_$mcb.present or pbu_$mcb.vme_set_up then
    ext_$fail;
end;

procedure set_present;
begin
  pbu_$mcb.present := mbus_$present;
end;

procedure queued(var x: xmit_t);
begin
  if x.queued then
    ext_$fail;
end;

procedure failed(var status: status_$t);
begin
  if status.fail then
    ext_$fail;
end;

function low_flag(var r: low_t): boolean;
begin
  low_flag := r.flag;
end;
