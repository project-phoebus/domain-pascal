{ TTY_$I_SET_RAW 0xE673AA..0xE673EA (0x40 bytes) and TTY_$I_INQ_RAW
  0xE673EA..0xE67422 (0x38 bytes; tools/imgdis.sh E673AA 78), a fragment
  of the TTY module's default code section (.TEXT), with TTY_$I_GET_DESC
  declared the way bead domain-pascal-yzxr's reading has it: a 4-byte
  integer result in A0 (`integer32; options(a0_return)', ref 5.5.11,
  p. 5-20), so that its callers that keep the descriptor in D0 hold it in
  an integer local (tests/conformance/tty_k_flags_int.pas) and these two
  convert it to the pointer type at the assignment.  Reconstructed from
  ../source's Domain Pascal translation (domain_os/tty/i_raw.pas, by
  Claude Opus 5.5), reduced to what the routines touch.  Semantics:
  ../source domain_os/tty/i_raw.c.  TTY_$I_GET_DESC 0xE66738 (OS_TERM) is
  external; tty_$i_set_raw_mode 0xE1BF70 is a WIRED_PROC module-local
  routine outside the unit (`module-routines=', called with `jsr abs.l'
  from .TEXT, codegen-patterns 1.6).  A5 is loaded and not used
  (`a5-always=', codegen-patterns 4); the data area is placed at the
  .DATA block 0xE8242C.  The pointer local left in A0 is dpc's default
  (formerly `dpc --no-park-pointer-locals'; bead domain-pascal-78n2).

  Patterns exercised (docs/codegen-patterns.md):
  - 3 "Pointer locals held in D0"  a type transfer of an `a0_return'
         integer function's value to a pointer type is the call's A0
         (`tty := tty_$desc_ptr_t(tty_$i_get_desc(line, status))'):
         copied to A2 where a call follows (`movea.l A0,A2', 0xE673CC),
         left in A0 itself where none does (`move.b (0x4d9,A0),(A1)',
         0xE67414), the `out' parameter's slot then in A1;
  - 12.4 R1b  the pointer local in A2 before the status parameter (A3),
         0xE673B8 / 0xE673CC;
  - 7.1, 9  `if status.all <> status_$ok then return';
  - 1.2, 1.3  the line by value as a word, the boolean by value as a
         byte from the high byte of its slot, the pad first; the last
         call not popped before `unlk' (1.1).

  Reconstructed (not known from the original): the names (the C tree's
  and ../source's); the integer declaration of TTY_$I_GET_DESC (the
  reading under test); `val_param' (../source's reasons, its i_raw.pas);
  the descriptor record as a stand-in array and the one field read at
  0x4D9; `standin', the unit's data area. }

module tty;
type
  status_$t = record case integer of 1: (all: integer32); end;
  tty_$desc_t = record
    pad: array[0..1240] of char;
    raw_mode: boolean;                    { 0x4D9 }
  end;
  tty_$desc_ptr_t = ^tty_$desc_t;
const
  status_$ok = 0;
var
  standin: integer32;

function tty_$i_get_desc(line: integer; out status: status_$t):
  integer32; options(extern, val_param, a0_return);

{ WIRED_PROC 0xE1BF70, outside the unit }
procedure tty_$i_set_raw_mode(in out tty: tty_$desc_t; raw: boolean);
  internal; extern;

procedure tty_$i_set_raw(line: integer; raw: boolean; out status: status_$t);
  options(val_param);
var
  tty: tty_$desc_ptr_t;                   { A2 }
begin
  tty := tty_$desc_ptr_t(tty_$i_get_desc(line, status));  { 0x00E673BC }
  if status.all <> status_$ok then                  { 0x00E673CE }
    return;
  tty_$i_set_raw_mode(tty^, raw);                   { 0x00E673D2 }
end;

procedure tty_$i_inq_raw(line: integer; out raw: boolean;
                         out status: status_$t);
  options(val_param);
var
  tty: tty_$desc_ptr_t;                   { A0 }
begin
  tty := tty_$desc_ptr_t(tty_$i_get_desc(line, status));  { 0x00E673FC }
  if status.all <> status_$ok then                  { 0x00E6740C }
    return;
  raw := tty^.raw_mode;                             { 0x00E67410 }
end;
