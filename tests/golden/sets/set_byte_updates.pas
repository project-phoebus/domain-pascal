{ Golden: codegen-patterns 5.1 "Several constant members" and "One
  constant member": a word set's constant members added or removed in
  place (beads domain-pascal-wrzd, domain-pascal-fozq).  sau5 image
  (`sau5/domain_os', map `sau5/domain_os.map'):
  - hi2, hi3, hi2d: every member in the high byte (the byte at the
    set's address): one `ori.b'/`andi.b' of that byte, mt_$queue_request
    0xE759CE `ori.b #-0x60,(0x4,A3)' (`+ [15, 13]'), init_unit 0xE75C46
    `andi.b #-0x39,(0x44,A4)' (`- [13, 12, 11]'), MT_$WAIT 0xE76A78
    `andi.b #-0x19,(0x44,A2)' (`- [12, 11]');
  - lo3: every member in the low byte stays word-wide (sau2 TTY_$I_RCV
    0xE1B98E `andi.w #-0xe1,(8,A2)', `- [5, 6, 7]');
  - both: members in both bytes, word-wide (sau2 PROC2_$INIT 0xE3065A
    `ori.w #0x180', type not known);
  - absolute: one member of an absolute word set keeps its number
    unreduced (MT_$WAIT 0xE76A1A `bclr #13,(0xe90cc0).l', MT_$SCB+0x32;
    PBU2_$ASSOC 0xE58CCA `bclr #15'; MMAP_$INIT 0xE330AE `bset #15');
    a member below 8 is on the low byte; several high-byte members are
    the byte `ori.b' (no absolute site: the rule above);
  - long: a longword set's members (no site): unchanged, `ori.l' and
    `bset #k mod 8' on the member's byte. }
module set_byte_updates;
type
  flags_t = set of 0..15;
  pb_t = record
    next: ^pb_t;                     { 0x00 }
    flags: flags_t;                  { 0x04 }
  end;
  pb_ptr = ^pb_t;
  scb_t = record
    pad: array[1..25] of integer;    { 0x00 }
    flags: flags_t;                  { 0x32 }
  end;
var
  s: extern scb_t;
  l: extern set of 0..31;

procedure hi2(in pb: pb_ptr);
begin
  pb^.flags := pb^.flags + [15, 13];
end;

procedure hi3(in pb: pb_ptr);
begin
  with pb^ do
    flags := flags - [13, 12, 11];
end;

procedure hi2d(in pb: pb_ptr);
begin
  pb^.flags := pb^.flags - [12, 11];
end;

procedure lo3(in pb: pb_ptr);
begin
  pb^.flags := pb^.flags - [5, 6, 7];
end;

procedure both(in pb: pb_ptr);
begin
  pb^.flags := pb^.flags + [7, 8];
end;

procedure absolute;
begin
  s.flags := s.flags - [13];
  s.flags := s.flags + [15];
  s.flags := s.flags + [3];
  s.flags := s.flags + [14, 9];
end;

procedure long;
begin
  l := l + [30, 29];
  l := l - [20];
end;
