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
  before the table are padding here.

  The block's last 16 bytes, +0x124..+0x133, are the exception frame
  size table of the image (`08 08 0C 00 10 00 00 00 3A 14 20 5C 00 00 00
  00', one byte per 68010 stack frame format; ../source fim/fim_data.c,
  FIM_$DATA's frame_size), given as initial values (ref 3.11.1.2; bead
  domain-pascal-63t), so the block is the map segment's 0x134 bytes with
  contents, compared with the image by tools/conform.py and with the C
  block's initial bytes by tools/linktest.py.  The table is a `packed
  array of 0..255': one byte per element (ref 3.11.4.2 Table 3-5 p. 3-47,
  "Subranges of integers: Exact number of bits required", 8 for 0..255;
  3.11.3 p. 3-45, packed arrays "store sequences of ... byte integers";
  bead domain-pascal-51e), read as unsigned bytes by FIM_$BUILD_DF
  (0xE0A486..0xE0A492 `clr.l D2; ...; move.b (0x124,A2),D2'; 0xE0A666
  `clr.l D1; move.b (0x124,A5),D1').  How the original declared it is not
  known: `array of [byte] 0..255' (ref 3.15.5 p. 3-62, ../source's
  fim/fim.pas) lays it out the same. }
module fim_;
var
  fim_pad: array[1..15] of integer32;                    { 0x00..0x3B }
  fim_$user_fim_addr: array[0..57] of integer32;         { 0x3C..0x123 }
  fim_frame_size: packed array[0..15] of 0..255 := [     { 0x124..0x133 }
    16#08, 16#08, 16#0C, 0, 16#10, 0, 0, 0, 16#3A, 16#14, 16#20, 16#5C, * of 0];
  proc1_$as_id: extern integer;
function fim_$get_fim_addr: integer32;
begin
  fim_$get_fim_addr := fim_$user_fim_addr[proc1_$as_id];
end;
