{ Golden: codegen-patterns 5.1 ("One-byte sets"; bead domain-pascal-tq8w):
  `[byte] set of 0..7' fields (ref 3.15.5 p. 3-61 and the owner's ruling,
  reference-clarifications "A size attribute on a set type"), in the
  records ../source bca1d04b gives ACL_ and APP_ (acl/acl.ins.pas,
  app/app.ins.pas).  Mirrors the SR10.2 image:
  - MEMBER: `k in s' on the byte, `btst #k,(d,An)' (ACL_$MIN_RIGHTS
    0xE46922 `btst.b #4,(-0x14,A6)'; acl_$setids 0xE46B90) and as a value
    `btst; sne' (APP_$RECEIVE 0xE00904 `btst.b #3,(7,A3); sne'); member 7
    the byte's sign, `tst.b; bmi' (the word-set rule, 0xE1B63E);
  - CLEAR: `s := []' as `clr.b' (acl_$convert_image 0xE44EDC `clr.b
    (27,A0)'); `s := [4]' and `s := [0..3]' as `move.b #c' (acl_$def_
    acldata 0xE47916 `move.b #0x10,(0x18,A0)', 0xE47928 `move.b
    #0xf,(0x1b,A0)'); `s := t' as `move.b' (ACL_$COPY 0xE4942A);
  - UPDATE: `s := s + [k]' / `s - [k]' as `bset'/`bclr #k' on the byte
    (acl_$convert_image 0xE44FBE `bset #2,(28,A1)'; acl_$image_internal
    0xE47D40 `bclr #6,(24,A3)'; acl_$get_obj_acl_attrs 0xE46084 `bclr
    #0,(3,A2)');
  - MINR: a word set held in a register, assigned a byte set (`clr.w Dn;
    move.b'), less byte sets in place (`clr.w; move.b; not.w; and.w Dm,Dn')
    and plus one (`clr.w; move.b; or.w'): ACL_$MIN_RIGHTS
    0xE4691C..0xE46956, acl_$eval_acl_entries 0xE462C0..0xE462C6;
  - EMPTY: `s <> []' widened to a word, `clr.w; move.b; tst.w'
    (acl_$prim_create_internal 0xE45270..0xE45278);
  - ARG: a byte set to a word-set value parameter, widened between the
    pushes (0xE4535A..0xE45366 `subq.l #2,SP; move.l (0x12,A6),-(SP);
    clr.w D0; move.b (0x18,A4),D0; move.w D0,-(SP)');
  - NARROW: a word set assigned to a byte set, its low byte: a function
    result `move.b D0' (acl_$expand_default_acl 0xE45A50), a word in
    memory read in place (acl_$find_acl_slot 0xE45F24 `move.b
    (2061,A2),(27,A1)'). }
module byte_sets;
type
  uid_t = record high: integer32; low: integer32 end;
  rights_t = set of 0..7;
  word_t = set of 0..15;
  prot_t = record
    owner: uid_t;                         { 0x00 }
    group: uid_t;                         { 0x08 }
    org: uid_t;                           { 0x10 }
    owner_rights: [byte] set of 0..7;     { 0x18 }
    group_rights: [byte] set of 0..7;     { 0x19 }
    org_rights: [byte] set of 0..7;       { 0x1A }
    world_rights: [byte] set of 0..7;     { 0x1B }
    subsys_rights: [byte] set of 0..7;    { 0x1C }
    reserved_1d: char;
    reserved_1e: char;
    reserved_1f: char;
    owner_ext: array[1..3] of integer32;  { 0x20 }
  end;
  hdr_t = record
    unused_00: array[0..6] of char;
    rh_flags_07: [byte] set of 0..7;      { 0x07 }
    rh_node_08: integer32;
  end;
  hdr_p = ^hdr_t;
  dir_t = record
    pad: array[1..6] of integer;
    world_rights: integer;                { 0x0C }
  end;
var
  dirs: array[0..3] of dir_t;
  gprot: prot_t;
  gx: integer32;
  gb: boolean;

function convert(r: integer32; in u: uid_t): integer; extern;

function mask(r: rights_t; in u: uid_t): integer32; internal;
begin
  mask := u.high;
end;

procedure member(in p: prot_t; h: hdr_p; out n: integer);
begin
  gb := 3 in h^.rh_flags_07;
  n := 0;
  if not (4 in p.owner_rights) then
    n := 1;
  if 7 in p.group_rights then
    n := 2;
end;

procedure clear(out p: prot_t; in q: prot_t);
begin
  p.world_rights := [];
  p.owner_rights := [4];
  p.org_rights := [0..3];
  p.subsys_rights := q.group_rights;
end;

procedure update(in out p: prot_t);
begin
  p.subsys_rights := p.subsys_rights + [2];
  p.owner_rights := p.owner_rights - [6];
  p.world_rights := p.world_rights - [0];
end;

procedure minr(in p: prot_t; out w: word_t);
var
  rights: word_t;
begin
  rights := p.world_rights;
  if not (4 in p.owner_rights) then
    rights := rights - p.owner_rights;
  if not (4 in p.group_rights) then
    rights := rights - p.group_rights;
  if not (4 in p.subsys_rights) then
    rights := rights + p.subsys_rights;
  w := rights;
end;

procedure empty(in p: prot_t; out n: integer);
begin
  n := 0;
  if not (4 in p.world_rights) and then (p.world_rights <> []) then
    n := 1;
end;

procedure arg(in prot: prot_t; in acl_type: uid_t); internal;
begin
  gx := mask(prot.owner_rights, acl_type);
  gx := mask(prot.group_rights, acl_type);
end;

procedure narrow(in u: uid_t; slot: integer);
begin
  gprot.world_rights := rights_t(convert(16#2000000, u));
  gprot.subsys_rights := rights_t(dirs[slot].world_rights);
end;
