{ PROC2_$UPGID_TO_UID, 0xE4100C, 0x66 bytes: 0xE4100C..0xE41072
  (tools/imgdis.sh E4100C 66), a fragment of the PROC2 module's code
  segment.  Semantics: ../source domain_os/proc2/upgid_to_uid.c; its
  helper, the module-local 0xE4232A (proc2/upgid_to_uid_internal.c), is
  outside the unit (`module-routines=', called with `bsr.w').

  Patterns exercised (docs/codegen-patterns.md):
  - 3    a function whose 8-byte record result goes through a hidden
         address ("Record results over 4 bytes"): the address of a frame
         temporary pushed last, after the word argument, with no pad
         (`move.w (-0xa,A6),-(SP); pea (-0x14,A6); bsr.w 0xE4232A; addq.w
         #6,SP', 0xE41030..0xE4103C, the only 6-byte pop of compiled
         code), then the record copied from the temporary into the
         destination (`lea (-0x14,A6),A0; move.l (A0)+,(-0x8,A6); move.l
         (A0)+,(-0x4,A6)', 0xE4103E..0xE41046);
  - 1.5, 2  the temporary below the locals at a multiple of 4 (-0x14,
         under the word at -0xA);
  - 1.1  the last call not popped (ML_$UNLOCK, 0xE41050), the unlk
         discarding its arguments;
  - 4    A5 loaded and not used (a5-always; a stand-in variable gives
         the unit a data area, as in proc2_pguid_to_upgid.pas).

  Reconstructed (not known from the original): the names (../source's);
  `upgid_val' as a local copy of the `in' parameter, as ../source's
  proc2/upgid_to_uid.pas reads 0xE41018..0xE4101E; the helper's heading
  (an `internal' function of a word value parameter, ../source's
  proc2/upgid_to_uid_internal.pas; its own code, 0xE4232A, has an 8-byte
  local at -0x10, which dpc places at -0xC: codegen-patterns 2, "Objects
  of 8 bytes or more at multiples of 8", so it is not in the unit). }
module proc2;
type
  uid_$t = record
    high: integer32;
    low: integer32;
  end;
  status_$t = record
    all: integer32;
  end;
const
  proc2_lock_id = 4;
  status_$ok = 0;

var
  proc2_$data_stand_in: integer32;  { stand-in: the unit addresses no data }

procedure ml_$lock(id: integer); options(val_param, extern);
procedure ml_$unlock(id: integer); options(val_param, extern);
function proc2_$upgid_to_uid_internal(upgid: integer): uid_$t; internal; extern;

procedure proc2_$upgid_to_uid(in upgid: integer; out uid_ret: uid_$t;
                              out status_ret: status_$t);
var
  uid: uid_$t;                            { (-0x8,A6) }
  upgid_val: integer;                     { (-0xA,A6) }
begin
  upgid_val := upgid;
  ml_$lock(proc2_lock_id);
  uid := proc2_$upgid_to_uid_internal(upgid_val);
  ml_$unlock(proc2_lock_id);
  uid_ret := uid;
  status_ret.all := status_$ok;
end;
