{ Execution test: `addr(x)' and `integer32(addr(x))' as values (bead
  domain-pascal-j28d; codegen-patterns 5.2, "`addr(x)` as a value").
  Differences of addresses (a field's offset in another unit's record,
  NET_$OPEN 0xE5A1B8 `lea abs.l; move.l; sub.l #sym`; an element's
  offset in a frame array, 0xE4BDCE, the subtrahend's `lea` first), an
  address plus a value and minus a value (`add.l #sym`/`sub.l #sym',
  0xE0F14E, 0xE09E6E), and addresses stored as integers and read back
  through a pointer (NETBUF_$INIT 0xE2F63C, SOCK_$INIT 0xE2FE4C), or
  passed by value as integers (FLP_$DINIT 0xE3E15E `pea').

  link: addr_values_data.mod.pas
  call: ADDRV_$RUN
  expect:
    8
    10
    12
    4
    24
    20
    -20
    1234
    5678
    99
    5678
    7
}
module addrv;
%include 'exec.ins.pas';

type
  drv_t = record
    a: integer32;
    b: integer32;
    c: integer;
    d: integer32;
  end;
  lp_t = ^integer32;
var
  addrv_$drv: extern drv_t;
  addrv_$long: extern integer32;
  mine: integer32;
  saved: integer32;
  w: integer;

procedure addrv_$through(p: integer32); options(val_param);
begin
  exec_$logv_l(lp_t(p)^);
end;

procedure addrv_$run;
var
  loc: array[1..8] of integer32;
  i: integer32;
  k: integer32;
begin
  { another unit's field offsets, as NET_$OPEN computes its entry's }
  k := integer32(addr(addrv_$drv.c)) - integer32(addr(addrv_$drv));
  exec_$log_l(k);
  w := integer32(addr(addrv_$drv.d)) - integer32(addr(addrv_$drv));
  exec_$log_w(w);
  k := integer32(addr(addrv_$drv.d)) - integer32(addr(addrv_$drv)) + 2;
  exec_$log_l(k);
  { an element's offset in a frame array: both addresses `lea'd }
  k := integer32(addr(loc[2])) - integer32(addr(loc));
  exec_$log_l(k);
  k := integer32(addr(loc[8])) - integer32(addr(loc[2]));
  exec_$log_l(k);
  { an address plus and minus a value }
  i := 20;
  k := (i & 16#3FF) + integer32(addr(addrv_$long));
  k := k - integer32(addr(addrv_$long));
  exec_$log_l(k);
  k := 0 - i;
  k := k + integer32(addr(addrv_$long)) ;
  k := k - integer32(addr(addrv_$long));
  exec_$log_l(k);
  { addresses stored as integers and read back through a pointer }
  mine := 1234;
  saved := integer32(addr(mine));
  exec_$logv_l(lp_t(saved)^);
  addrv_$long := 5678;
  saved := integer32(addr(addrv_$long));
  exec_$logv_l(lp_t(saved)^);
  loc[3] := 99;
  saved := integer32(addr(loc[3]));
  exec_$logv_l(lp_t(saved)^);
  { passed by value as an integer }
  addrv_$through(integer32(addr(addrv_$long)));
  mine := 7;
  addrv_$through(integer32(addr(mine)));
end;
