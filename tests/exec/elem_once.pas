{ Execution test: element addresses of other units' arrays read once
  (codegen-patterns 5.2, "The element's address as a value", read once;
  bead domain-pascal-1ibr): `lea (d,A0,Dx),A1' where A1 is free and
  clean (the module-local 0xE1C204 at 0xE1C242, an address stored;
  NETBUF_$RTN_DAT 0xE0F082, an element at a displacement beyond a byte,
  `lea (0,A0,Dx),A1' then `(d,A1)'), `lea (d,A0,Dx),A0' where A1 holds
  an earlier statement's address (SIO_$INIT 0xE32CD6).  Elements of 12
  bytes (1-based) and 288 bytes (the field at -284), word indices, read
  and written through, the stored address read back.
  The constants stored into elements lie outside the range of `moveq': a
  smaller longword is stored from a register (codegen-patterns 13), set
  up before or after the element's base: not seen, refused (bead
  domain-pascal-53xi).

  link: elem_once_data.mod.pas
  call: EB_$FILL_ONCE
  call: EO_$GET w:3 -> l
  call: EO_$PUT w:2
  call: EO_$GET w:2 -> l
  call: EO_$ADDR w:4 -> l
  call: EO_$TWO w:2 w:5 -> l
  call: EO_$AFTER w:6 -> l
  expect:
    => -3
    => 777
    => 42
    => 524
    => 63
}
module elem_once;
%include 'exec.ins.pas';

type
  rec12 = record a, b, c: integer32; end;
  rec288 = record a: integer32; z: integer32; pad: array[1..70] of integer32; end;
  rec12_ptr = ^rec12;
var
  eo_$recs: extern array[1..6] of rec12;
  eo_$quits: extern array[1..6] of rec12;
  eo_$big: extern array[1..4] of rec288;
  p, q: rec12_ptr;

procedure eo_$fill; extern;

procedure eb_$fill_once;
begin
  eo_$fill;
end;

{ an element beyond a byte read: `lea (0,A0,Dx),A1; move.l (-284,A1),D0' }
function eo_$get(i: integer): integer32;
  options(val_param);
begin
  eo_$get := eo_$big[i].z;
end;

{ written (read back by eo_$get) }
procedure eo_$put(i: integer);
  options(val_param);
begin
  eo_$big[i].z := 777;
end;

{ an address stored, then read through }
function eo_$addr(i: integer): integer32;
  options(val_param);
begin
  p := addr(eo_$recs[i]);
  eo_$addr := p^.b;
end;

{ two addresses stored: the second with A1 dirty, into A0 itself }
function eo_$two(i, j: integer): integer32;
  options(val_param);
begin
  p := addr(eo_$recs[i]);
  q := addr(eo_$quits[j]);
  eo_$two := p^.a + q^.c;
end;

{ after a call: A1 last written by the call }
function eo_$after(i: integer): integer32;
  options(val_param);
begin
  eo_$fill;
  p := addr(eo_$recs[i]);
  eo_$after := p^.c;
end;
