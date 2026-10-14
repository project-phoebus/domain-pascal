{ TTY_$I_SET_RAW 0xE673AA..0xE673EA (0x40 bytes; tools/imgdis.sh E673AA
  40), a fragment of the TTY module's default code section (.TEXT).
  Reconstructed from ../source's Domain Pascal translation
  (domain_os/tty/i_raw.pas at ../source 650ee33c, by Claude Opus 5.5),
  reduced to what the routine touches.  Semantics: ../source
  domain_os/tty/i_raw.c.  TTY_$I_GET_DESC 0xE66738 (OS_TERM) is
  external; tty_$i_set_raw_mode 0xE1BF70 is a WIRED_PROC module-local
  routine outside the unit (`module-routines=', called with `jsr abs.l'
  from .TEXT, codegen-patterns 1.6).  A5 is loaded and not used
  (`a5-always=', codegen-patterns 4); the data area is placed at the
  .DATA block 0xE8242C.

  Patterns exercised (docs/codegen-patterns.md):
  - 12.4 R1b  a pointer local assigned a pointer function's result
         (`movea.l A0,A2', 0xE673CC) gets its register before the status
         parameter loaded at entry and live across the call, whose last
         use (`tst.l (A3)', 0xE673CE) is in the same block: the status
         address in A3, the local in A2 (bead domain-pascal-ue7);
  - 7.1, 9  `if status.all <> status_$ok then return' (`tst.l (A3);
         bne.b' to the epilogue);
  - 1.2, 1.3  the line by value as a word, the boolean by value as a
         byte from the high byte of its slot (`move.b (0xa,A6),-(SP)'),
         the pad first; the last call not popped before `unlk' (1.1).

  Reconstructed (not known from the original): the names (the C tree's
  and ../source's); the heading `val_param' (../source's reasons, its
  i_raw.pas); the descriptor record as one stand-in field, the routine
  passing it on without touching it; `standin', the unit's data area. }
module tty;
type
  status_$t = record case integer of 1: (all: integer32); end;
  tty_$desc_t = record
    pad: array[0..1] of integer32;
  end;
  tty_$desc_ptr_t = ^tty_$desc_t;
const
  status_$ok = 0;
var
  standin: integer32;

function tty_$i_get_desc(line: integer; out status: status_$t):
  tty_$desc_ptr_t; options(extern, val_param);

{ WIRED_PROC 0xE1BF70, outside the unit }
procedure tty_$i_set_raw_mode(in out tty: tty_$desc_t; raw: boolean);
  internal; extern;

procedure tty_$i_set_raw(line: integer; raw: boolean; out status: status_$t);
  options(val_param);
var
  tty: tty_$desc_ptr_t;                   { A2 }
begin
  tty := tty_$i_get_desc(line, status);             { 0x00E673BC }
  if status.all <> status_$ok then                  { 0x00E673CE }
    return;
  tty_$i_set_raw_mode(tty^, raw);                   { 0x00E673D2 }
end;
