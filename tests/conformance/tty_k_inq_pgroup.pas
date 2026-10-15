{ TTY_$K_INQ_PGROUP, 0xE67942..0xE67986 (0x44 bytes; tools/imgdis.sh
  E67942 44), a fragment of the TTY module's default code section
  (.TEXT).  Reconstructed from ../source's Domain Pascal translation
  (domain_os/tty/k_pgroup.pas at ../source 340effeeac20), reduced to what
  the routine touches.  Semantics: ../source domain_os/tty/k_pgroup.c.
  TTY_$I_GET_DESC 0xE66738 is external; A5 is loaded and not used
  (`a5-always=', codegen-patterns 4), the data area placed at .DATA
  0xE8242C.  TTY_$K_INQ_INPUT_BREAK_MODE 0xE678BC (tty/k_break.pas) has
  the same code with the field at 0x38 (srcsurvey: OK).

  Patterns exercised (docs/codegen-patterns.md):
  - 12.4 R4(b)  the `out' parameter's slot loaded after the status test,
         into A3: A2, the status loaded at entry, holds a dead slot load
         on every path and is passed over; A0 holds the descriptor, A1
         the copy's source (0xE6796E; bead domain-pascal-i24w);
  - 5.2  an 8-byte record assignment out of the descriptor reached
         through the integer local held in D0: `movea.l D0,A0', the
         destination's slot, `lea (0x4c,A0),A1', `move.l (A1)+,(A3);
         move.l (A1)+,(4,A3)' (0xE6796C..0xE67978; bead domain-pascal-fbg);
  - 3, 12.4  the local held in D0 (`move.l A0,D0', 0xE67966);
  - 7.1, 9  `if status.all <> status_$ok then return'.

  Reconstructed (not known from the original): the names; the
  descriptor record up to pgroup_uid at 0x4C (../source
  domain_os/tty/tty.h), the bytes before it as one stand-in array;
  `unused_8' as in tty_k_set_pgroup.pas. }
module tty;
type
  status_$t = record case integer of 1: (all: integer32); end;
  uid_$t = record
    high: integer32;
    low: integer32;
  end;
  tty_$desc_t = record
    pad: array[0..37] of integer;         { 0x00..0x4B }
    pgroup_uid: uid_$t;                   { 0x4C }
  end;
  tty_$desc_ptr_t = ^tty_$desc_t;
const
  status_$ok = 0;
var
  standin: integer32;

function tty_$i_get_desc(line: integer; out status: status_$t):
  integer32; options(extern, val_param, a0_return);

procedure tty_$k_inq_pgroup(in line: integer; out uid: uid_$t;
                            out status: status_$t);
var
  tty: integer32;
  unused_8: integer32;
begin
  tty := tty_$i_get_desc(line, status);
  if status.all <> status_$ok then
    return;
  uid := tty_$desc_ptr_t(tty)^.pgroup_uid;
end;
