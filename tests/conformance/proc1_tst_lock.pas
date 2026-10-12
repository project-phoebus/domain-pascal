{ PROC1_$TST_LOCK, 0xE148CA, 0x1C bytes, and PROC1_$GET_LOCKS, 0xE148E6,
  0x12 bytes: 0xE148CA..0xE148F8 (tools/imgdis.sh E148CA 2E).
  Semantics: ../source/domain_os/proc1/tst_lock.c, get_locks.c.

  The unit is a fragment of the PROC1 module that starts at 2 mod 4 and is
  0x2E bytes long, so tools/conform.py compiles it with
  `dpc --fragment-origin 0xE148CA': no module-end 0x2048 padding
  (codegen-patterns 7, 11).

  Patterns exercised (docs/codegen-patterns.md):
  - 1.2  val_param: lock_id by value in a 2-byte slot at (8,A6);
  - 2    link.w A6,#-4 for a 1-byte (TST_LOCK) or 4-byte (GET_LOCKS)
         frame, the result variable, no movem, unlk; rts;
  - 3    boolean function result materialised with sne, moved to D0.b;
         the result is computed straight into D0, the frame slot is unused;
         GET_LOCKS: integer32 result loaded straight into D0;
  - 4    another unit's variable by absolute address (PROC1_$CURRENT_PCB);
  - 5.1  `in` on a one-longword set: btst Dn,Dm, the set's address
         loaded before the element.

  The PCB layout is reduced to the one field used: resource_locks_held is
  at 0x40 (proc1_t in ../source/domain_os/proc1/proc1.h).  The variant
  that reads the same longword as an integer32 for GET_LOCKS is our
  reconstruction; how the original declared it is unknown. }
module proc1_tst;
type
  proc1_$pcb_t = record
    pad: array[1..16] of integer32;      { 0x00..0x3F }
    case integer of                      { 0x40 }
      1: (resource_locks_held: set of 0..31);
      2: (resource_locks_word: integer32);
  end;
var
  proc1_$current_pcb: extern ^proc1_$pcb_t;
function proc1_$tst_lock(lock_id: integer): boolean; options(val_param);
begin
  proc1_$tst_lock := lock_id in proc1_$current_pcb^.resource_locks_held;
end;
function proc1_$get_locks: integer32;
begin
  proc1_$get_locks := proc1_$current_pcb^.resource_locks_word;
end;
