{ Execution test: packed record fields (ref 3.10.7; codegen-patterns
  5.3): stores of a bit field leave the neighbouring bits alone, a field
  across a byte boundary, reads back.  The layout is status_$t's (bit 7
  of byte 0 `fail', `code' the low word) and the RIP/APP_ fields of
  tests/golden/packed/fields.pas.

  call: PACKED_FIELDS_$STATUS w:0x1234
  call: PACKED_FIELDS_$SPAN w:165
  expect:
    0x85ffffff
    0x05ffffff
    0x05ff1234
    0x85ff1234
    5
    true
    0x807fffff
    0x800000ff
    0xd28000ff
    165
    true
    0
    0x52800000
    false
    0x00000000
}
module packed_fields;
%include 'exec.ins.pas';

type
  status_$t = packed record case integer of
      1:  (fail:  boolean;
           subsys: 0..127;
           modc: 0..255;
           code: integer);
      2:  (all: integer32);
      end;
  span_t = packed record case integer of
      1:  (b: boolean;
           depth: 0..255;
           low: 0..32767);
      2:  (all: integer32);
      end;
var
  st: status_$t;
  sp: span_t;
  w: integer;
  ok: boolean;

procedure packed_fields_$status(v: integer); options(val_param);
begin
  st.all := -1;
  st.subsys := 5;
  exec_$log_x(st.all);
  st.fail := false;
  exec_$log_x(st.all);
  st.code := v;
  exec_$log_x(st.all);
  st.fail := true;
  exec_$log_x(st.all);
  w := st.subsys;
  exec_$logv_w(w);
  ok := st.fail;
  exec_$log_bool(ok);
end;

procedure packed_fields_$span(v: integer); options(val_param);
begin
  sp.all := -1;
  sp.depth := 0;
  exec_$log_x(sp.all);
  sp.low := 0;
  exec_$log_x(sp.all);
  sp.depth := v;
  exec_$log_x(sp.all);
  w := sp.depth;
  exec_$logv_w(w);
  ok := sp.b;
  exec_$log_bool(ok);
  w := sp.low;
  exec_$logv_w(w);
  sp.all := 0;
  sp.depth := v;
  exec_$log_x(sp.all);
  ok := sp.b;
  exec_$log_bool(ok);
  sp.depth := 0;
  exec_$log_x(sp.all);
end;
