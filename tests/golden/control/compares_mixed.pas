{ Golden: codegen-patterns 7.1 ("Two variables", "Mixed widths and
  signs", "Records"), 8 (widening), 12.4 R11 (the order of the two
  widenings).  Bead domain-pascal-aaj; survey `tools/cmpsurvey.py'.

  - Two variables in memory: the left one loaded into a new data
    register, `cmp.<s>' of the right one (TTY_$K_SET_FLAG 0xE67462
    `move.w (0x2cc,A2),D0; cmp.w (0x2ca,A2),D0; beq.b'; ACL_$CLEAR_SUPER
    0xE4701A; 358 such pairs against 4 with both loaded), a char with
    `cmp.b' and an unsigned branch.
  - A signed and an unsigned word (`integer' against `0..65535'): both
    widened to longwords, `ext.l' and `clr.l; move.w', then `cmp.l
    Dr,Dl' and a signed branch (TTY_$K_PUT 0xE67C42..0xE67C4A, the two
    widenings interleaved by the scheduler as there; tty_$i_put_chars
    0xE1B050..0xE1B058 the other way round).  So -1 < 40000 holds (ref
    4.3.1 p. 4-5; p. 9-32, message 250).
  - A word against a longword: the word widened, `cmp.l <ea>,Dn'
    (0xE179BC).
  - Records, `=' and `<>' (ref Table 4-1, p. 4-2): left address in A0,
    right in A1 (another unit's with `movea.l #abs'), `moveq #n-1', then
    8 bytes as two `cmpm.l' with a `bne' between them
    (TTY_$K_SIMULATE_TERMINAL_INPUT 0xE1C1AC; PACCT_$ON 0xE5A9A4), 6
    bytes as a `cmpm.w' loop (TIME_$Q_SCAN_QUEUE 0xE16EC2), 32 bytes as a
    `cmpm.l' loop (DISK_IO 0xE3D84E), `dbne' closing the loop; as a
    value, `sne'/`seq' into the count's register (PACCT_$ON 0xE5A9C0,
    0xE57786). }
module cmpm;
type
  pinteger = 0..65535;
  uid_t = record high, low: integer32; end;
  clock_t = record high: integer32; low: pinteger; end;
  sid_t = record ids: array[1..8] of integer32; end;
var
  w, w2: integer;
  l: integer32;
  u: pinteger;
  c, c2: char;
  r: integer;
  a, b: uid_t;
  k1, k2: clock_t;
  s1, s2: sid_t;
  uid_$nil: extern uid_t;

procedure two_vars; internal;
begin
  if w <> w2 then r := 1;
  if w < w2 then r := 2;
  if l >= l then r := 3;
  if c > c2 then r := 4;
end;

procedure mixed; internal;
begin
  if w < u then r := 1;
end;

procedure mixed2; internal;
begin
  if u > w then r := 2;
end;

procedure word_long; internal;
begin
  if w <= l then r := 3;
end;

procedure records; internal;
begin
  if a = b then r := 1;
  if a <> uid_$nil then r := 2;
  if k1 = k2 then r := 3;
  if s1 <> s2 then r := 4;
end;

function uid_set: boolean; internal;
begin
  uid_set := a <> uid_$nil;
end;

function same_clock: boolean; internal;
begin
  same_clock := k1 = k2;
end;
