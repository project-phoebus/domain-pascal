{ Golden: codegen-patterns 5.2 ("Byte indices").  Bead domain-pascal-ji9;
  survey `tools/cmpsurvey.py' (bidx).

  An index of an unsigned byte type (`char', a char subrange) is
  zero-extended to a word with `clr.w Dn; move.b x,Dn', scaled in place
  and used as a `.w' index: 21 of the 27 byte indices of compiled code
  have the `clr.w' right before the `move.b', 5 earlier, one a `clr.l'
  (tty_$i_enable_crash_func
  0xE672AA..0xE672B4 `clr.w D2; move.b D0,D2; add.w D2,D2; lea
  (0,A0,D2.w),A1; move.w #0x11,(0xa4,A1)', ch a by-value parameter held
  in D0; TTY_$K_SET_INPUT_BREAK_MODE 0xE67874 the byte read from
  memory; VTOC_$LOOKUP 0xE38FD2..0xE38FD8 `clr.w D0; move.b
  (28,A3),D0; lea (0,A5,D0.w),A0', element size 1).  None has `andi.w
  #255' or `ext.w'. }
module bidx;
type
  class_t = array[char] of integer;
  desc_t = record
    pad: array[1..16#52] of integer;
    char_class: class_t;          { +0xA4 }
    crash_char: char;
  end;
  desc_ptr_t = ^desc_t;
var
  classes: class_t;
  bytes: array[char] of char;
  longs: array['a'..'z'] of integer32;
  c: char;
  w: integer;

procedure from_memory; internal;
begin
  classes[c] := 17;
  w := classes[c];
  bytes[c] := 'x';
  longs[c] := 0;
end;

procedure in_record(var d: desc_t; ch: char);
  options(val_param);
begin
  d.char_class[ch] := 18;
  d.char_class[d.crash_char] := 17;
end;
