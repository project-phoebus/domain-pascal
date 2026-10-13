{ REM_NAME_$REGISTER_SERVER, 0xE4A4AE, 0x1A bytes (tools/imgdis.sh E4A4AE
  1A).  Semantics: ../source/domain_os/rem_name/register_server.c: stores
  TIME_$CLOCKH (map 0xE2B0D4) in the REM_NAME data's time-heard-from-
  server field and sets its heard-from-server flag; its two by-address
  arguments are never read.

  A fragment of the REM_NAME module (code segment 0xE4A408), compiled with
  `dpc --fragment-origin 0xE4A4AE'.  Its data area,
  `.bss.moddata.REM_NAME', is placed at the map's `D E7DBB8 REM_NAME'
  (manifest --section-addr).  The map's next name is
  REM_NAME_$GET_ENTRY_BY_NAME 0xE4A588; a module-local routine starts at
  0xE4A4C8, so the manifest gives the length (`:1A').

  This routine does not load A5: it addresses its own data block from
  its absolute address, `lea 0xE7DBB8,A0' (codegen-patterns 4, one of the
  three used-mode routines that do; the cause is not known), so the
  manifest passes `a5-absolute=REM_NAME_$REGISTER_SERVER'.

  Patterns exercised (docs/codegen-patterns.md):
  - 4    the unit's data addressed absolutely: `lea <block>.l,A0' at the
         first static operand, A0 kept for the second statement,
         `(0x28,A0)', `(0x3c,A0)' (0xE4A4B2..0xE4A4C0); no A5 save;
  - 2    `link #-8' with a frame the body never touches;
  - 4, 13  another unit's variable by absolute address, memory to memory
         `move.l TIME_$CLOCKH,(0x28,A0)'; `st' for true.

  Reconstructed (not known from the original): the parameters (`in
  integer32' for the two 4-byte slots; ../source's names); `frame', a
  local declared only for the 8 bytes of `link #-8' (the routine never
  touches its frame; whether the original declared locals is Uncertain,
  codegen-patterns 2); the data-block layout up to the two fields
  (padding arrays; the map names REM_NAME_$TIME_HEARD_FROM_SERVER at
  0xE7DBE0 = +0x28 and REM_NAME_$HEARD_FROM_SERVER at 0xE7DBF4 = +0x3C,
  ../source rem_name/rem_name_internal.h); the variables are not
  `define'd here. }
module rem_name;
var
  rem_name_$pad_00: array[0..9] of integer32;          { +0x00..+0x27 }
  rem_name_$time_heard_from_server: integer32;         { +0x28 }
  rem_name_$pad_2c: array[0..3] of integer32;          { +0x2C..+0x3B }
  rem_name_$heard_from_server: boolean;                { +0x3C }
  time_$clockh: extern integer32;

procedure rem_name_$register_server(in net: integer32; in node: integer32);
var
  frame: array[0..1] of integer32;   { stand-in for link #-8, see above }
begin
  rem_name_$time_heard_from_server := time_$clockh;
  rem_name_$heard_from_server := true;
end;
