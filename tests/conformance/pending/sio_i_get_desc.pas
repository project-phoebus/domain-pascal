{ SIO_$I_GET_DESC, 0xE667C6..0xE6681C (0x56 bytes; tools/imgdis.sh
  E667C6 56), the second routine of the OS_TERM code segment (map `I
  E66738 OS_TERM size = AA4', the default section), so a fragment
  (`dpc --fragment-origin 0xE667C6').  Semantics:
  ../source/domain_os/sio/i_get_desc.c (and its Pascal,
  sio/i_get_desc.pas).  Externals (SAU2 map): TERM_$GET_REAL_LINE
  0xE1A9D4, DTTE 0xE2DC90.

  Patterns exercised (docs/codegen-patterns.md):
  - 3    a pointer result whose every assignment is last, from memory
         (bead domain-pascal-yqsm): the result variable kept in its frame
         slot, `move.l (0x28,A0,D0.w),(-4,A6)' at 0xE66808, and reloaded
         at the start of the epilogue, `movea.l (-4,A6),A0' at 0xE6680E,
         where both `return's branch to (0xE667E6, 0xE66806) with the slot
         unwritten;
  - 1.4  the by-value word `line' pushed from its slot, the pad before
         the pushes (0xE667D2);
  - 8    the element offset `real_line * 0x38' as 64x - 8x (0xE667E8).

  Pending: dpc compiles it, and the slot store and reload match the
  image, but the element offset is computed again for the assignment
  (tools/conform.py check --routine SIO_$I_GET_DESC:56; 0xE66808 reuses
  D0 and A0 from the test; bead domain-pascal-gg57).  The result
  variable is at -4, above real_line's slot (-6), as in the image
  (codegen-patterns 2, "Where the compiler's objects go"; bead
  domain-pascal-67c).
  Its manifest line would be `tests/conformance/pending/sio_i_get_desc.pas
  SIO_$I_GET_DESC:56'.

  Reconstructed (not known from the original): every name; the result
  a pointer (`../source' declares it `integer32' with `a0_return', which
  gives the same code in A0: ref 5.5.11); real_line a word local, held
  in D1; the error arms as `return'. }
module os_term;
type
  status_$t = integer32;
  pinteger = 0..65535;
  dtte_t = record
    ec_00: array[1..12] of char;
    input_ec: array[1..12] of char;
    output_ec: array[1..12] of char;
    handler_ptr: univ_ptr;
    tty_handler: univ_ptr;
    alt_handler: univ_ptr;
    ptr_30: univ_ptr;
    discipline: integer;
    flags: boolean;
    pad_37: char;
  end;

var
  dtte: extern array[0..3] of dtte_t;

function term_$get_real_line(line: integer; out status: status_$t):
  pinteger; options(extern, val_param);

function sio_$i_get_desc(line: integer; out status: status_$t):
  univ_ptr; options(val_param);
var
  real_line: pinteger;
begin
  real_line := term_$get_real_line(line, status);
  if status <> 0 then
    return;
  if dtte[real_line].tty_handler = nil then begin
    status := 16#B000D;
    return;
  end;
  sio_$i_get_desc := dtte[real_line].tty_handler;
end;
