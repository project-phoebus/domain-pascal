{ tty_$i_xmit_char 0xE1B3B4..0xE1B3CE (0x1A bytes, WIRED_PROC; tools/
  imgdis.sh E1B3B4 1A) and TTY_$K_SET_SESSION_ID, TTY_$K_INQ_SESSION_ID
  0xE67986..0xE67A02 (0x7C bytes, the default code section .TEXT): a
  fragment of two of the TTY module's three code sections (bead
  domain-pascal-xdh.6; README.md, "Fragments of several code sections").
  conform.py compiles it with `--fragment-origin=WIRED_PROC=0xE1B3B4
  --fragment-origin=.TEXT=0xE67986' and links each code section at its
  own origin.  Reconstructed from ../source's Domain Pascal translation
  (domain_os/tty/i_xmit_char.pas and k_pgroup.pas at ../source 3898a2ce,
  by Claude Opus 5.5), reduced to what the routines touch; tools/
  srcsurvey.py reports tty_$i_xmit_char byte-identical in context.
  Semantics: ../source domain_os/tty/i_xmit_char.c, k_pgroup.c.

  Patterns exercised (docs/codegen-patterns.md):
  - 11     a unit in two code sections, each a fragment at its own origin
           (WIRED_PROC at 0 mod 4, .TEXT at 2 mod 4 with its pad);
  - 1.6    tty_$i_xmit_char's `bsr.w' to tty_$i_put_chars 0xE1B00A, a
           module-local routine of its own section outside the unit
           (`module-routines=TTY_$I_PUT_CHARS=0xE1B00A', WIRED_PROC:
           ...), not popped before `unlk' (1.1); two word constants
           merged into `move.l #0x1000c,-(SP)' (1.4); the by-value
           character's slot passed by address (`pea (0xc,A6)');
  - 3, 12.4, 7.1, 9  TTY_$K_SET/INQ_SESSION_ID as in tty_k_session_id.pas
           (A5 loaded and not used, `a5-always=', codegen-patterns 4; the
           data area placed at .DATA 0xE8242C).

  Reconstructed (not known from the original): the names; tty_$i_put_chars'
  heading (../source's: `var' descriptor, `univ' one-char buffer, count
  and avail_hint by value, `internal'); the call as `discard' of the
  function; the descriptor record up to session_id at 0x54 (../source
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
  tty_$uword_t = 0..65535;
  tty_$chars_t = array[1..1] of char;
const
  status_$ok = 0;
var
  standin: integer32;

function tty_$i_get_desc(line: integer; out status: status_$t):
  integer32; options(extern, val_param, a0_return);

{ WIRED_PROC 0xE1B00A, outside the unit }
function tty_$i_put_chars(var tty: tty_$desc_t; in buf: univ tty_$chars_t;
                          count: tty_$uword_t; avail_hint: integer):
                          tty_$uword_t;
  internal; extern;

[section(wired_proc, wired_data)]
procedure tty_$i_xmit_char(var tty: tty_$desc_t; ch: char); internal;
begin
  discard(tty_$i_put_chars(tty, ch, 1, 12));        { 0x00E1B3B8 }
end;

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
