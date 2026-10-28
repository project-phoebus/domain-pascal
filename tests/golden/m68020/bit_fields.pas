{ codegen-patterns 14 ("Bit fields"); bead domain-pascal-vzzh.  With
  `-cpu mathchip', a packed record's field is written with `bfins' and
  read with `bfextu', the offset counted from the unit's most
  significant bit:

  - a 20-bit field in the low bits of a longword from a static:
    `move.l NODE_$ME,D0; bfins D0,(4,A0)' at offset 12, width 20 (SAU9 UID_$INIT
    0x3C43699E; the MC68010 code clears with `andi.l' and `or.l's);
  - a boolean in bit 0 of a byte from a variable: `move.b (84,A2),D1;
    lsr.b #7,D1; bfins D1,(1970,A0)' at offset 7, width 1 (SAU9 MAC_$OPEN
    0x3C40D94E);
  - a 3-bit field read: `bfextu (A0),D0' at offset 5, width 3 (SAU9 DIR_$ADD_LINKU
    0x3C457F42);
  - a boolean in bit 7 keeps the MC68010 form, `andi.b #127,M; andi.b
    #-128,Dn; or.b Dn,M' (SAU9 0x3C401C90), and a constant its `andi' /
    `ori' (SAU9 AST_$LOAD_AOTE 0x3C403AE2 `andi.w #-1024,(2,A0)'). }
module bit_fields;
type
  uid_$t = packed record
    high: integer32;
    rsvd: 0..4095;      { bits 31..20 of the second longword }
    node: 0..1048575;   { bits 19..0 }
  end;
  port_t = packed record
    pad: array[1..985] of integer;   { 0..1969 }
    f7, f6, f5, f4, f3, f2, f1: boolean;
    open: boolean;                   { 1970, bit 0 }
  end;
  entry_t = packed record
    kind: 0..31;        { bits 7..3 of the byte }
    typ: 0..7;          { bits 2..0 }
  end;
var
  uid_$nil: uid_$t;
  node_$me: extern integer32;
  mac_$port: port_t;
  mac_$want_open: boolean;
procedure uid_$init;
begin
  uid_$nil.node := node_$me;
end;
procedure mac_$open;
begin
  mac_$port.open := mac_$want_open;
end;
function dir_$entry_type(in e: entry_t): integer;
begin
  dir_$entry_type := e.typ;
end;
procedure mac_$set_f7(v: boolean); options(val_param);
begin
  mac_$port.f7 := v;
end;
procedure dir_$set_kind(var e: entry_t);
begin
  e.kind := 3;
end;
