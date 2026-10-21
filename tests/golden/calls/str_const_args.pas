{ Golden: codegen-patterns 1.5 "Constant by address", "A string constant
  to an array of char"; bead domain-pascal-jgtd.

  A string constant passed by address to a value or `in' parameter that
  is an unpacked array of char, not `univ', is a cell of the
  parameter's size: the characters, then blanks (ref p. 4-204, p.
  4-207), an odd size followed by one zero byte:
  - SAU5 TAPE_$BOOT's tape_$check_block 0xE33F12 `pea (0x8c,PC)' names
    0xE33FA0..0xE34068, 80 characters and 120 blanks, for an `in
    vfmt_$string_t' (200 bytes); its two `univ' strings keep their own
    sizes; the three cells in the routine's order, the 2-mod-4 one
    first (0xE33F72, 0xE33FA0, 0xE34068);
  - SAU5 TAPE_$BOOT 0xE344C0, 0xE344D4, 0xE344E4: one 16-byte cell,
    0xE346DE, for a 16-character name constant passed three times to a
    16-character `in' formal (val_param, with a word by value). }
module scon;
type
  vfmt_$string_t = array[1..200] of char;
  name_t = array[1..16] of char;
  odd_t = array[1..5] of char;
  arg_t = [aligned(0)] integer32;

procedure vfmt_$write2(in fmt: univ vfmt_$string_t; in a1, a2: univ arg_t); extern;
procedure ext_$print(in msg: vfmt_$string_t); extern;
function ext_$find(in name: name_t; len: integer): boolean; options(val_param); extern;
procedure ext_$upcase(in name: name_t; len: integer); options(val_param); extern;
procedure ext_$odd(s: odd_t); extern;

procedure scon_$check(seq, expected: integer32);
begin
  ext_$print('tape error: UID in block header does not match those read from earlier blocks.%.');
  vfmt_$write2('correct uid = %lh.%lh, uid read = %lh.%lh, seq=%lh%.', seq, expected);
  vfmt_$write2('** Seq out of order: expected %lh, read %lh%.', expected, seq);
end;

function scon_$find: boolean;
const
  shell_name = 'bscom/rbak_shell';
begin
  scon_$find := true;
  if not ext_$find(shell_name, 16) then begin
    ext_$upcase(shell_name, 16);
    if not ext_$find(shell_name, 16) then
      scon_$find := false;
  end;
  ext_$odd('abc');
end;
