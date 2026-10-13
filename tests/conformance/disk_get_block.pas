{ DISK_$GET_BLOCK, 0xE3BB80..0xE3BBD4 (0x54 bytes; tools/imgdis.sh
  E3BB80 54), the first routine of the DISK_ module (code segment
  0xE3BB80).  Semantics: ../source/domain_os/disk/get_block.c
  (DBUF_$GET_BLOCK under the disk resource lock 15).  Externals (SAU2
  map): ML_$LOCK 0xE20B12, ML_$UNLOCK 0xE20B62, DBUF_$GET_BLOCK 0xE3A5B0.

  A fragment of the module, compiled with `dpc --fragment-origin
  0xE3BB80'.  No loop, no unit data (no A5, codegen-patterns 4).

  Patterns exercised (docs/codegen-patterns.md):
  - 3    a pointer result in a body of several statements: the result
         variable held in an address register up to the epilogue, A2
         because ML_$UNLOCK follows (12.4 R3), assigned from
         DBUF_$GET_BLOCK's A0 with `movea.l A0,A2' after the pop
         (0xE3BBBC), moved to A0 with `movea.l A2,A0' at the start of
         the epilogue (0xE3BBCA); its 4 bytes make `link #-4';
  - 2    A2 the only saved register, `pea (A2)' / `movea.l (-8,A6),A2';
  - 1.2-1.4  `val_param': the seven parameters by value in word and
         longword slots (the two by-address ones are the pointers in
         their slots), pushed again in the same order; a 2-byte pad
         before ML_$LOCK's and ML_$UNLOCK's word and before the
         22 argument bytes of DBUF_$GET_BLOCK; ML_$UNLOCK not popped.

  Reconstructed (not known from the original): the names; `val_param'
  for both DISK_$GET_BLOCK and DBUF_$GET_BLOCK (their arguments are
  values in the slots: DBUF_$GET_BLOCK reads them with `move.w
  (0x16,A6),D4' etc., ../source/domain_os/dbuf/dbuf.h); the UID and
  status as `var' parameters (by address under val_param, ref 5.2
  Table 5-1); the block type and flags as integers. }
module disk_;
type
  status_$t = integer32;
  uid_$t = record high, low: integer32; end;

procedure ml_$lock(resource_id: integer); options(val_param, extern);
procedure ml_$unlock(resource_id: integer); options(val_param, extern);
function dbuf_$get_block(vol: integer; daddr: integer32; var uid: uid_$t;
                         hint: integer32; block_type: integer; flags: integer;
                         var status: status_$t): univ_ptr; val_param; extern;

function disk_$get_block(vol: integer; daddr: integer32; var uid: uid_$t;
                         hint: integer32; block_type: integer; flags: integer;
                         var status: status_$t): univ_ptr; val_param;
begin
  ml_$lock(15);
  disk_$get_block := dbuf_$get_block(vol, daddr, uid, hint, block_type, flags, status);
  ml_$unlock(15);
end;
