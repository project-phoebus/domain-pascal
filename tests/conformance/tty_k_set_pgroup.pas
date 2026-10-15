{ TTY_$K_SET_PGROUP, 0xE67900..0xE67942 (0x42 bytes; tools/imgdis.sh
  E67900 42), a fragment of the TTY module's default code section
  (.TEXT).  Reconstructed from ../source's Domain Pascal translation
  (domain_os/tty/k_pgroup.pas at ../source 368121458ed7, by Claude Opus
  5.5), reduced to what the routine touches.  Semantics: ../source
  domain_os/tty/k_pgroup.c.  TTY_$I_GET_DESC 0xE66738 is external; A5
  is loaded and not used (`a5-always=', codegen-patterns 4), the data
  area placed at .DATA 0xE8242C.

  Patterns exercised (docs/codegen-patterns.md):
  - 5.2  an 8-byte record assignment (a UID) into a field of the
         descriptor reached through the integer local held in D0:
         `movea.l D0,A0' first, as the destination's base, then the `in'
         parameter's slot into A1, the source post-incremented, the
         destination by displacement, `move.l (A1)+,(0x4c,A0); move.l
         (A1)+,(0x50,A0)' (0xE6792A..0xE67934; bead domain-pascal-fbg);
  - 3, 12.4  the local held in D0 (`move.l A0,D0', 0xE67924), as in
         tty_k_session_id.pas;
  - 7.1, 9  `if status.all <> status_$ok then return'.

  Reconstructed (not known from the original): the names; the
  descriptor record up to pgroup_uid at 0x4C (../source
  domain_os/tty/tty.h), the bytes before it as one stand-in array;
  `unused_8' and `standin' as in tty_k_flags_int.pas. }
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

procedure tty_$k_set_pgroup(in line: integer; in uid: uid_$t;
                            out status: status_$t);
var
  tty: integer32;
  unused_8: integer32;
begin
  tty := tty_$i_get_desc(line, status);
  if status.all <> status_$ok then
    return;
  tty_$desc_ptr_t(tty)^.pgroup_uid := uid;
end;
