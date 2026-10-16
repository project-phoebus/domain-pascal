{ Golden: codegen-patterns 13, "Stores of constants": a longword constant
  in the range of `moveq' (-128..127, not 0) is stored from a register,
  `moveq #n,Dk; move.l Dk,<mem>' (bead domain-pascal-53xi;
  tools/moveqsurvey.py: 44 in straight-line code against 3 `move.l #n',
  all three in a run of constants).  Sites: a frame local,
  DISK_$PV_MOUNT 0xE6C9F4 `moveq #18,D0; move.l D0,(-20,A6)'; another
  unit's variable, MMAP_$INIT 0xE31B6E `moveq #100,D0; move.l D0,abs.l';
  one constant stored three times from one register, NETWORK_$INIT
  0xE2F7C2 (NETWORK_$RQST_WAIT[0..2] := 1); a run of different constants
  through a pointer in D0, D1, D2, TTY_$I_INIT 0xE33262..0xE3326E (the
  record's offsets 8..28 as there); `move.l #n' below the next
  statement's zero, MAC_OS_$INIT 0xE2F55E (`version := 1; config := 0',
  a word above) and the module-local 0xE311C8 (`version := 7; net_port
  := 0', a longword above); the `moveq' below the previous statement's
  word, EC2_$INIT_S 0xE309C6..0xE309CE `move.w #1,(3740,A1); moveq
  #1,D0; move.l D0,(3736,A1)'.  Word constants stay `move.w #n' (639
  sites).  A `device' longword keeps `move.l #n' (no site).  The frame
  and the registers beyond those named are dpc's rules (codegen-patterns
  12), not compared with those routines. }
module smallk;
type
  tty_desc = record
    pad0: integer32;
    pad1: integer32;
    state: integer;          { 8 }
    pad2: integer;
    b: integer32;            { 12 }
    f16: integer32;          { 16 }
    a: integer32;            { 20 }
    f24: integer32;          { 24 }
    c: integer32;            { 28 }
  end;
  port_info = record
    version: integer32;
    config: integer;
    mtu: integer;
  end;
  hint_hdr = record
    version: integer32;
    net_port: integer32;
  end;
var
  waits: array[0..3] of integer32;
  mmap_x: extern integer32;
  dev: [device] integer32;

procedure use_l(var x: integer32); extern;

procedure frame_local;
var
  x: integer32;
begin
  x := 18;
  use_l(x);
end;

procedure other_unit;
begin
  mmap_x := 100;
end;

procedure same_constant;
begin
  waits[0] := 1;
  waits[1] := 1;
  waits[2] := 1;
end;

procedure run_apart(var d: tty_desc);
begin
  d.state := 0;
  d.a := 41;
  d.b := 2;
  d.c := 35;
  d.f24 := 0;
  d.f16 := 0;
end;

procedure below_a_word(var p: port_info);
begin
  p.version := 1;
  p.config := 0;
end;

procedure below_a_long(var h: hint_hdr);
begin
  h.version := 7;
  h.net_port := 0;
end;

procedure below_a_word_first(var p: port_info);
begin
  p.config := 1;
  p.version := 1;
end;

procedure device_long;
begin
  dev := 5;
end;
