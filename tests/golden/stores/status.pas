{ codegen-patterns 9 (status out-parameters), 12.4 R4(b), 12.2, 7.1, 3,
  1.5.  status_$t is a record with the integer32 field `all' (ref
  9.1.1-9.1.2, pp. 9-1..9-3; only `all' is declared, at offset 0).  The
  status codes are written in hex; their texts in the 10.2 status-code
  database: 16#6000E "contiguous pages unavailable" (OS / MMAP manager),
  16#2C000A "device not in system" (OS / internet interface controller),
  16#100008 "illegal unit number", 16#100009 "unit not acquired",
  16#30004 "segment is not deactivatable" (OS / AST manager; any code
  would do there).

  - busy_wait: `status.all := status_$ok' alone, `movea.l (0xc,A6),A0;
    clr.l (A0)' (SMD_$BUSY_WAIT 0xE1D8AA, here without its A5 load:
    codegen-patterns 4);
  - alloc_contig: two slots used once, A0 then A1 (MMAP_$ALLOC_CONTIG
    0xE0D8FC..0xE0D906, without A5), `move.l #0x6000e,(A1)';
  - receive: three slots used once, A0, A1, A2, A2 saved with `pea (A2)'
    (IIC_$RECEIVE 0xE70ABA);
  - get_stats: a slot used three times held in A0 from the entry, then
    the status slot in A1, not the dead A0 (XNS_IDP_$GET_STATS 0xE18FF4,
    here from another unit's variables);
  - after_copy: registers a block copy used up with `(A0)+' / `(A1)+'
    count as free (DIR_$ADD_MOUNT 0xE534DC: A0 again after an `(A0)+'
    copy), so the status slot takes A0;
  - check_unit: the status loaded at entry (used three times), error
    stores `move.l #imm,(A0)', `return' as a `bra' to the epilogue
    (module-local 0xE1A4A4 for the shape: `clr.l (A0)' then the stores);
  - call_then_test: the status held in A2 across the call, passed on with
    `pea (A2)', tested with `tst.l (A2)', `if status.all <> status_$ok
    then return' as `bne' straight to the register restore (TERM_$CONTROL
    0xE66B3A; module-local 0xE01960 holds the status in A2);
  - receive_check: a status store, then a constant boolean result
    `clr.b D0' (IIC_$RECEIVE_CHECK 0xE70ADE; IO_$ALLOC 0xE1A9C6 with
    `clr.l D0'). }
module status;
const
  status_$ok = 0;
type
  status_$t = record
    all: integer32;
  end;
  stats_t = record
    sent, received, dropped: integer32;
  end;
var
  sent, received, dropped: extern integer32;
  acquired: extern integer;

procedure ext_$op(in x: integer32; out status: status_$t); options(val_param, extern);

procedure busy_wait(in x: integer32; out status: status_$t);
begin
  status.all := status_$ok;
end;

procedure alloc_contig(n: integer; out pages: integer32; out status: status_$t);
  options(val_param);
begin
  pages := 0;
  status.all := 16#6000E;
end;

procedure receive(in a1, a2: integer32; out c3: integer; in a4: integer32;
                  out c5: integer; out status: status_$t);
begin
  status.all := 16#2C000A;
  c5 := 0;
  c3 := 0;
end;

procedure get_stats(out stats: stats_t; out status: status_$t);
begin
  stats.sent := sent;
  stats.received := received;
  stats.dropped := dropped;
  status.all := status_$ok;
end;

procedure after_copy(in src: stats_t; out stats: stats_t; out status: status_$t);
begin
  stats := src;
  status.all := status_$ok;
end;

procedure check_unit(n: integer; out status: status_$t); options(val_param);
begin
  status.all := status_$ok;
  if n > 31 then begin
    status.all := 16#100008;
    return;
  end;
  if acquired = 0 then
    status.all := 16#100009;
end;

procedure call_then_test(out status: status_$t);
begin
  ext_$op(1, status);
  if status.all <> status_$ok then return;
  status.all := 16#30004;
end;

function receive_check(in a1: integer32; out status: status_$t): boolean;
begin
  status.all := 16#2C000A;
  receive_check := false;
end;
