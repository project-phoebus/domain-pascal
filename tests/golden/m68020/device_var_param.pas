{ codegen-patterns 5.5 ("device pointers in parameters") and 12.2; bead
  domain-pascal-prgf.  A `var' parameter of a `[device]' record follows
  the rules of every by-address slot: used twice or more, it is loaded
  into an address register at entry and held for its arguments and its
  device references alike.  SAU9 RING: ring_$setup_tx 0x3C47D7DE
  `movea.l (0x1c,A6),A2' at entry, four `pea (A2)' and the store `move.w
  D2,(0x1002,A2)' at 0x3C47D966; ring_$set_timer 0x3C47D000 `movea.l
  (0xc,A6),A0', then nine stores through A0 and nothing else;
  ring_$set_rx_buffer 0x3C47D1AE `movea.l (0xa,A6),A2'.  Only a by-value
  pointer parameter is reloaded before each device reference (module-local
  0xE0A328, golden stores/const_stores and the conformance unit
  dma_init_m68450_channel). }
module device_var_param;
type
  r_t = [device] record a: integer; b: integer; end;
procedure q(var r: r_t; n: integer); options(extern, val_param);
{ held: two arguments and a store }
procedure p(var r: r_t; n: integer); options(val_param);
begin
  q(r, n);
  q(r, n);
  r.b := n;
end;
{ held: stores only }
procedure stores(var r: r_t; n: integer); options(val_param);
begin
  r.a := n;
  r.b := n;
end;
