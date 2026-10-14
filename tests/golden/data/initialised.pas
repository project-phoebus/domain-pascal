{ codegen-patterns 11 ("Initial values"), 5.1; bead domain-pascal-63t.
  Initialised static data (ref 3.2.2, 3.5.1, 3.6.2, 3.9.2, 3.10.4,
  3.11.1, 3.13.4): one block per data section with contents,
  `.moddata.<UNIT>[.<SECT>]', zeros where no value is given.  The values
  mirror the image's: TTY's .DATA 0xE8242C (words, a set of slots
  0001FFFF, as a routine's static), WIRED_DATA 0xE2DDB4 (a `set of
  char', here in a named data section), TTY_$I_INIT's OS_INIT_DATA
  0xE351B0 (status_$t's `all', a char array).  A section with no initial
  value stays NOBITS. }
module initd;
type
  st = record case integer of 1: (all: integer32); 2: (code: integer) end;
  colour = (red, green, blue);
var
  classes: array[0..3] of integer := [7, 9, 2 of 8];
  status: array[0..1] of st := [[all := 16#120010], [all := 16#B000E]];
  name: array[1..8] of char := 'tty';
  ok: boolean := true;
  hue: colour := blue;
  p: ^integer := nil;
  plain: integer32;
  late: integer32 := -1;

procedure set_dfl(var x: integer32); internal;
var
  mask: static set of 0..31 := [0..16];
begin
  x := plain;
end;

[section(wired_proc, wired_data)]
procedure word_erase(var x: integer32);
var
  seps: static set of char := [chr(8), chr(9), chr(10), chr(12), chr(13), ' '];
begin
  x := 0;
end;

[section(os_init_proc, os_init_data)]
procedure zero_only(var x: integer32);
var
  z: static integer32;
begin
  x := z;
end;
