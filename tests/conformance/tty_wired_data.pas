{ tty_$i_lock and tty_$i_unlock, module-local at 0xE1AED0 and 0xE1AEE4
  (0x28 bytes; tools/imgdis.sh E1AED0 28), the first two routines of
  TTY's WIRED_PROC segment, with TTY's whole WIRED_DATA block 0xE2DDB4
  (0x28 bytes; `D E2DDB4 TTY' in the map) as initial values (bead
  domain-pascal-63t).  Reconstructed from ../source's Domain Pascal
  translation (domain_os/tty/i_lock.pas, i_unlock.pas, i_word_erase.pas,
  i_set_raw_mode.pas and tty.pas, "Data", at ../source 3898a2ce, by
  Claude Opus 5.5), reduced to what the unit needs.  Semantics: ../source
  domain_os/tty/i_lock.c, i_unlock.c.  The routines do not use the data
  and load no A5 (codegen-patterns 4, used mode).

  The data block, image bytes (`.moddata.TTY.WIRED_DATA', compared
  whole):
    +0x00  27 zero bytes, 01 at +0x1B, 00 00, 37 at +0x1E, 00: the word
           separators [BS, HT, LF, FF, CR, ' '] as a `set of char', 256
           members, member k in byte (255 - k) div 8, bit k mod 8 (ref
           3.9.3, Figure 3-6): ' ' (32) is bit 0 of +0x1B; 8, 9, 10, 12, 13
           are bits 0, 1, 2, 4, 5 of +0x1E (tty_$i_word_erase's static in
           ../source; here `word_seps', a stand-in at the unit level)
    +0x20  0000001F  [0..4], the output flags raw mode clears
    +0x24  0000003C  [2..5], the input flags raw mode clears (both
           tty_$i_set_raw_mode's statics in ../source; here stand-ins at
           the unit level, `set of 0..31' for Apollo's flag enumerations)
  So the block exercises: a 32-byte set of char given with `chr' and a
  character, sets given as ranges (ref 3.9.2), longword-sized sets after
  it, and a named data section with contents (`.moddata.<UNIT>.<SECT>').

  Patterns exercised in the code (docs/codegen-patterns.md):
  - 2     `link #-4', the slot of a routine that is not global;
  - 1.3, 1.4  a word constant by value after the pad (`subq.l #2,SP;
          move.w #3,-(SP)'); the last call not popped (1.1).

  Reconstructed (not known from the original): the names (the C tree's
  and ../source's); the code and data sections given by the module
  heading (ref 7.1.1), not by `[section(wired_proc, wired_data)]' on
  each routine as in ../source, so that the stand-ins, declared at the
  unit level, go to WIRED_DATA as the routines' statics do there; the
  descriptor record reduced to an opaque field. }
module tty, wired_proc, wired_data;
type
  tty_$desc_t = record
    line_id: integer32;
  end;
const
  tty_$lock_id = 3;

var
  { WIRED_DATA +0x00 }
  word_seps: set of char := [chr(8), chr(9), chr(10), chr(12), chr(13), ' '];
  { WIRED_DATA +0x20, +0x24 }
  raw_output_flags: set of 0..31 := [0..4];
  raw_input_flags: set of 0..31 := [2..5];

procedure ml_$lock(resource_id: integer); options(val_param, extern);
procedure ml_$unlock(resource_id: integer); options(val_param, extern);

procedure tty_$i_lock(var tty: tty_$desc_t); internal;
begin
  ml_$lock(tty_$lock_id);
end;

procedure tty_$i_unlock(var tty: tty_$desc_t); internal;
begin
  ml_$unlock(tty_$lock_id);
end;
