{ Golden: codegen-patterns 5.2, "Block addresses", "Elements" (bead
  domain-pascal-dsr5): block copies and record comparisons where one
  block is an array element indexed at run time, in the unit's data area
  or through a pointer held in an address register.
  - mount: the element is the destination (the second address): its
    index first, the source's `lea' in the index's stall (12.4 R11), then
    the element's `lea (d,A5,Dx)' (VTOC_$MOUNT 0xE3860E..0xE3861A `moveq
    #100,D0; lea (76,A4),A0; mulu.w D2,D0; lea (-84,A5,D0.w),A1; moveq
    #24,D1'), the count after the element's `lea'.  Here the index dies
    at the `lea', so the count reuses its D0 and must come after; where
    the index lives on (VTOC_$MOUNT's next statement reads it, so its
    count is in D1), dpc's R11 would hoist the count above the `lea', and
    that is refused (codegen-patterns 5.2, "Elements");
  - dismount: the element is the source (the first address), its index
    of two instructions and its `lea' kept together, then the
    destination (VTOC_$DISMOUNT 0xE387C0..0xE387CC `moveq #100,D0; mulu.w
    D2,D0; lea (-84,A5,D0.w),A0; lea (76,A4),A1; moveq #24,D0');
  - vdm: a frame UID compared with an element's field, the element right
    (VOLX_$DISMOUNT 0xE6B412..0xE6B41E `move.w D3,D0; lea (-8,A6),A0;
    lsl.w #5,D0; lea (-24,A5,D0.w),A1; moveq #1,D0'), then an element's
    field compared with another unit's UID, the element left
    (0xE6B48C..0xE6B49A `move.w D3,D2; lsl.w #5,D2; lea (-16,A5,D2.w),A0;
    movea.l #UID_$NIL,A1; moveq #1,D0');
  - set_wdirus: a held `in' UID compared with an element beyond a byte:
    `lea (0,A5,Dx),Am' with the index, the left `lea (An)', then `lea
    (d,Am),Am' (NAME_$SET_WDIRUS 0xE5868C..0xE58696 `lsl.w #3,D0; lea
    (A2),A1; lea (0,A5,D0.w),A0; moveq #1,D0; lea (2384,A0),A0');
  - fm_write: a held `in' block into an element of an array through a
    held pointer, beyond a byte, a `.l' index (FM_$WRITE 0xE3A556..
    0xE3A564 `move.w D6,D0; lea (A4),A1; mulu.w #336,D0; lea
    (0,A3,D0.l),A0; moveq #31,D0; lea (216,A0),A0'; here a signed
    `integer' index, `muls.w', codegen-patterns 5.2 "Word indices used as
    `.l'").  The registers differ from the image's where its context
    holds others. }
module blkelem;
type
  uid_$t = record high: integer32; low: integer32; end;
  vol_t = record
    hash_size: integer;
    hash_type: integer;
    a: array[1..24] of integer32;
  end;
  lab_t = record
    version: integer;
    pad: array[1..37] of integer;
    hdr: vol_t;                           { 0x4c }
  end;
  labp = ^lab_t;
  volx_t = record
    dev: integer32;
    e_lv_uid: uid_$t;
    pv_uid: uid_$t;
    lv_uid: uid_$t;
    pad: integer32;
  end;
  blk_t = array[1..32] of integer32;
  ent_t = record
    pad: array[1..108] of integer;
    b: blk_t;                             { 0xd8 }
    pad2: array[1..4] of integer;
  end;
  tbl_t = array[0..200] of ent_t;
var
  volx: array[1..6] of volx_t;
  vols: array[1..10] of vol_t;
  mounted: array[1..10] of boolean;
  wdir: array[0..63] of uid_$t;
  uid_$nil: extern uid_$t;
  cur: extern integer;

function get_block(i: integer): labp; extern;
procedure get_uid(out u: uid_$t); extern;

procedure mount(vol_idx: integer; out st: integer32); options(val_param);
var lp: labp;
begin
  lp := get_block(vol_idx);
  vols[vol_idx] := lp^.hdr;
  lp^.version := 0;
end;

procedure dismount(vol_idx: integer; out st: integer32); options(val_param);
var lp: labp;
begin
  lp := get_block(vol_idx);
  lp^.hdr := vols[vol_idx];
  lp^.version := 0;
end;

procedure vdm(vol_idx: integer; out st: integer32); options(val_param);
var lv: uid_$t;
begin
  get_uid(lv);
  if lv <> volx[vol_idx].lv_uid then get_uid(lv);
  if volx[vol_idx].e_lv_uid = uid_$nil then get_uid(lv);
end;

procedure set_wdirus(in u: uid_$t; out st: integer32);
begin
  if u = wdir[cur] then st := 0
  else begin
    wdir[cur].high := u.high;
    st := u.low;
  end;
end;

procedure fm_write(in out t: tbl_t; in b: blk_t; i: integer); options(val_param);
begin
  t[i].b := b;
  t[i].pad[1] := 0;
  t[0].pad[2] := b[2];
end;
