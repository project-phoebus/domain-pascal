{ Integer subranges and integer types as each other's `var', `out' and
  `in out' arguments (bead domain-pascal-4tm; ref Appendix D, ISO
  6.6.3.5/6.6.3.6, p. D-3: "Domain Pascal treats integer and subrange of
  integer as identical"; reference-clarifications Q38).  codegen-patterns
  1.2: by default every argument is passed by address, `pea (d,A6)' for a
  local (TONE_$TIME 0xE1732C `pea (-8,A6)' to TIME_$WAIT).

  - sa_$exact: each argument of exactly the parameter's type, the
    reference shape.
  - sa_$mixed: the same calls with an argument of the other type of the
    same size (integer16 for 0..32767, 0..127 for integer16, integer32 for
    0..2147483647, 0..32767 for a routine pointer's integer formal, as
    ../source net_io/init.pas `driver^.start^(port^.socket, status)'):
    the same `pea' of the argument's own address, no copy.
  - sa_$consts: an integer constant outside an integer subrange (Appendix
    D, ISO 6.4.6, p. D-3; Q38): the low bytes are stored, as dmwin.pas
    `connv[ios_$stderr] := ios_$no_stream' (16#7FFF into 0..127), and
    pushed by value under `val_param' (40000 for 0..32767).  No image
    site; the stores are codegen-patterns 13's, the push 1.3's.
  - sa_$reg: the same into a register-held local (codegen-patterns
    12.5): `move.w #<low word>,Dn'.
  - sa_$more: an out-of-range constant passed by address (its pooled
    cell holds the low word), a byte `val_param' push (300 for a [byte]
    0..255: the low byte in the high half of the slot), and a packed
    bit-field store (17 into a 4-bit field: its low bits, `andi.b;
    ori.b').  The static initial values `sa_w' (70000 into 0..32767) and
    `sa_pr' (16 into a 4-bit field) keep their low bytes and bits. }
module sa;
type
  pinteger = 0..32767;
  linteger = 0..2147483647;
  id_t = 0..127;
  bid_t = [byte] 0..127;
  sendp_t = ^procedure(var socket: integer; out st: integer32);
  ubyte_t = [byte] 0..255;
  nib_t = packed record a: 0..15; b: 0..15 end;
var
  sa_w: pinteger := 70000;
  sa_pr: nib_t := [a := 16, b := 2];
  sa_r: nib_t;
procedure sa_$p16(var x: integer16); extern;
procedure sa_$ppi(var x: pinteger); extern;
procedure sa_$pli(out x: linteger); extern;
procedure sa_$pio(in out x: id_t); extern;
procedure sa_$val(x: pinteger; var b: bid_t); val_param; extern;
procedure sa_$use(x: integer); val_param; extern;
procedure sa_$byref(x: pinteger); extern;
procedure sa_$byte(x: ubyte_t); val_param; extern;

procedure sa_$exact(sp: sendp_t);
var i16: integer16; pi: pinteger; li: linteger; id: id_t; st: integer32;
begin
  sa_$p16(i16);
  sa_$ppi(pi);
  sa_$pli(li);
  sa_$pio(id);
  sp^(i16, st);
end;

procedure sa_$mixed(sp: sendp_t);
var i16: integer16; pi: pinteger; i32: integer32; id: id_t; st: integer32;
begin
  sa_$p16(id);
  sa_$ppi(i16);
  sa_$pli(i32);
  sa_$pio(i16);
  sp^(pi, st);
end;

procedure sa_$consts;
var conn: array[1..3] of id_t; b: array[1..2] of bid_t; w: pinteger;
begin
  conn[2] := 16#7FFF;
  b[2] := 16#7F7E;
  w := 70000;
  sa_$pio(conn[1]);
  sa_$ppi(w);
  sa_$val(40000, b[1]);
end;

procedure sa_$reg(n: integer); val_param;
var w: pinteger; i: integer;
begin
  w := 70000;
  for i := 1 to n do begin
    sa_$use(w);
    w := w + 1;
  end;
end;

procedure sa_$more;
begin
  sa_$byref(70000);
  sa_$byte(300);
  sa_r.a := 17;
end;
