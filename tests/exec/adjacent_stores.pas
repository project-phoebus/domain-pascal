{ Execution test (bead domain-pascal-2m0): consecutive stores to adjacent
  objects that no one reference could make, which dpc accepts and keeps
  apart (codegen-patterns 13, "Store merging"; ref 3.15.3 p. 3-57).

  - Three words of a record through a pointer, each read through its own
    by-address parameter (TPAD_$SET_UNIT_MODE 0xE6982C..0xE69840).
  - A constant word next to a word read through a parameter.
  - Two bytes of a frame record, the lower at an odd address (no word
    reference there on the 68010; in the frame 0xE62A42, absolute
    0xE30DD2): the run would stop with an address error if they were
    merged.

  call: ADJACENT_STORES_$SCALES @w:7 @w:-3 @w:300
  call: ADJACENT_STORES_$MODE @w:4660
  call: ADJACENT_STORES_$ODD_BYTES
  expect:
    7
    -3
    300
    0
    4660
    300
    mem 00 61 62 00
}
module adjacent_stores;
%include 'exec.ins.pas';
type
  unit_t = record
    mode: integer;
    x_scale: integer;
    y_scale: integer;
  end;
  unit_ptr = ^unit_t;
  three = record
    a: char;
    b: char;
    c: char;
    d: char;
  end;
var
  u: unit_t;

procedure set_scales(p: unit_ptr; var m, x, y: integer); options(val_param, internal);
begin
  p^.mode := m;
  p^.x_scale := x;
  p^.y_scale := y;
end;

procedure set_mode(p: unit_ptr; var x: integer); options(val_param, internal);
begin
  p^.mode := 0;
  p^.x_scale := x;
end;

procedure adjacent_stores_$scales(var m, x, y: integer);
begin
  set_scales(addr(u), m, x, y);
  exec_$log_w(u.mode);
  exec_$log_w(u.x_scale);
  exec_$log_w(u.y_scale);
end;

procedure adjacent_stores_$mode(var x: integer);
begin
  set_mode(addr(u), x);
  exec_$log_w(u.mode);
  exec_$log_w(u.x_scale);
  exec_$log_w(u.y_scale);
end;

procedure adjacent_stores_$odd_bytes;
var
  r: three;
begin
  r.a := chr(0);
  exec_$clobber;
  r.b := 'a';
  r.c := 'b';
  exec_$clobber;
  r.d := chr(0);
  exec_$log_mem(r, 4);
end;
