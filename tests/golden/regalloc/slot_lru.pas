{ Golden: codegen-patterns 12.4 R4(a), R4(b) (bead domain-pascal-ei42).
  Slot loads kept apart from earlier ones run out of address registers:
  the next one takes the register written least recently, which is also
  the one whose value died first, where every free register's last value
  is a dead parameter slot load (19 of 19 loads before the first branch,
  call or label; argument lists MST_$MAP_AT 0xE42F9C, MSG_$SENDI
  0xE59AE0..0xE59AF2).  dpc applies it only in that straight-line prefix
  (MSG_$SEND_HW 0xE59B6E..0xE59B8C fits it after a call, still refused).
  - copies: seven value parameters copied at entry (section 1.2), in A0,
    A1, A2, A3, A4, then A0 and A1 again (MSG_$SEND 0xE59A0A..0xE59A38);
  - args: seven `in' words read once by the next push of one argument
    list, right to left, in A0..A4, then A0 and A1 (MSG_$SENDI
    0xE59AB6..0xE59AF2, A0..A4 then A0, A1, A2, A3). }
module slru;
var
  w: extern integer;

procedure slru_$take(a, b, c, d, e, f, g: integer);
  options(extern, val_param);

procedure slru_$copies(a, b, c, d, e, f, g: integer);
begin
  slru_$take(a, b, c, d, e, f, g);
end;

procedure slru_$args(in a, b, c, d, e, f, g: integer);
begin
  slru_$take(a, b, c, d, e, f, g);
end;
