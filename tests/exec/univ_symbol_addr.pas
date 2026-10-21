{ Execution test (bead domain-pascal-68tc): `addr(x)' of another unit's
  object or of a routine, passed by address to a parameter of 4 bytes or
  more, is a 4-byte cell in the code section holding the address
  (codegen-patterns 1.5, "Address of a symbol by address"; SAU5 VME_$INT
  0xE0ABBE, the cell 0xE0AC40 holding PBU_CTL's address, for LOG_$ADD's
  192-byte `univ' data).  The callees read the address out of the
  parameter's first 4 bytes and follow it: to a 192-byte `univ' record,
  to pointer parameters (an object, a field: the symbol plus an offset,
  the same cell twice), to an integer32 (`integer32(addr(x))'), and a
  routine's address, compared with the routine's address by value.  A
  variable of the unit's own overlay section stays a frame temporary
  (SAU2 PCHIST_$CNTL 0xE5CF06 `lea (0x934,A2),A0; move.l A0,(-40,A6);
  pea (-40,A6)'; dpc's `lea abs.l' is a best effort).

  link: univ_symbol_addr_data.mod.pas
  call: USA_$RUN
  warns: best effort (not byte-identical): `addr` of another unit's or a constant's object passed by address
  expect:
    6
    4660
    22136
    7
    703710
    22136
    22136
    1
    1
    703710
    321
    321
}
module usa;
%include 'exec.ins.pas';

type
  ctl_t = record
    csr: integer;
    mask: integer;
  end;
  ctl_p = ^ctl_t;
  { 192 bytes, LOG_$ADD's data in ../source's reading: the address
    passed sits in the first 4. }
  big_t = record
    p: ctl_p;
    rest: array[1..94] of integer;
  end;
  lp_t = ^integer32;
  wp_t = ^integer;
var (usa_ov)
  ov_pages: array[1..3] of integer32;
  ov_count: integer;

var
  usa_$ctl: extern ctl_t;
  usa_$long: extern integer32;

procedure usa_$cb;
begin
end;

procedure take_big(k: integer; in d: univ big_t; n: integer);
  options(val_param);
begin
  exec_$logv_w(k);
  exec_$logv_w(d.p^.csr);
  exec_$logv_w(d.p^.mask);
  exec_$logv_w(n);
end;

procedure take_ptrs(in l: lp_t; in a: wp_t; in b: wp_t);
begin
  exec_$logv_l(l^);
  exec_$logv_w(a^);
  exec_$logv_w(b^);
end;

procedure take_int(in v: integer32);
begin
  if v = integer32(addr(usa_$ctl)) then exec_$logv_w(1)
  else exec_$logv_w(0);
end;

procedure take_routine(in r: univ_ptr);
begin
  if integer32(r) = integer32(addr(usa_$cb)) then exec_$logv_w(1)
  else exec_$logv_w(0);
end;

procedure usa_$run;
begin
  ov_count := 321;
  take_big(6, addr(usa_$ctl), 7);
  take_ptrs(addr(usa_$long), addr(usa_$ctl.mask), addr(usa_$ctl.mask));
  take_int(integer32(addr(usa_$ctl)));
  take_routine(addr(usa_$cb));
  take_ptrs(addr(usa_$long), addr(ov_count), addr(ov_count));
end;
