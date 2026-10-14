{ TTY_$K_SET_SESSION_ID, TTY_$K_INQ_SESSION_ID, 0xE67986..0xE67A02 (0x7C
  bytes; tools/imgdis.sh E67986 7C), a fragment of the TTY module's
  default code section (.TEXT).  Reconstructed from ../source's Domain
  Pascal translation (domain_os/tty/k_pgroup.pas at ../source 3898a2ce,
  by Claude Opus 5.5; TTY_$I_GET_DESC and the locals as at ../source
  c8f980f8), reduced to what the routines touch.  Semantics:
  ../source domain_os/tty/k_pgroup.c.  TTY_$I_GET_DESC 0xE66738 is
  external; A5 is loaded and not used (`a5-always=', codegen-patterns 4),
  the data area placed at .DATA 0xE8242C.

  Patterns exercised (docs/codegen-patterns.md):
  - 3, 12.4  an integer local assigned TTY_$I_GET_DESC's `a0_return'
         result and dereferenced through a type transfer, held in D0
         (beads domain-pascal-dgb, -yzxr, -78n2: formerly a pointer
         local, whose D0 holding is retired), `move.l A0,D0' after the
         call (0xE679AA), `movea.l D0,A0' first
         in the statement that dereferences it, as the destination's base
         (`movea.l D0,A0; movea.l (0xc,A6),A1; move.w (A1),(0x54,A0)',
         0xE679B0) and as the source's (0xE679EE);
  - 7.1, 9  `if status.all <> status_$ok then return'.

  Reconstructed (not known from the original): the names; the
  descriptor record up to session_id at 0x54 (../source
  domain_os/tty/tty.h), the bytes before it as one stand-in array;
  `unused_8' and `standin' as in tty_k_flags_int.pas. }
module tty;
type
  status_$t = record case integer of 1: (all: integer32); end;
  tty_$desc_t = record
    pad: array[0..41] of integer;         { 0x00..0x53 }
    session_id: integer;                  { 0x54 }
  end;
  tty_$desc_ptr_t = ^tty_$desc_t;
const
  status_$ok = 0;
var
  standin: integer32;

function tty_$i_get_desc(line: integer; out status: status_$t):
  integer32; options(extern, val_param, a0_return);

procedure tty_$k_set_session_id(in line: integer; in sid: integer;
                                out status: status_$t);
var
  tty: integer32;
  unused_8: integer32;
begin
  tty := tty_$i_get_desc(line, status);
  if status.all <> status_$ok then
    return;
  tty_$desc_ptr_t(tty)^.session_id := sid;
end;

procedure tty_$k_inq_session_id(in line: integer; out sid: integer;
                                out status: status_$t);
var
  tty: integer32;
  unused_8: integer32;
begin
  tty := tty_$i_get_desc(line, status);
  if status.all <> status_$ok then
    return;
  sid := tty_$desc_ptr_t(tty)^.session_id;
end;
