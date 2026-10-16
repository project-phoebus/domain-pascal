{ Golden: codegen-patterns 5.2, "Block addresses" (bead domain-pascal-21xu):
  the address registers of block copies (10 bytes and more) and of
  record comparisons, beyond the frame, the data area and a pointer slot:
  - get_info: another unit's record copied into a by-address parameter,
    `movea.l #sym,A0; movea.l (8,A6),A1', the source first, then three
    `move.l (A0)+,(A1)+' (CAL_$GET_INFO 0xE6856C, CAL_$TIMEZONE);
  - get_86: the same with a `dbf' loop and a final word (DISK_$GET_ERROR_
    INFO 0xE6C090..0xE6C0A2, 86 bytes, `moveq #0x14');
  - put_big: a frame record into another unit's, `lea (d,A6),A0; movea.l
    #sym,A1' (0xE336E2..0xE336E8);
  - check: an `in' record held in an address register since the entry,
    compared with another unit's UID, `lea (An),Am' (never `movea.l
    An,Am') then `movea.l #UID_$NIL,Am' (NAME_ 0xE588D8..0xE588DA `lea
    (A1),A2; movea.l #UID_$NIL,A3'; ACL_ 0xE44DD0), and copied from the
    same register (an 8-byte copy, `lea (An),Am' as its source,
    TTY_$K_SET_INPUT_BREAK_MODE 0xE6784E); the pointer is read after
    the comparison (where it dies at the `lea', the original passes its
    register over, 0xE44E0A, and dpc refuses that case: bead
    domain-pascal-76uw);
  - stats: the unit's record into an `out' record held in A0 (read again
    after the copy): `lea (A5),A1; lea (A0),A2', source first
    (DISK_$GET_STATS 0xE3DBBE..0xE3DBC2 `lea (0x180,A5),A1; lea (A2),A3',
    its registers moved on by an earlier statement's A0 and its held A2;
    CAL_$WRITE_TIMEZONE 0xE3E626 `lea (A0),A1; lea (A5),A3');
  - scan: an `in out' record whose address is also data, held in a data
    register, compared as a 6-byte record: `movea.l Dn,A1' (TIME_$Q_SCAN_
    QUEUE 0xE16EBC..0xE16EC4 `lea (-0x10,A6),A0; movea.l D3,A1; moveq
    #0x2,D0; cmpm.w (A1)+,(A0)+; dbne');
  - find: a field of another unit's array element reached through a `for'
    loop's stepped pointer, `lea (-0x48,An),Am' (DISK_$LVUID_TO_VOLX
    0xE6D182 `lea (-0x48,A0),A2; lea (-0x8,A6),A3; moveq #0x1,D2;
    cmpm.l'; the loop's registers differ: codegen-patterns 7.4). }
module blkaddr;
type
  uid_$t = record
    high: integer32;
    low: integer32;
  end;
  tz_t = record
    utc_delta: integer;
    tz_name: array[0..3] of char;
    drift_high: integer32;
    drift_low: integer;
  end;
  info_t = array[1..43] of integer;
  big_t = array[1..131] of integer32;
  stats_t = record
    a: array[1..5] of integer32;
    b: integer;
  end;
  clock_t = record
    high: integer32;
    low: integer;
  end;
  addr_t = 0..16#7fffffff;
  dvte_t = record
    lv_uid: uid_$t;                       { 0x00 }
    lv_start: integer32;                  { 0x08 }
    pad: array[1..10] of integer;         { 0x0c }
    mount_state: integer;                 { 0x20 }
    pad2: array[1..19] of integer;        { 0x22 }
  end;
var
  uid_$nil: extern uid_$t;
  cal_$timezone: extern tz_t;
  disk_$error_info: extern info_t;
  other_big: extern big_t;
  disk_$dvtbl: extern array[1..6] of dvte_t;
  template: stats_t;
  g: addr_t;

procedure sub48(in out a: clock_t; in b: clock_t); extern;

procedure get_info(out tz_rec: tz_t);
begin
  tz_rec := cal_$timezone;
end;

procedure get_86(out buffer: info_t);
begin
  buffer := disk_$error_info;
end;

procedure put_big;
var
  b: big_t;
begin
  b[1] := 0;
  other_big := b;
end;

procedure check(in u: uid_$t; out r: integer; out c: uid_$t);
begin
  c := u;
  r := 0;
  if u = uid_$nil then
    r := 1;
  if u.low = 0 then
    r := 2;
end;

procedure stats(out s: stats_t);
begin
  s := template;
  s.b := 1;
end;

procedure scan(in out now: clock_t; in later: clock_t);
var
  expire: clock_t;
begin
  g := addr_t(addr(now)) - 4;
  expire := later;
  if expire <> now then
    sub48(now, expire);
  sub48(now, later);
end;

function find(lv_uid: uid_$t): integer;
var
  i: integer;
  found: integer;
begin
  found := 0;
  for i := 1 to 6 do
    if (disk_$dvtbl[i].mount_state = 3) and then
       (disk_$dvtbl[i].lv_start <> 0) and then
       (disk_$dvtbl[i].lv_uid = lv_uid) then begin
      found := i;
      exit;
    end;
  find := found;
end;
