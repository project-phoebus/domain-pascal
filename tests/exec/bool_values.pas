{ Execution test (bead domain-pascal-2lse): boolean values formed from
  conditions other than a comparison (ref 3.5.3: a boolean is a byte,
  0xFF true and 0 false; exec_$log_bool prints `bool?0xNN' for any other
  byte).

  - `not' of a boolean variable (a static, a frame local, a parameter by
    address and by value, a field through a pointer), of a function's
    result, of a one-bit packed field, of a comparison, of a plain
    `and'/`or', and `not not' (ref "Not", p. 4-126), stored to memory, to
    a register-held local and to the function result, and passed by
    value and by address.
  - `and then' / `or else' as values (ref "And Then", pp. 4-18..4-19;
    "Or Else", p. 4-134) of comparisons, boolean variables, `not' and
    each other, stored, passed and returned; their right operand is not
    evaluated when the left one decides (`probe' marks its evaluation).
  - `not', `and then' and `or else' as operands of a plain `and'/`or'.
  - `not' of one-bit packed booleans in bit 7 and bit 0 of a byte whose
    other bits are all set (BOOL_VALUES_$PK7, the review of
    domain-pascal-2lse), also under `not not', `and then' and `not' of an
    `and then'.
  - A compare that computes into a data register first (`(w & 15) =
    15'), stored to a register-held local (PARITY_$CHK 0xE0AF18).

  Each routine is called with inputs that make each value both true and
  false.

  call: BOOL_VALUES_$NOTS w:1 b:255
  call: BOOL_VALUES_$NOTS w:0 b:0
  call: BOOL_VALUES_$SHORT w:0 w:4
  call: BOOL_VALUES_$SHORT w:0 w:3
  call: BOOL_VALUES_$SHORT w:2 w:4
  call: BOOL_VALUES_$HELD w:3
  call: BOOL_VALUES_$FUNC w:1 w:4 -> b
  call: BOOL_VALUES_$FUNC w:0 w:4 -> b
  call: BOOL_VALUES_$FUNC w:0 w:5 -> b
  call: BOOL_VALUES_$NOTF w:5 -> b
  call: BOOL_VALUES_$NOTF w:0 -> b
  call: BOOL_VALUES_$MIXED w:1 w:0
  call: BOOL_VALUES_$MIXED w:0 w:7
  call: BOOL_VALUES_$LANES w:15
  call: BOOL_VALUES_$LANES w:16
  call: BOOL_VALUES_$LANES w:31
  call: BOOL_VALUES_$LANES2 w:15
  call: BOOL_VALUES_$LANES2 w:16
  call: BOOL_VALUES_$LANES2 w:31
  call: BOOL_VALUES_$PK7 w:127 b:255
  call: BOOL_VALUES_$PK7 w:127 b:0
  expect:
    false
    false
    false
    false
    false
    false
    false
    true
    false
    false
    false
    true
    false
    false
    false
    true
    true
    true
    true
    true
    true
    true
    false
    true
    true
    true
    false
    true
    true
    true
    true
    true
    mark 1
    true
    mark 1
    false
    false
    true
    true
    true
    true
    true
    false
    false
    false
    mark 1
    false
    mark 1
    true
    false
    true
    false
    false
    true
    false
    true
    false
    true
    false
    true
    true
    false
    true
    false
    true
    false
    true
    true
    true
    false
    mark 22
    mark 21
    => true
    mark 21
    => false
    mark 21
    => false
    mark 5
    => false
    mark 0
    => true
    true
    true
    true
    true
    false
    false
    false
    true
    false
    true
    true
    true
    mark 1
    false
    mark 0
    true
    mark 1
    true
    true
    false
    false
    true
    false
    true
    true
    false
    true
    true
    false
    true
    false
    false
    false
    true
    true
    false
    true
    true
    true
    false
    false
    true
}
module bool_values;
%include 'exec.ins.pas';

type
  rec_t = record
    w: integer;
    f: boolean;
  end;
  rec_ptr_t = ^rec_t;
  bits_t = packed record
    pad: 0..63;
    hi: boolean;
    lo: boolean;
  end;
  top_t = packed record
    hi: boolean;
    pad: 0..127;
  end;
  low_t = packed record
    pad: 0..127;
    lo: boolean;
  end;

var
  sb: boolean;
  sc: boolean;
  si: integer;
  r: rec_t;
  rp: rec_ptr_t;
  pk: bits_t;
  q1: top_t;
  q2: low_t;

procedure take(b1: boolean; b2: boolean); options(val_param);
begin
  exec_$log_bool(b1);
  exec_$log_bool(b2);
end;

procedure take_ref(in b: boolean);
begin
  exec_$log_bool(b);
end;

function probe(v: integer; b: boolean): boolean; options(val_param);
begin
  exec_$mark(v);
  probe := b;
end;

function is_pos(v: integer): boolean; options(val_param);
begin
  is_pos := v > 0;
end;

{ `not' of variables, fields, results, comparisons, plain and/or }
procedure bool_values_$nots(v: integer; b: boolean); options(val_param);
var
  lb: boolean;
  p: rec_ptr_t;
begin
  sb := v = 1;
  sc := not sb;
  exec_$log_bool(sc);
  lb := not b;
  exec_$log_bool(lb);
  take_ref(lb);
  r.f := b;
  rp := addr(r);
  sc := not rp^.f;
  exec_$log_bool(sc);
  p := rp;
  p^.f := not p^.f;
  exec_$log_bool(r.f);
  sc := not is_pos(v);
  exec_$log_bool(sc);
  pk.lo := b;
  pk.hi := not b;
  sc := not pk.lo;
  exec_$log_bool(sc);
  sc := not pk.hi;
  exec_$log_bool(sc);
  sc := not (v = 1);
  exec_$log_bool(sc);
  sc := not (sb or b);
  exec_$log_bool(sc);
  sc := not (sb and b);
  exec_$log_bool(sc);
  sc := not not sb;
  exec_$log_bool(sc);
  take(not b, not (v > 0));
  take_ref(not sb);
end;

{ `and then' / `or else' as values }
procedure bool_values_$short(v: integer; w: integer); options(val_param);
var
  lb: boolean;
begin
  si := w;
  sb := (v = 0) and then (si = 4);
  exec_$log_bool(sb);
  sc := (v <> 0) or else (si = 4);
  exec_$log_bool(sc);
  sb := (v = 0) and then probe(1, w = 4);
  exec_$log_bool(sb);
  sb := (v <> 0) or else probe(1, w = 3);
  exec_$log_bool(sb);
  lb := sb and then sc;
  exec_$log_bool(lb);
  lb := not sb or else not sc;
  exec_$log_bool(lb);
  sb := ((v = 0) and then (w > 3)) or else ((v = 2) and then (w = 4));
  exec_$log_bool(sb);
  take((v = 0) and then (si = 4), (w = 4) or else (v = 0));
  take_ref((v = 0) and then (w = 4));
  sb := not ((v = 0) and then (w = 4));
  exec_$log_bool(sb);
end;

{ values stored to a register-held local in a loop }
procedure bool_values_$held(n: integer); options(val_param);
var
  i, k: integer;
  b, c: boolean;
begin
  i := 0;
  k := 0;
  c := false;
  repeat
    b := not c;
    c := (i = 1) or else (i = 2);
    if b then k := k + 1;
    if c then k := k + 10;
    exec_$log_bool(b);
    i := i + 1;
  until i >= n;
  exec_$mark(k);
end;

{ the function result }
function bool_values_$func(v: integer; w: integer): boolean; options(val_param);
begin
  exec_$mark(21);
  bool_values_$func := (v <> 0) or else ((w = 4) and then (v = 1));
end;

function bool_values_$notf(v: integer): boolean; options(val_param);
begin
  exec_$mark(v);
  bool_values_$notf := not is_pos(v);
end;

{ `not', `and then', `or else' inside a plain `and'/`or' }
procedure bool_values_$mixed(v: integer; w: integer); options(val_param);
begin
  sb := v = 1;
  sc := (not (w > 1)) and sb;
  exec_$log_bool(sc);
  sc := ((v = 1) and then (w = 0)) or (w = 7);
  exec_$log_bool(sc);
  sc := sb and ((w = 0) or else (v = 5));
  exec_$log_bool(sc);
  sc := not sb or not (w = 7);
  exec_$log_bool(sc);
  sc := (not sb) and (not (w = 0));
  exec_$log_bool(sc);
  sc := not (sb and then (w = 0));
  exec_$log_bool(sc);
end;

{ a compare computing into a data register, to a local in memory and to
  register-held locals }
procedure bool_values_$lanes(w: integer); options(val_param);
var
  ok: boolean;
  m: integer;
begin
  m := w;
  ok := (m & 15) = 15;
  if ok then si := 1 else si := 0;
  exec_$mark(si);
  ok := not ((m & 16) = 0);
  exec_$log_bool(ok);
end;

procedure bool_values_$lanes2(w: integer); options(val_param);
var
  ok, nk: boolean;
  m: integer;
begin
  m := w;
  ok := (m & 15) = 15;
  nk := not ((m & 16) = 0);
  take(ok, nk);
  ok := not ok and nk;
  take(ok, not ok);
end;

{ one-bit booleans in bit 7 and bit 0, the other bits of the byte set }
procedure bool_values_$pk7(n: integer; b: boolean); options(val_param);
begin
  q1.pad := n;
  q1.hi := b;
  q2.pad := n;
  q2.lo := b;
  sc := not q1.hi;
  exec_$log_bool(sc);
  sc := not q2.lo;
  exec_$log_bool(sc);
  take(not q1.hi, not not q1.hi);
  sc := q1.hi and then q2.lo;
  exec_$log_bool(sc);
  sc := not (q1.hi and then q2.lo);
  exec_$log_bool(sc);
end;
