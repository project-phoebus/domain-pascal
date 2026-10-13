{ TERM_$SEND_KBD_STRING, 0xE1AAFC..0xE1AB28 (0x2A bytes of code and the
  word cell 0xE1AB26; tools/imgdis.sh E1AAFC 2C), the last routine of the
  OS_TERM code segment (0xE1A9D4, size 0x154), so a module-tail fragment
  (`dpc --fragment-origin 0xE1AAFC --fragment-ends-module').  Semantics:
  ../source/domain_os/term/send_kbd_string.c.  Externals (SAU2 map):
  KBD_$PUT 0xE1CEAC.  The routine it calls with `bsr.s' at 0xE1AB20 is
  the module-local routine at 0xE1AAA8, which the map does not name
  (`TERM_$STATUS_CONVERT' in ../source): it lies outside the unit and is
  given by the manifest as `module-routines=STATUS_CONVERT=0xE1AAA8'
  (tests/conformance/README.md, "Routines outside the unit").

  Patterns exercised (docs/codegen-patterns.md):
  - 1.5  constants passed by address (bead domain-pascal-5aw.9): the two
         word constants 0 for KBD_$PUT's first two parameters share one
         cell in the code section, after the routine's `rts' (0xE1AB26,
         `0000'), passed with `pea (0x18,PC)' (0xE1AB0C) and, for the
         second, `move.l (SP),-(SP)' (0xE1AB10); the cell is the module's
         last word, so the module-end padding adds nothing (OS_TERM ends
         at 0xE1AB28);
  - 1.5  by-address parameters passed on, `move.l (d,A6),-(SP)';
  - 1.1  `lea (0x14,SP),SP' after the first call, no pop after the last.

  Reconstructed (not known from the original): every name; KBD_$PUT's
  parameter list read off the pushes (two word values by address, the
  string and its length passed on, a status); `in' for the parameters
  passed on (the routine neither copies nor assigns them, ref 5.3.2);
  the string type. }
module os_term;
type
  status_$t = integer32;
  kbd_str_t = array[1..256] of char;

procedure kbd_$put(line: integer; in kbd: integer; in str: univ kbd_str_t; in len: integer;
                   out status: status_$t); extern;
procedure status_convert(var status: status_$t); internal; extern;

procedure term_$send_kbd_string(in str: univ kbd_str_t; in len: integer);
var status: status_$t;
begin
  kbd_$put(0, 0, str, len, status);
  status_convert(status);
end;
