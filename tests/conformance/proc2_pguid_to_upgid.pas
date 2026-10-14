{ PROC2_$PGUID_TO_UPGID, 0xE41072, 0x56 bytes: 0xE41072..0xE410C8
  (tools/imgdis.sh E41072 56).  Semantics:
  ../source/domain_os/proc2/pguid_to_upgid.c; the module-local routine at
  0xE422CC (map: no name; ../source's PROC2_$UID_TO_UPGID_INTERNAL) lies
  outside the unit (`module-routines=PROC2_$UID_TO_UPGID_INTERNAL=
  0xE422CC').  Externals (SAU2 map): ML_$LOCK 0xE20B12, ML_$UNLOCK
  0xE20B62.

  The unit is a fragment of the PROC2 module, so tools/conform.py
  compiles it with `dpc --fragment-origin 0xE41072'.  Its data area,
  `.bss.moddata.PROC2', is placed at 0xE7BE84, the address the routine
  loads into A5 without using the data (`a5-always=
  PROC2_$PGUID_TO_UPGID').

  Patterns exercised (docs/codegen-patterns.md):
  - 12.4 R4(b) after a call (bead domain-pascal-178): `upgid' and
         `status', slots used once after ML_$UNLOCK in straight-line code,
         in A0 (0xE410B2) and then A1 (0xE410B8): the straight-line rule
         restarted at the call (A0, left by the statement before, is
         passed over though dead);
  - 1.2  the copy of a value parameter passed by address, `(A0)+' into
         the frame (0xE41080..0xE41088);
  - 12.5 the local `idx' held in D2 across ML_$UNLOCK (0xE410A4);
  - 3    a word function result in D0 from an `internal' routine;
  - 1.1  pops where a push follows, none after the last call;
  - 4    always mode: `lea 0xE7BE84,A5', A5 saved in the `movem'.

  Reconstructed (not known from the original): the names; uid_$t as two
  integer32 fields; `out' for the two by-address results;
  `proc2_$data_stand_in', a variable only so that the unit has a data
  area (mmap_alloc_contig.pas). }
module proc2;
type
  uid_$t = record high, low: integer32; end;
  status_$t = integer32;
  pinteger = 0..65535;
var
  proc2_$data_stand_in: integer32;  { stand-in, see above }

procedure ml_$lock(id: integer); options(val_param, extern);
procedure ml_$unlock(id: integer); options(val_param, extern);
function proc2_$uid_to_upgid_internal(var uid: uid_$t): pinteger; internal; extern;

procedure proc2_$pguid_to_upgid(pgroup_uid: uid_$t; out upgid: pinteger;
                                out status: status_$t);
var
  upgid_local: pinteger;
begin
  ml_$lock(4);
  upgid_local := proc2_$uid_to_upgid_internal(pgroup_uid);
  ml_$unlock(4);
  upgid := upgid_local;
  status := 0;
end;
