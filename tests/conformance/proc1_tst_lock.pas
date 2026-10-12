{ PROC1_$TST_LOCK, 0xE148CA, 0x1C bytes (tools/imgdis.sh E148CA 1C).
  Semantics: ../source/domain_os/proc1/tst_lock.c.

  Patterns exercised (docs/codegen-patterns.md):
  - 1.2  val_param: lock_id by value in a 2-byte slot at (8,A6);
  - 2    link.w A6,#-4 for a 1-byte frame (result variable), no movem,
         unlk; rts;
  - 3    boolean function result materialised with sne, moved to D0.b;
         the result is computed straight into D0, the frame slot is unused;
  - 4    another unit's variable by absolute address (PROC1_$CURRENT_PCB);
  - 5.1  `in` on a one-longword set: btst Dn,Dm, the set's address
         loaded before the element.

  The PCB layout is reduced to the one field used: resource_locks_held is
  at 0x40 (proc1_t in ../source/domain_os/proc1/proc1.h). }
module proc1_tst;
type
  proc1_$pcb_t = record
    pad: array[1..16] of integer32;      { 0x00..0x3F }
    resource_locks_held: set of 0..31;   { 0x40 }
  end;
var
  proc1_$current_pcb: extern ^proc1_$pcb_t;
function proc1_$tst_lock(lock_id: integer): boolean; options(val_param);
begin
  proc1_$tst_lock := lock_id in proc1_$current_pcb^.resource_locks_held;
end;
