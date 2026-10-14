{ Execution test (bead domain-pascal-k1sv): computed booleans stored and
  pushed, `in_range' of an enumeration, and stores whose destination and
  value both need a register.

  - A comparison or `odd' stored to a static, to a frame local whose
    address escapes, and to a record through a pointer (`s<cc>' into a
    data register, then `move.b'; codegen-patterns 3, "Comparisons as
    boolean results"); a comparison assigned to a register-held local in
    a loop (`s<cc>' into the local's register, TTY_$K_GET 0xE1C42E).
  - A comparison and a set member passed by value (`val_param') to a
    boolean parameter (TTY_$K_GET 0xE1C54E).
  - `in_range' of an enumeration (ref "In_range", p. 4-104: true when the
    value is within the type's range; an enumeration's ordinals are
    0..n-1, ref 3.7.1) in a condition and as a value, for words in and out
    of the range (a caller's word passed by address, as TTY_$K_SET_FUNC_
    CHAR's func).

  call: BOOL_STORES_$CMP w:3 w:3
  call: BOOL_STORES_$CMP w:-2 w:7
  call: BOOL_STORES_$LOOP w:4
  call: BOOL_STORES_$PUSH w:5 w:1
  call: BOOL_STORES_$PUSH w:-5 w:2
  call: BOOL_STORES_$RANGE @w:2
  call: BOOL_STORES_$RANGE @w:4
  call: BOOL_STORES_$RANGE @w:5
  call: BOOL_STORES_$RANGE @w:-1
  expect:
    true
    true
    false
    false
    false
    false
    false
    true
    true
    true
    false
    false
    true
    false
    mark 3
    true
    true
    true
    true
    false
    false
    false
    false
    mark 1
    true
    mark 0
    true
    mark 0
    false
    mark 0
    false
}
module bool_stores;
%include 'exec.ins.pas';

type
  color_t = (red, orange, yellow, green, blue);
  rec_t = record
    a: array[0..7] of integer;
    f: boolean;
    g: boolean;
  end;
  rec_ptr_t = ^rec_t;
  flags_t = set of 0..15;

var
  sb: boolean;
  si: integer;
  r: rec_t;
  rp: rec_ptr_t;
  fl: flags_t;

procedure take(b1: boolean; b2: boolean); options(val_param);
begin
  exec_$log_bool(b1);
  exec_$log_bool(b2);
end;

procedure bool_stores_$cmp(v: integer; w: integer); options(val_param);
var
  lb: boolean;
begin
  si := v;
  sb := v = 3;
  exec_$log_bool(sb);
  lb := si >= 2;
  exec_$log_bool(lb);
  rp := addr(r);
  rp^.f := odd(si + 1);
  exec_$log_bool(r.f);
  rp^.g := v < 0;
  exec_$log_bool(r.g);
  sb := odd(v + 1);
  exec_$log_bool(sb);
end;

procedure bool_stores_$loop(n: integer); options(val_param);
var
  i, k: integer;
  b: boolean;
begin
  si := n;
  i := 0;
  k := 0;
  repeat
    b := i = 2;
    if b then k := k + 1;
    if not b then k := k + 2;
    exec_$log_bool(b);
    i := i + 1;
  until i >= si;
  exec_$mark(k - 4);
end;

procedure bool_stores_$push(v: integer; m: integer); options(val_param);
begin
  if m = 1 then fl := [3] else fl := [4];
  take(v > 0, 3 in fl);
  take(v = 5, odd(m));
end;

procedure bool_stores_$range(in c: color_t);
begin
  si := 1;
  if not in_range(c) or else (c > yellow) then
    si := 0;
  exec_$mark(si);
  sb := in_range(c);
  exec_$log_bool(sb);
end;
