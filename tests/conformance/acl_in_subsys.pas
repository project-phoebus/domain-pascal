{ ACL_$IN_SUBSYS, 0xE47098, 0x2C bytes (tools/imgdis.sh E47098 2C), a
  fragment of the ACL_ module's code segment 0xE44C3C.  Semantics:
  ../source/domain_os/acl/in_subsys.c (whether the current process's
  subsystem level is above 0).  Data: the overlay section ACL_$DATA
  (map `D69 E88834 ACL_$DATA size = AD98', a common symbol conform.py
  resolves from the map), the level array at +0xAD08 (../source
  acl/acl.pas); the ACL_ data block, map `D E7CF54 ACL_', loaded and not
  used; PROC1_$CURRENT (0xE20608) is another unit's.

  Patterns exercised (docs/codegen-patterns.md):
  - 5.2  an array of an overlay section of more than 0x8000 bytes at
         0x7C00 or beyond, from the section's start plus 60000
         (`movea.l #ACL_$DATA+60000,A0', `207c 00e9 7294'), its
         displacement less 60000 (-15706 for the 1-based array at
         0xAD08), beyond a byte: `lea (0,A0,D0.w),A1', then
         `tst.w (-15706,A1)' (bead domain-pascal-7pcf);
  - 5.2  the element's address read once, its base in A0: into A1,
         free and never written before (bead domain-pascal-1ibr,
         `tools/regalloc.py elemonce');
  - 3    the comparison as a one-statement boolean function's result,
         `sgt D0' (codegen-patterns 3, the register's other bytes left);
  - 2    `link #-4', the result variable;
  - 4    A5 saved with `pea (A5)', loaded (always mode from ACL_$UP,
         tools/a5survey.py), restored.

  Reconstructed (not known from the original): the names (the C's and
  ../source acl/acl.pas's); the stand-ins `pad_low' and `pad_high' for the
  rest of the section (its size 0xAD98, the map's) and `block' for the
  ACL_ data block (size 0xBFC, the map's), which the routine does not
  read; the array's bounds (1..64, acl/acl.pas's). }
module acl_;
var (acl_$data)
  pad_low: array[1..16#5684] of integer;           { +0x0000 }
  acl_$subsys_level: array[1..64] of integer;      { +0xAD08 }
  pad_high: array[1..4] of integer32;              { +0xAD88 }
var
  block: array[1..16#5FE] of integer;              { the ACL_ block }
  proc1_$current: extern integer;

function acl_$in_subsys: boolean;
begin
  acl_$in_subsys := acl_$subsys_level[proc1_$current] > 0;
end;
