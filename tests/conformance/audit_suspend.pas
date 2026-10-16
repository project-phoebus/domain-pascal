{ AUDIT_$SUSPEND and AUDIT_$RESUME, 0xE70DB6..0xE70DF6 (0x20 bytes each;
  tools/imgdis.sh E70DB6 40), a fragment of the AUDIT module.  Semantics:
  ../source/domain_os/audit/suspend.c and resume.c (raise or lower the
  current process's audit suspension count); layout: audit/audit_internal.h
  (suspend_count[0..63] at +0 of AUDIT_$DATA, indexed with pid - 1).  The
  data block is the map's `D E854D8 AUDIT' (manifest --section-addr);
  neither routine loads A5 (manifest `a5-absolute', codegen-patterns 4).

  Patterns exercised (docs/codegen-patterns.md):
  - 8    `x[i] := x[i] + 1' on a word of a word array of the unit's data
         indexed by another unit's word, at a displacement within a byte:
         a load, `addq.w' in a data register and a store through the same
         operand (0xE70DC8..0xE70DCE `move.w (-2,A0,D0.w),D1; addq.w
         #1,D1; move.w D1,(-2,A0,D0.w)'; `subq.w' at 0xE70DEC; bead
         domain-pascal-9pkq);
  - 4    the unit's data addressed absolutely, `lea <block>.l,A0',
         between the index's load and its scaling (0xE70DC0);
  - 5.2  a 1-based word array, the lower bound in the displacement
         `(-2,A0,D0.w)'; the index read from memory and doubled in place.

  Reconstruction notes: the C's suspend_count[pid - 1] is a 1-based array
  here, as tests/conformance/audit_is_process_audited.pas; AUDIT_$DATA's
  other fields are left out (only the 128 bytes of the array are
  declared). }
module audit;
var
  audit_$suspend_count: array[1..64] of integer;   { 0x00..0x7F }
  proc1_$current: extern integer;

procedure audit_$suspend;
begin
  audit_$suspend_count[proc1_$current] := audit_$suspend_count[proc1_$current] + 1;
end;

procedure audit_$resume;
begin
  audit_$suspend_count[proc1_$current] := audit_$suspend_count[proc1_$current] - 1;
end;
