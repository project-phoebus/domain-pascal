{ NAME_$GET_WDIR_UID, NAME_$GET_NDIR_UID, 0xE58960..0xE589BC (0x5C bytes;
  tools/imgdis.sh E58960 5C), a fragment of the NAME module's code
  segment (map `I E58488 NAME').  Semantics: ../source
  domain_os/name/uid_getters.c; data layout: the NAME data area 0xE80264
  (map `D E80264 NAME', 0xB20 bytes) as ../source domain_os/name/name.pas
  and name.ins.pas give it: ndir_uid at +0x3E0 and wdir_uid at +0x950,
  arrays [0..57] of 8-byte UIDs, indexed by PROC1_$AS_ID (map 0xE2060A).
  Patterns exercised (docs/codegen-patterns.md):
  - 4    an exported routine using its unit's data: `pea (A5)', `lea
         abs.l,A5', `movea.l (-4,A6),A5' before unlk;
  - 5.2  an 8-byte copy from an element of the unit's array at a
         displacement beyond a byte into an `out' parameter, the index
         a word loaded from another unit: the index's load, the slot's
         load in its stall, the scaling, `lea (0,A5,D0.w),A0; lea
         (d,A0),A0', then `move.l (A0)+,(A1); move.l (A0)+,(4,A1)' (bead
         domain-pascal-zrug).
  Reconstructed (not known from the original): the names of the data
  (../source's), uid_$t as two integer32 fields (../source uid_t's high
  and low), the bytes around the arrays as stand-in arrays; `out' as the
  mode (written, never read). }
module name;
type
  uid_$t = record
    high: integer32;
    low: integer32;
  end;
var
  proc1_$as_id: extern integer;
  standin1: array[0..247] of integer32;   { +0x000..+0x3DF }
  ndir_uid: array[0..57] of uid_$t;       { +0x3E0 }
  standin2: array[0..231] of integer32;   { +0x5B0..+0x94F }
  wdir_uid: array[0..57] of uid_$t;       { +0x950 }

procedure name_$get_wdir_uid(out uid: uid_$t);
begin
  uid := wdir_uid[proc1_$as_id];
end;

procedure name_$get_ndir_uid(out uid: uid_$t);
begin
  uid := ndir_uid[proc1_$as_id];
end;
