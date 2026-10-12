{ AUDIT_$IS_PROCESS_AUDITED, 0xE70D94, 0x22 bytes: 0xE70D94..0xE70DB6
  (tools/imgdis.sh E70D94 22).  Semantics: ../source/domain_os/audit/
  is_process_audited.c (true when the current process's suspend count is
  0); layout: audit/audit_internal.h (suspend_count[0..63] at +0 of
  AUDIT_$DATA, indexed with pid - 1).

  The unit is a fragment of the AUDIT module, so tools/conform.py compiles
  it with `dpc --fragment-origin 0xE70D94'.  The data block is the map's
  `D E854D8 AUDIT' (manifest --section-addr).

  Patterns exercised (docs/codegen-patterns.md):
  - 2, 4 link.w A6,#-4 (the result variable), `pea (A5)', `lea
         <AUDIT data>,A5', restore from (-8,A6);
  - 4    another unit's variable by absolute address (PROC1_$CURRENT);
  - 5.2  an element of a 1-based word array in the unit's data: the index
         read from memory and doubled in place, `add.w D0,D0', the lower
         bound folded into the displacement, `(-2,A5,D0.w)'
         (0xE70DA0..0xE70DA8);
  - 7.1  the comparison with 0 as `tst.w';
  - 3    a comparison as the boolean result: `seq' into the index
         register, which is D0, so no move (0xE70DAC).

  Reconstruction notes: the C's suspend_count[pid - 1] is a 1-based array
  here; AUDIT_$DATA's other fields are left out (only the 128 bytes of the
  array are declared). }
module audit;
var
  audit_$suspend_count: array[1..64] of integer;   { 0x00..0x7F }
  proc1_$current: extern integer;
function audit_$is_process_audited: boolean;
begin
  audit_$is_process_audited := audit_$suspend_count[proc1_$current] = 0;
end;
