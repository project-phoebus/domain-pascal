{ tty_$i_set_dfl_funcs, module-local at 0xE6726E (0x24 bytes;
  tools/imgdis.sh E6726E 24), with TTY's whole .DATA block 0xE8242C
  (0x2C bytes; `D E8242C TTY' in the map) as initial values (bead
  domain-pascal-63t).  Reconstructed from ../source's Domain Pascal
  translation (domain_os/tty/i_set_dfl_funcs.pas, k_break.pas and the
  types of tty.ins.pas, at ../source 3898a2ce, by Claude Opus 5.5),
  reduced to what the unit needs.  Semantics: ../source
  domain_os/tty/i_set_dfl_funcs.c.  tty_$i_set_funcs 0xE6720E, the
  routine before it, is outside the unit (`module-routines=', reached by
  `bsr.s', 0xE67288); the routine loads its own A5 (`a5-always=', an
  internal routine called from other code sections, codegen-patterns 4).

  The data block, image bytes (`.moddata.TTY', compared whole):
    +0x00  0007 0009 0008 000C 000B 0010 000F 000E 0001 0000 0002 0003
           0004 0005 0006 000D 000A 0000   the class of each function slot
           (tty_$i_set_funcs's static in ../source; here `func_classes',
           a stand-in at the unit level, as that routine is not in the
           unit)
    +0x24  0001FFFF  [tty_$func_erase..tty_$func_replay], this routine's
           static, pushed by value (`move.l (0x24,A5),-(SP)', 0xE67280)
    +0x28  000000D0  [tty_$func_eol, tty_$func_cr, tty_$func_nl] (members
           4, 6, 7; TTY_$K_SET_INPUT_BREAK_MODE's static in ../source;
           here `line_end_funcs', a stand-in at the unit level)
  So the block exercises: a word array initialiser (ref 3.11.1.2), a set
  of a 32-value enumeration given as a range and as members (ref 3.9.2,
  big-endian bit string, ref 3.9.3 Figure 3-6), a routine's `static'
  variable (ref 7.2) between unit-level ones, laid out in declaration
  order; the block is one `.moddata' section with contents.

  Patterns exercised in the code (docs/codegen-patterns.md):
  - 4     an `internal' routine that loads A5 (`pea (A5); lea .l,A5' and
          the restore from (-8,A6)), `a5-always=';
  - 1.3, 1.4  a byte by value (`move.b (0xc,A6),-(SP)' after the pad
          `subq.l #2,SP'), a static set by value from A5, an `in out'
          record passed on by its slot (`move.l (8,A6),-(SP)');
  - 1.1   the last call not popped (`unlk' follows).

  Reconstructed (not known from the original): the names (the C tree's
  and ../source's); the descriptor record reduced to an opaque field (the
  routine only passes its address on); the stand-ins' names. }
module tty;
type
  tty_$func_t = (
      tty_$func_erase, tty_$func_erase_word, tty_$func_kill, tty_$func_eof,
      tty_$func_eol, tty_$func_tab, tty_$func_cr, tty_$func_nl,
      tty_$func_quit, tty_$func_int, tty_$func_susp, tty_$func_dsusp,
      tty_$func_quote, tty_$func_stop, tty_$func_start, tty_$func_oflush,
      tty_$func_replay, tty_$func_reserved_18, tty_$func_reserved_19,
      tty_$func_reserved_20, tty_$func_reserved_21, tty_$func_reserved_22,
      tty_$func_reserved_23, tty_$func_reserved_24, tty_$func_reserved_25,
      tty_$func_reserved_26, tty_$func_reserved_27, tty_$func_reserved_28,
      tty_$func_reserved_29, tty_$func_reserved_30, tty_$func_reserved_31,
      tty_$func_reserved_32);
  tty_$func_set_t = set of tty_$func_t;
  tty_$desc_t = record
    line_id: integer32;
  end;

var
  { .DATA +0x00 }
  func_classes: array[tty_$func_erase..tty_$func_reserved_18] of integer := [
    16#07, 16#09, 16#08, 16#0C, 16#0B, 16#10, 16#0F, 16#0E, 16#01,
    16#00, 16#02, 16#03, 16#04, 16#05, 16#06, 16#0D, 16#0A, 0];

procedure tty_$i_set_funcs(in out tty: tty_$desc_t; funcs: tty_$func_set_t;
                           use_dfl: boolean); internal; extern;

procedure tty_$i_set_dfl_funcs(in out tty: tty_$desc_t; use_dfl: boolean);
  internal;
var
  { .DATA +0x24 }
  tty_$i_dfl_func_enable_mask: static tty_$func_set_t :=
    [tty_$func_erase..tty_$func_replay];
begin
  tty_$i_set_funcs(tty, tty_$i_dfl_func_enable_mask, use_dfl);
end;

var
  { .DATA +0x28 }
  line_end_funcs: tty_$func_set_t :=
    [tty_$func_eol, tty_$func_cr, tty_$func_nl];
