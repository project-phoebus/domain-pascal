{ Golden: codegen-patterns 7.4, `for' indices beyond the loop body's own
  reads (bead domain-pascal-h1io).
  - A longword bound read as its low word, in place (10 of 10 sites: the
    eight DIR_ loops over DIR_$MTTAB's count, DIR_$FIND_UID 0xE4E46A
    `move.w (0x155a,A5),D0; subq.w #1,D0; bmi', the count a longword at
    0x1558; EC2_$REGISTER_EC1 0xE42960 `move.w (0xe9a,A5),D0';
    VTOC_$TRUNCATE 0xE3A12C, a frame longword at -0x34 read at -0x32),
    with a call in the body as at PEB_$LOAD_WCS 0xE3214A.
  - The index passed by address: its register stored to its slot before
    the call's pushes, the slot's address pushed (8 of 8 sites:
    DISK_$UNASSIGN_ALL 0xE6BE28 `move.w D3,(-6,A6); pea (-4,A6); pea
    (-6,A6)'; SMD_$FREE_ASID 0xE75278; ASKNODE_$INTERNET_INFO 0xE64D64).
  - The index read after its loop, left in its register (one past the
    upper bound after normal termination, the value at an `exit'), and
    set up although the body reaches its elements through a stepped
    pointer only; coloured before the count: PROC2_$PGROUP_SET
    0xE41EF2..0xE41F12 `moveq #69,D1; movea.l #abs,A0; moveq #1,D0; addq.l
    #8,A0', the head `tst.w (d,A0); beq.s', `addq.w #1,D0; addq.l #8,A0;
    dbf D1', then `cmpi.w #70,D0; ble.s'; CAL_$DECODE_TIME 0xE6882E.
  - An element at an index read through another element, an ordinary
    run-time index each pass: TTY_$I_SET_DFL_FUNCS 0xE6723A `clr.w D4;
    move.b (24,A0,D3.w),D4; add.w D4,D4; lea (0,A0,D4.w),A2'. }
module fli;
type
  status_$t = record
    all: integer32;
  end;
  pgroup_t = record
    ref_count: integer;
    leader: integer;
    upgid: integer;
    session: integer;
  end;
var
  count: integer32;
  pgroups: extern array[1..70] of pgroup_t;
  chars: array[0..17] of char;
  cls: array[0..127] of integer;
  kinds: array[0..17] of integer;
  found: integer;

procedure ext_$put(in x: integer); extern;
procedure ext_$unassign(in vol: integer; out status: status_$t); extern;
procedure crash_system(in st: status_$t); extern;

procedure fli_$all;
var
  i: integer;
begin
  for i := 1 to count do
    ext_$put(i);
end;

procedure fli_$unassign_all;
var
  status: status_$t;
  vol: integer;
begin
  for vol := 1 to 10 do
    ext_$unassign(vol, status);
end;

procedure fli_$alloc(in st: status_$t);
var
  idx: integer;
begin
  for idx := 1 to 70 do
    if pgroups[idx].ref_count = 0 then
      exit;
  if idx > 70 then begin
    crash_system(st);
    return;
  end;
  found := idx;
end;

procedure fli_$classes;
var
  f: integer;
begin
  for f := 0 to 17 do
    cls[ord(chars[f])] := kinds[f];
end;
