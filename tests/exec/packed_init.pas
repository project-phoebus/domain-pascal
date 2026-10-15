{ Execution test: initial values of packed records' bit fields (ref
  3.10.7 p. 3-35; codegen-patterns 11 "Initial values"; bead
  domain-pascal-c0j8): the data bytes (KBD's kbd_$state_table, WIRED_DATA
  0xE2DDE4+0xC0) and the fields read back with the packed-field reads of
  codegen-patterns 5.3: 4-bit subranges, a boolean, an enumeration, a
  10-bit field across a byte boundary, a negative subrange (its bytes
  only: reading one back is not supported).

  call: PACKED_INIT_$SHOW
  expect:
    mem fb 20 e8 31 df 31 fe b4 ff 10 00 10
    mem c9 cf fe 00 c9 cf fe 00 0f f0 40 00
    251
    2
    0
    3
    11
    4
    15
    12
    true
    4
    1023
    true
    false
    0
    16
    false
}
module packed_init;
%include 'exec.ins.pas';

type
  entry = packed record
    key: char;
    action: 0..15;
    next_state: 0..15;
  end;
  colour = (red, green, blue, cyan, magenta);
  mixed = packed record
    b: boolean;
    c: colour;
    s: -100..40;
    w: 0..1023;
    t: boolean;
  end;
var
  table: array[0..9] of entry := [
    [key := chr(16#FB), action := 2, next_state := 0],
    [key := chr(16#E8), action := 3, next_state := 1],
    [key := chr(16#DF), action := 3, next_state := 1],
    [key := chr(16#FE), action := 11, next_state := 4],
    [key := chr(16#FF), action := 1, next_state := 0],
    [key := chr(0), action := 1, next_state := 0],
    [key := chr(0), action := 4, next_state := 2],
    [key := chr(0), action := 5, next_state := 3],
    [key := chr(0), action := 6, next_state := 15],
    [key := chr(0), action := 12, next_state := 0]];
  m: array[1..3] of mixed := [2 of [true, magenta, -100, 1023, true],
                              [w := 5 * 3 + 1, s := -1]];
  w: integer;
  ok: boolean;

procedure packed_init_$show;
begin
  exec_$log_mem(table, 12);
  exec_$log_mem(m, 12);
  w := ord(table[0].key);
  exec_$logv_w(w);
  w := table[0].action;
  exec_$logv_w(w);
  w := table[0].next_state;
  exec_$logv_w(w);
  w := table[1].action;
  exec_$logv_w(w);
  w := table[3].action;
  exec_$logv_w(w);
  w := table[3].next_state;
  exec_$logv_w(w);
  w := table[8].next_state;
  exec_$logv_w(w);
  w := table[9].action;
  exec_$logv_w(w);
  ok := m[1].b;
  exec_$log_bool(ok);
  w := ord(m[2].c);
  exec_$logv_w(w);
  w := m[2].w;
  exec_$logv_w(w);
  ok := m[2].t;
  exec_$log_bool(ok);
  ok := m[3].b;
  exec_$log_bool(ok);
  w := ord(m[3].c);
  exec_$logv_w(w);
  w := m[3].w;
  exec_$logv_w(w);
  ok := m[3].t;
  exec_$log_bool(ok);
end;
