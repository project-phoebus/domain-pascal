{ PKT_$NEXT_ID, 0xE1248E, 0x4E bytes (tools/imgdis.sh E1248E 4E).
  Semantics: ../source/domain_os/pkt/next_id.c; data layout:
  pkt_$data_t in ../source/domain_os/pkt/pkt_internal.h (its
  _Static_asserts: spin_lock 0x50, short_id 0x5C; map `D E24C9C PKT size =
  A8').  Externals (SAU2 map): ML_$SPIN_LOCK 0xE20BB6, ML_$SPIN_UNLOCK
  0xE20BBE.

  A fragment of the PKT module (code segment 0xE1202C), compiled with
  `dpc --fragment-origin 0xE1248E'; the data area `.bss.moddata.PKT' is
  placed at 0xE24C9C by the manifest.  The routine uses the unit's data,
  so it loads A5 (codegen-patterns 4; `tools/a5survey.py runs PKT').

  Patterns exercised (docs/codegen-patterns.md):
  - 3    the result variable assigned before other statements, held in a
         register up to the epilogue: `move.w (0x5c,A5),D2' (0xE124AC),
         callee-saved because a call follows (12.4 R3), then `move.w
         D2,D0' (0xE124D0); its 2 bytes and the token's make `link #-4';
  - 12.5 the token, a local stored from the call's D0 (`move.w
         D0,(-2,A6)', 0xE124A8) and pushed from D0, which nothing wrote
         since, across the `if' (0xE124C4 `move.w D0,-(SP)');
  - 8    `short_id := short_id + 1' in place (`addq.w #1,(0x5c,A5)');
  - 7.1  an unsigned word compare with the constant 64000 (`cmpi.w
         #-0x600'; `bls');
  - 1.3, 1.4  the token by value in a word slot, the 2-byte pad first.

  Reconstructed (not known from the original): the names; ML_$SPIN_LOCK
  as a function of a `var' lock returning an integer token, and
  ML_$SPIN_UNLOCK as `val_param' (the lock passed by address, the token
  by value); the lock as an integer32 (the C's uint32_t); the fields
  before 0x50 and between 0x54 and 0x5C declared as filler to put the two
  used ones at their offsets (dpc-sema lays statics out in declaration
  order). }
module pkt;
var
  missing_nodes: array[1..20] of integer32;   { 0x00 }
  spin_lock: integer32;                         { 0x50 }
  filler: array[0..3] of integer;               { 0x54 }
  short_id: 0..65535;                           { 0x5C }

function ml_$spin_lock(var lock: integer32): integer; extern;
procedure ml_$spin_unlock(var lock: integer32; token: integer); options(extern, val_param);

function pkt_$next_id: integer;
var
  token: integer;
begin
  token := ml_$spin_lock(spin_lock);
  pkt_$next_id := short_id;
  short_id := short_id + 1;
  if short_id > 16#FA00 then short_id := 1;
  ml_$spin_unlock(spin_lock, token);
end;
