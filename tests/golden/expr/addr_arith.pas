{ Golden: `addr(x)' as a value in `+' and `-' (ref "Addr", p. 4-13;
  "Type Transfer Functions", p. 4-197; codegen-patterns 5.2, "`addr(x)`
  as a value"; bead domain-pascal-j28d):
  - an address inside another unit's record minus the record's address,
    passed by value as a word: `lea (sym+d).l,A0; move.l A0,D0; sub.l
    #sym,D0' with the immediate-EA encoding `90bc' (NET_$OPEN
    0xE5A1B8..0xE5A1C6 and its four neighbours; ACL_$INIT 0xE310AA);
  - another unit's address added to a computed value and subtracted
    from a variable: `add.l #sym,Dn' (`d0bc' + the register, 0xE0F148
    `andi.l #1023,D1; add.l #AST_$COPY_BUFF,D1'), `sub.l #sym,Dn'
    (0xE09E6A `move.l (8,A2),D3; sub.l #PIT_PAGES,D3');
  - two frame addresses subtracted: the subtrahend's `lea' first
    (ASKNODE_$SERVER 0xE65DA0 `lea (-592,A6),A0; lea (-582,A6),A1;
    move.l A1,D1; sub.l A0,D1');
  - `integer32(addr(x))' stored and passed by value as `addr(x)' is
    (NETBUF_$INIT 0xE2F63C `move.l #sym,<dst>'; SOCK_$INIT 0xE2FE4C `lea;
    move.l An,<dst>'; FLP_$DINIT 0xE3E15E `pea (0x80,A5)'). }
module aa;
type
  drv_t = record
    a, b: integer32;
    open: univ_ptr;
  end;
  pint = 0..65535;
var
  nil_driver: extern drv_t;
  bufpool: extern integer32;
  g: integer32;
  buf: array[1..4] of integer32;

function find(net_id: integer; off: pint): univ_ptr;
  options(extern, val_param);
procedure vtop(va: integer32); options(extern, val_param);

procedure aa_$offset(in net_id: integer);
var
  h: univ_ptr;
begin
  h := find(net_id, integer32(addr(nil_driver.open))
                    - integer32(addr(nil_driver)));
  g := integer32(h);
end;

procedure aa_$sums(x: integer32); options(val_param);
begin
  g := (x & 16#3FF) + integer32(addr(bufpool));
  g := g - integer32(addr(bufpool));
end;

procedure aa_$frame;
var
  loc: array[1..8] of integer32;
begin
  g := integer32(addr(loc[3])) - integer32(addr(loc));
  g := integer32(addr(bufpool));
  g := integer32(addr(buf[2]));
  vtop(integer32(addr(buf)));
end;
