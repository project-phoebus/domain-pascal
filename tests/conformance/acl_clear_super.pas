{ ACL_$CLEAR_SUPER, 0xE46FF8, 0x46 bytes (tools/imgdis.sh E46FF8 46), a
  fragment of the ACL_ module's code segment 0xE44C3C.  Semantics:
  ../source/domain_os/acl/clear_super.c (clear the current process's
  super-mode count; if the locksmith override is on and held by this
  process, leave the ACL exclusion region and turn it off).  Data: the
  ACL_ data block, map `D E7CF54 ACL_ size = BFC'; the offsets are the
  image's (0xB72, 0xB76, 0xBF8).  PROC1_$CURRENT (0xE20608),
  ACL_$EXCLUSION_LOCK (0xE2C014) and ML_$EXCLUSION_STOP (0xE20E7E) are
  other units'.

  Patterns exercised (docs/codegen-patterns.md):
  - 7.1  two variables in memory compared, the left one loaded (`move.w
         PROC1_$CURRENT,D0; cmp.w (0xb72,A5),D0; bne.s'), as the second
         operand of `and then' after a boolean's `tst.b; bpl.s';
  - 5.2  a word array of the unit's data indexed by another unit's word:
         `move.w abs.l,D0; add.w D0,D0; lea (0,A5,D0.w),A0' and the
         displacement 0xB76 beyond -128..127;
  - 1.1, 1.5  another unit's object by address, `move.l #abs,-(SP)';
         no pop after the call, the epilogue follows the next store;
  - 4    A5 saved with `pea (A5)', loaded, restored.

  Reconstructed (not known from the original): the names (the C's; the
  stand-ins `pad1', `pad2' fill the data block up to the fields used);
  the override flag as a boolean (`tst.b; bpl', `clr.b'; the C's
  `locksmith_override < 0' on a signed byte); the count array's bounds
  (0 based, so its offset is the displacement; a 1-based array at 0xB74
  would compile the same) and its 65 elements (to the flag at 0xBF8);
  `and then' (the C's `&&'); `unused_4', a stand-in for the 4 frame bytes
  no instruction addresses (`link #-4', as ../source's TTY_$K_ routines'
  `unused_8', tests/conformance/tty_k_flags.pas).  PROC1_$CURRENT is read
  twice: no common subexpression across the statements, as dpc does. }
module acl_;
type
  ml_$exclusion_t = record
    f1: integer32;
    f2: integer32;
    f3: integer32;
    f4: integer32;
    f5: integer;
  end;
var
  pad1: array[1..16#5b9] of integer;      { +0x000 }
  locksmith_owner_pid: integer;           { +0xB72 }
  pad2: integer;                          { +0xB74 }
  super_count: array[0..64] of integer;   { +0xB76 }
  locksmith_override: boolean;            { +0xBF8 }
  proc1_$current: extern integer;
  acl_$exclusion_lock: extern ml_$exclusion_t;

procedure ml_$exclusion_stop(var excl: ml_$exclusion_t); extern;

procedure acl_$clear_super;
var
  unused_4: integer32;                    { (-4,A6): stand-in }
begin
  super_count[proc1_$current] := 0;
  if locksmith_override and then (proc1_$current = locksmith_owner_pid) then begin
    ml_$exclusion_stop(acl_$exclusion_lock);
    locksmith_override := false;
  end;
end;
