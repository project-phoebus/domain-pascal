{ codegen-patterns 7.1 ("One bit"), 8 (`odd'), 5.1; bead
  domain-pascal-3xwx.  One-bit tests of integers:

  - a word or longword variable in memory: loaded whole into a data
    register, `btst #k' on it (tty_$i_put_chars 0xE1B01E `move.w
    (8,A0),D1; btst #5,D1; beq.s', 0xE1B0E6 `move.l (12,A0),D1; btst
    #3,D1; beq.s'; the module-local at 0xE1BCFC, ../source
    tty_$i_store_parity, at 0xE1BD0A `move.l (20,A0),D0; btst #12,D0');
  - `odd' of one: `lsr.b #1' and the carry (tty_$i_put_chars 0xE1B0F6
    `move.l (12,A0),D1; lsr.b #1,D1; bcc.s');
  - a register-held value: `btst #k' on its register (pacct_$compress
    0xE5A9DE); a computed one: on the register that holds it
    (DIR_$SET_PROTECTION 0xE522D4 `lsr.w #4,D1; btst #4,D1; bne').
  The set operand is read through a type transfer (ref "Type Transfer
  Functions", p. 4-197). }
module one_bit;
type
  flags16 = set of 0..15;
  flags32 = set of 0..31;
  desc = record
    line: integer32;
    pad: integer32;
    state: flags16;
    pad2: integer;
    oflags: flags32;
  end;
var
  r: integer;

function puts(var d: desc; count: integer): integer;
  options(val_param, internal);
begin
  if (integer(d.state) & 16#20) <> 0 then begin
    puts := count;
    return;
  end;
  if ((integer32(d.oflags) & 8) = 0) or else (r <> 0) then
    if odd(integer32(d.oflags)) then
      r := 1
    else
      r := 2;
  if (integer32(d.oflags) & 2) <> 0 then
    r := 3;
  puts := 0;
end;

procedure held(v: integer32); options(val_param);
begin
  if (v & 16#1000) <> 0 then r := 4;
  if (rshft(v, 4) & 16#10) <> 0 then r := 5;
end;
