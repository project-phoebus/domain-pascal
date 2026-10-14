{ TIME_$Q_REMOVE_ELEM, 0xE16E48, 0x4C bytes: 0xE16E48..0xE16E94
  (tools/imgdis.sh E16E48 4C).  Semantics:
  ../source/domain_os/time/q_remove_elem.c; the module-local routine at
  0xE16B70 (map: no name; ../source's time_$q_remove_internal,
  time/time_internal.h) lies outside the unit (`module-routines=
  TIME_$Q_REMOVE_INTERNAL=0xE16B70').  Externals (SAU2 map):
  ML_$SPIN_LOCK 0xE20BB6, ML_$SPIN_UNLOCK 0xE20BBE.

  The unit is a fragment of the TIME module, so tools/conform.py compiles
  it with `dpc --fragment-origin 0xE16E48'.

  Patterns exercised (docs/codegen-patterns.md):
  - 12.4 R4(b) after a call (bead domain-pascal-178): `status', a slot
         used once, is loaded after the last call into A0 (0xE16E84), the
         lowest free register, which no earlier statement left a value in
         (the calls wrote A0 and A1 after the entry load into A2, and
         nothing else writes A0 or A1);
  - 12.2 `queue', used three times and across calls, loaded at entry
         into A2 (0xE16E4E), saved with `pea (A2)';
  - 3    a word function result stored from D0 into a frame local
         (0xE16E5E);
  - 1.1  pops: `addq.w #4' and `lea (0xc,SP),SP' where a push follows,
         none after the last call;
  - 1.2/1.3 `pea (4,A2)' for a field passed by address, a by-address
         parameter passed on (`move.l (0xc,A6),-(SP)'), a val_param word
         with its pad.

  Reconstructed (not known from the original): the names; the element
  record (passed on only); the queue record as in time_q_init_queue.pas;
  the declaration order of the locals (token at -6, status at -4). }
module time_q;
type
  status_$t = integer32;
  time_$queue_t = record
    head: integer32;          { 0x0 }
    lock: integer32;          { 0x4: ML_$SPIN_LOCK cell }
    is_vt: boolean;           { 0x8 }
    queue_id: integer;        { 0xA }
  end;
  time_$elem_t = record
    next: integer32;
  end;

function ml_$spin_lock(var lock: integer32): integer; extern;
procedure ml_$spin_unlock(var lock: integer32; token: integer); options(val_param, extern);
procedure time_$q_remove_internal(var queue: time_$queue_t; var elem: time_$elem_t;
                                  out status: status_$t); internal; extern;

procedure time_$q_remove_elem(var queue: time_$queue_t; var elem: time_$elem_t;
                              out status: status_$t);
var
  local_status: status_$t;
  token: integer;
begin
  token := ml_$spin_lock(queue.lock);
  time_$q_remove_internal(queue, elem, local_status);
  ml_$spin_unlock(queue.lock, token);
  status := local_status;
end;
