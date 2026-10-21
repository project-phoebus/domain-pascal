{ Execution test (bead domain-pascal-45yc): an integer constant passed
  by address to a 4-byte non-integer `univ' parameter is a 4-byte cell
  holding its integer32 value, big-endian and sign-extended (ref 5.3.3
  p. 5-11: "Domain Pascal extends the expression to be the same size as
  the univ parameter"; 3.2.3 p. 3-5: an integer constant is a 32-bit
  integer).  SAU5 MBUS_$UNDEF_INT 0xE0AA10 passes CRASH_SYSTEM's status
  as the cell 0xE0AA20 `0000 002a'.  The callee reads the parameter as a
  longword (the record's integer32 variant) and as its two words, and
  logs its bytes.

  call: UNIV_SMALL_CONST_$RUN
  expect:
    42
    0
    42
    mem 00 00 00 2a
    -1
    -1
    -1
    mem ff ff ff ff
    -2
    -1
    -2
    mem ff ff ff fe
    65535
    0
    -1
    mem 00 00 ff ff
    -32768
    -1
    -32768
    mem ff ff 80 00
    74565
    1
    9029
    mem 00 01 23 45
    42
    42
}
module univ_small_const;
%include 'exec.ins.pas';

type
  status_t = packed record
    case integer of
      1: (fail: boolean; subsys: 0..127; modc: 0..255; code: integer);
      2: (all: integer32);
  end;
  halves_t = record hi, lo: integer; end;
  u16 = 0..65535;

const
  c = 16#2A;
  m1 = -1;

procedure crash(in status: univ status_t);
var h: halves_t;
begin
  exec_$log_l(status.all);
  h := halves_t(status);
  exec_$log_w(h.hi);
  exec_$log_w(h.lo);
  exec_$log_mem(status, 4);
end;

procedure take_l(in v: univ integer32);
begin
  exec_$log_l(v);
end;

procedure univ_small_const_$run;
begin
  crash(c);
  crash(m1);
  crash(-2);
  crash(lastof(u16));
  crash(-32768);
  crash(16#12345);
  take_l(c);
  take_l(16#2A);
end;
