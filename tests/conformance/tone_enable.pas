{ TONE_$ENABLE, 0xE1ACE8..0xE1AD18 (0x2E bytes and the module-end filler
  0x2048 at 0xE1AD16; tools/imgdis.sh E1ACE8 30), the last routine of the
  OS_TERM_INIT code segment (0xE1AB28, size 0x1F0), so a module-tail
  fragment.  Semantics: ../source/domain_os/tone/enable.c.  Externals
  (SAU2 map): SIO2681_$TONE 0xE1D172.  The data area `.bss.moddata.
  OS_TERM_INIT' is placed at the OS_TERM_INIT data segment 0xE2C9F0
  (`lea (0xe2c9f0).l,A5'), which ../source calls TERM_$DATA; the SIO2681
  channel-A record TONE_$CHANNEL is at +0x1268 in it.

  Patterns exercised (docs/codegen-patterns.md):
  - 1.5  an expression passed by address (bead domain-pascal-5aw.9):
         `addr(tone_$channel)' for SIO2681_$TONE's pointer value
         parameter is stored into a frame temporary and the temporary's
         address pushed, `lea (0x1268,A5),A0; move.l A0,(-8,A6); pea
         (-8,A6)' (0xE1ACFC..0xE1AD04), right before its push, after the
         two arguments to its right;
  - 2    the temporary below the locals: `link #-8', status at -4,
         the temporary at -8;
  - 4    A5 saved with `pea (A5)', loaded, restored;
  - 1.1  no pop after the last call; 7: the module-end 0x2048.

  Reconstructed (not known from the original): every name; the
  parameter types (an untyped pointer for the channel; `in' for the
  enable flag passed on, a boolean: TONE_$TIME passes TONE_$ENABLE the
  cells 0xE1735A `ff00' and 0xE1735C `0000'); the 0x1268 bytes before
  TONE_$CHANNEL. }
module os_term_init;
type
  status_$t = integer32;
  filler_t = array[1..16#1268] of char;
var
  filler: filler_t;
  tone_$channel: integer32;

procedure sio2681_$tone(chan: univ_ptr; in enable: boolean; out status: status_$t); extern;

procedure tone_$enable(in enable: boolean);
var status: status_$t;
begin
  sio2681_$tone(addr(tone_$channel), enable, status);
end;
