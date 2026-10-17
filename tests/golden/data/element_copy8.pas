{ Golden: codegen-patterns 5.2, "6 and 8 bytes", "Elements" (bead
  domain-pascal-zrug): 8-byte copies (UIDs) from an array element indexed
  at run time, `move.l (Am)+,<d>; move.l (Am)+,<d+4>' with Am the
  element's address:
  - get_wdir, get_ndir: an element of the unit's array beyond a byte
    into an `out' parameter, the index loaded from another unit's word:
    the index's load, the slot's load in its stall, the scaling, then
    `lea (0,A5,Dx),A0; lea (d,A0),A0' (NAME_$GET_WDIR_UID
    0xE5896C..0xE58982 `move.w PROC1_$AS_ID,D0; movea.l (8,A6),A1; lsl.w
    #3,D0; lea (0,A5,D0.w),A0; lea (2384,A0),A0; move.l (A0)+,(A1);
    move.l (A0)+,(4,A1)'; NAME_$GET_NDIR_UID 0xE5899A);
  - unlock: the same element into the frame: the element alone (DIR_
    0xE5720E..0xE57222 `move.w PROC1_$CURRENT,D0; lsl.w #3,D0; lea
    (0,A5,D0.w),A0; lea (696,A0),A0; move.l (A0)+,(-8,A6)' ×2; 0xE5473C
    the same, with an entry load in the index's stall);
  - get_info: an element within a byte into an `out' parameter, the
    index held in a register: the index and the `lea' together, then the
    slot (VOLX_$GET_INFO 0xE6B602..0xE6B610 `move.w D2,D0; lsl.w #5,D0;
    lea (-32,A5,D0.w),A0; movea.l (12,A6),A1; move.l (A0)+,(A1); move.l
    (A0)+,(4,A1)');
  - touch: an element of an array through a pointer held in an address
    register, within a byte, into the frame (SMD_ 0xE6E4F4..0xE6E502
    `clr.l D0; move.w D2,D0; lsl.l #3,D0; lea (-8,A2,D0.l),A0; move.l
    (A0)+,(-16,A6)' ×2; here the index a frame word, so loaded).
  The registers differ from the image's where its context holds others. }
module elemcopy;
type
  uid_$t = record high: integer32; low: integer32; end;
  status_$t = record all: integer32; end;
  volx_t = record
    dev: integer32;
    e_lv_uid: uid_$t;
    pv_uid: uid_$t;
    lv_uid: uid_$t;
    pad: integer32;
  end;
  rect_t = record x1, y1, x2, y2: integer; end;
  rects_t = array[1..5000] of rect_t;
  count_t = 0..65535;
var
  volx: array[1..6] of volx_t;
  pad: array[1..41] of integer32;
  lock_uid: array[0..57] of uid_$t;
  ndir_uid: array[0..57] of uid_$t;
  wdir_uid: array[0..57] of uid_$t;
  proc1_$as_id: extern integer;
  proc1_$current: extern integer;

procedure bat(vol_idx: integer; out st: status_$t); options(val_param); extern;
procedure unlock_it(in u: uid_$t); extern;
procedure show(in r: rect_t); extern;

procedure get_wdir(out u: uid_$t);
begin
  u := wdir_uid[proc1_$as_id];
end;

procedure get_ndir(out u: uid_$t);
begin
  u := ndir_uid[proc1_$as_id];
end;

procedure unlock;
var dir_uid: uid_$t;
begin
  dir_uid := lock_uid[proc1_$current];
  unlock_it(dir_uid);
end;

procedure get_info(vol_idx: integer; out u: uid_$t; out st: status_$t);
  options(val_param);
begin
  bat(vol_idx, st);
  if st.all = 0 then
    u := volx[vol_idx].lv_uid;
end;

procedure touch(in rects: rects_t; count: count_t); options(val_param);
var r: rect_t;
begin
  show(rects[1]);
  r := rects[count];
  show(r);
end;
