{ NAME_$GET_ROOT_UID, NAME_$GET_NODE_UID, NAME_$GET_NODE_DATA_UID,
  0xE589BC..0xE58A20 (0x64 bytes; tools/imgdis.sh E589BC 64), a fragment
  of the NAME module's code segment (map `I E58488 NAME').  Semantics:
  ../source domain_os/name/uid_getters.c; data layout: the NAME data area
  0xE80264 (map `D E80264 NAME', NAME_$NODE_DATA_UID at its start) as
  ../source domain_os/name/name_internal.h describes it: node_data_uid at
  +0x00, node_uid at +0x30, root_uid at +0x38, 8 bytes each.

  Patterns exercised (docs/codegen-patterns.md):
  - 4    an exported routine using its unit's data: `pea (A5)', `lea
         abs.l,A5', `movea.l (-4,A6),A5' before unlk;
  - 5.2  an 8-byte record assignment into an `out' parameter from the
         data area: the destination's slot loaded first, into A1
         (`movea.l (8,A6),A1'), the source's address after it into A0
         (`lea (0x38,A5),A0'; `lea (A5),A0' for offset 0 at 0xE58A10),
         then `move.l (A0)+,(A1); move.l (A0)+,(4,A1)' (bead
         domain-pascal-fbg).

  Reconstructed (not known from the original): the names of the data
  (NAME_$NODE_DATA_UID is the map's; the others ../source's), uid_$t as
  two integer32 fields (../source uid_t's high and low), the bytes
  between the UIDs as one stand-in array; `out' as the mode (written,
  never read). }
module name;
type
  uid_$t = record
    high: integer32;
    low: integer32;
  end;
var
  node_data_uid: uid_$t;                  { +0x00 }
  standin: array[0..9] of integer32;      { +0x08..+0x2F }
  node_uid: uid_$t;                       { +0x30 }
  root_uid: uid_$t;                       { +0x38 }

procedure name_$get_root_uid(out uid: uid_$t);
begin
  uid := root_uid;
end;

procedure name_$get_node_uid(out uid: uid_$t);
begin
  uid := node_uid;
end;

procedure name_$get_node_data_uid(out uid: uid_$t);
begin
  uid := node_data_uid;
end;
