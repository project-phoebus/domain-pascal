{ Golden: codegen-patterns 5.1 (set that fits in a register: btst Dn,Dm),
  3 (boolean result: sne, then move.b into D0) and 2 (link.w #-4 for a
  1-byte frame; unlk; rts).  Mirrors PROC1_$TST_LOCK 0xE148CA..0xE148E4. }
module proc1_tst;
type
  proc1_$pcb_t = record
    pad: array[1..16] of integer32;
    resource_locks_held: set of 0..31;
  end;
var
  proc1_$current_pcb: extern ^proc1_$pcb_t;
function proc1_$tst_lock(lock_id: integer): boolean; options(val_param);
begin
  proc1_$tst_lock := lock_id in proc1_$current_pcb^.resource_locks_held;
end;
