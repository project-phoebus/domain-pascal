{ SUMA_$INIT, 0xE33224, 0x26 bytes of code and the filler `movea.l A0,A0'
  (0xE3324A) to the segment's end: map `I E33224 BITPAD size = 28' in
  OS_INIT_PROC (tools/imgdis.sh E33224 28).  Semantics:
  ../source/domain_os/suma/init.c (the Pascal rendering suma/init.pas).

  The routine is in OS_INIT_PROC; the BITPAD module's only data block is
  in WIRED_DATA (`D E2DD88 BITPAD size = 2C'), so the variables it sets
  are another data section's (codegen-patterns 11, "Data per section";
  bead domain-pascal-a2g): no A5, the block's address as an immediate,
  `movea.l #0xE2DD88,A0' (`207c', 0xE33228), then four stores off A0,
  one load for the four statements (no call or branch between them).

  Patterns exercised (docs/codegen-patterns.md):
  - 11   another data section's block, `movea.l #block,A0; (d,A0)'
         (0xE33228..0xE33240);
  - 1.5  `addr' of another unit's variable as an immediate, `move.l
         #TERM_$TPAD_BUFFER,(0x24,A0)' (0xE3322E);
  - 2    `link.w A6,#0', nothing saved.

  Reconstructed (not known from the original): the variable names and
  types are ../source's (suma/suma.pas, suma/suma.ins.pas), the sample
  record reduced to `id_flags' at +0x0A and fillers (16 bytes); `tpad_buffer_t' is an opaque stand-in; the
  section names as identifiers (the map's, folded; OS_INIT_DATA as the
  routine's data section is the map's pairing of OS_INIT_PROC, where
  BITPAD allocates nothing). }
module bitpad, wired_proc, wired_data;
type
  suma_sample_t = record
    time: integer32;
    x, y, z: integer;
    id_flags: char;                         { +0x0A }
    buttons: char;
    spare: integer32;
  end;
  tpad_buffer_t = record
    head, tail: integer;
  end;
  tpad_buffer_ptr_t = ^tpad_buffer_t;
var
  term_$tpad_buffer: extern tpad_buffer_t;
  suma_$last_time: integer32;             { +0x00 }
  suma_$prev: suma_sample_t;              { +0x04 }
  suma_$cur: suma_sample_t;               { +0x14 }
  suma_$tpad_buffer: tpad_buffer_ptr_t;   { +0x24 }
  suma_$rcv_state: integer;               { +0x28 }
  suma_$threshold: 0..65535;              { +0x2A }

[section(os_init_proc, os_init_data)]
procedure suma_$init;
begin
  suma_$tpad_buffer := addr(term_$tpad_buffer);
  suma_$rcv_state := 0;
  suma_$cur.id_flags := chr(1);
  suma_$threshold := 16#200;
end;
