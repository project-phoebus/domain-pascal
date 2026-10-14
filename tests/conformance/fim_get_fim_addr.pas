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
  block's initial bytes by tools/linktest.py.  The element type is not
  known (../source: uint8_t): `char' with chr() stands in for an
  unsigned byte (a subrange 0..255 is 2 bytes, ref 3.8.1 p. 3-14; a
  `packed array of 0..255', 8 bits per element by ref 3.11.4.2 Table
  3-5, is not supported by dpc yet). }
module fim_;
var
  fim_pad: array[1..15] of integer32;                    { 0x00..0x3B }
  fim_$user_fim_addr: array[0..57] of integer32;         { 0x3C..0x123 }
  fim_frame_size: array[0..15] of char := [              { 0x124..0x133 }
    chr(16#08), chr(16#08), chr(16#0C), chr(0), chr(16#10), chr(0), chr(0),
    chr(0), chr(16#3A), chr(16#14), chr(16#20), chr(16#5C), * of chr(0)];
  proc1_$as_id: extern integer;
function fim_$get_fim_addr: integer32;
begin
  fim_$get_fim_addr := fim_$user_fim_addr[proc1_$as_id];
end;
