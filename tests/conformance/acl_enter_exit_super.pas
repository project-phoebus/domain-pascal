{ ACL_$ENTER_SUPER and ACL_$EXIT_SUPER, 0xE46F90..0xE46FF8 (0x24 and
  0x40 bytes, then the 4-byte constant cell 0xE46FF4; tools/imgdis.sh
  E46F90 68), a fragment of the ACL_ module's code segment 0xE44C3C.
  Semantics: ../source/domain_os/acl/enter_super.c and exit_super.c (count
  one super-user entry more or less for the current process; leaving
  more often than entering crashes the system).  Data: the ACL_ data
  block, map `D E7CF54 ACL_ size = BFC'; the count array's offset is the
  image's (0xB76).  PROC1_$CURRENT (0xE20608) and CRASH_SYSTEM (0xE1E700)
  are other units'.

  Patterns exercised (docs/codegen-patterns.md):
  - 8    `x[i] := x[i] + 1' on a word of a word array of the unit's
         data indexed by another unit's word, at a displacement beyond a
         byte: `lea (0,A5,D0.w),A0', then `addq.w #1,(0xb76,A0)' in place
         (0xE46FA4..0xE46FA8; `subq.w' at 0xE46FE8; bead
         domain-pascal-9pkq);
  - 5.2  the index read from memory and doubled in place, `move.w
         abs.l,D0; add.w D0,D0'; the same element's address formed again
         for the next statement (0xE46FDC, after the `tst.w');
  - 7.1  `tst.w' of the element, `bne.s' over the call;
  - 1.5  the status constant by address, `pea (d,PC)', not popped;
  - 4    A5 saved with `pea (A5)', loaded, restored.

  Reconstructed (not known from the original): the names (../source's;
  the stand-in `pad1' fills the data block up to the array); the status's
  name (../source acl/acl.ins.pas; the stcode database's text for 0x230003
  is "exit_super called more often than enter_super"); the array's bounds
  (0 based at 0xB76, as tests/conformance/acl_clear_super.pas, which
  starts right after this unit). }
module acl_;
type
  status_$t = integer32;
const
  status_$acl_exit_super_unbalanced = 16#00230003;
var
  pad1: array[1..16#5bb] of integer;      { +0x000 }
  super_count: array[0..64] of integer;   { +0xB76 }
  proc1_$current: extern integer;

procedure crash_system(in status: univ status_$t); extern;

procedure acl_$enter_super;
begin
  super_count[proc1_$current] := super_count[proc1_$current] + 1;
end;

procedure acl_$exit_super;
begin
  if super_count[proc1_$current] = 0 then
    crash_system(status_$acl_exit_super_unbalanced);
  super_count[proc1_$current] := super_count[proc1_$current] - 1;
end;
