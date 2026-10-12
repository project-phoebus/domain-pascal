{ FIM_$GET_FIM_ADDR, 0xE0AA04, 0x20 bytes: 0xE0AA04..0xE0AA24
  (tools/imgdis.sh E0AA04 20).  Semantics: ../source/domain_os/fim/
  get_fim_addr.c; layout: fim/fim.h (FIM_$DATA, user_fim_addr[0..57] at
  +0x3C of the FIM_ data block, the map's FIM_$USER_FIM_ADDR).

  The unit is a fragment of the FIM_ module, so tools/conform.py compiles
  it with `dpc --fragment-origin 0xE0AA04'.  The data block is the map's
  `D E2126C FIM_' (manifest --section-addr).

  Patterns exercised (docs/codegen-patterns.md):
  - 2, 4 link.w A6,#-4 (the 4-byte result variable), `pea (A5)', `lea
         <FIM_ data>,A5', restore from (-8,A6);
  - 4    another unit's variable by absolute address (PROC1_$AS_ID);
  - 5.2  an element of a 0-based longword array in the unit's data: the
         index read from memory and scaled in place, `lsl.w #2,D0', then
         the indexed mode with an 8-bit displacement, `(0x3c,A5,D0.w)'
         (0xE0AA10..0xE0AA18);
  - 3    the longword result loaded straight into D0.

  Reconstruction notes: the C returns a pointer.  A Pascal pointer result
  comes back in A0 (ref 5.5.10, p. 5-20), and `d0_return' puts it in both
  A0 and D0; the routine writes only D0 (it is a TRAP #0 service, whose
  results are taken from D0), so its result type is declared integer32
  here.  How the original declared it is not known.  The 0x3C bytes
  before the table are padding here. }
module fim_;
var
  fim_pad: array[1..15] of integer32;                    { 0x00..0x3B }
  fim_$user_fim_addr: array[0..57] of integer32;         { 0x3C..0x123 }
  proc1_$as_id: extern integer;
function fim_$get_fim_addr: integer32;
begin
  fim_$get_fim_addr := fim_$user_fim_addr[proc1_$as_id];
end;
