{ Execution test (bead domain-pascal-tq8w): one-byte sets, `[byte] set
  of 0..7' (ref 3.15.5 p. 3-61; the owner's ruling, reference-
  clarifications "A size attribute on a set type": member k is bit k of
  the byte), fields at odd offsets (codegen-patterns 5.1, "One-byte
  sets").  A word set narrowed into one (its low byte, member 8 lost) from
  memory and from a function result; constant members tested in memory
  (`btst #k', member 7 as the sign); `[]' and constant stores; one member
  added and removed (`bset'/`bclr'); widened to a word (`clr.w; move.b')
  into a word set, which takes `-' and `+' of byte sets in place, and as
  a word value argument; compared with `[]' (widened) and `='.  The
  sets' bits are logged through a type transfer to `[byte] 0..255' (ref
  p. 4-197: the bits unchanged).

  call: BYTE_SETS_$RUN w:169 w:385
  call: BYTE_SETS_$RUN w:0 w:256
  expect:
    true
    true
    true
    mark 1
    233
    128
    14
    134
    mark 2
    mark 3
    233
    false
    false
    false
    64
    0
    14
    14
    mark 3
    64
}
module byte_sets;
%include 'exec.ins.pas';

type
  bs_t = [byte] set of 0..7;
  rights_t = set of 0..7;
  ws_t = set of 0..15;
  bp_t = [byte] 0..255;
  rec_t = record
    pad: char;
    a: bs_t;              { 1 }
    b: bs_t;              { 2 }
    c: bs_t;              { 3 }
    w: ws_t;              { 4 }
  end;
var
  r: rec_t;

function as_int(s: ws_t): integer; internal;
begin
  as_int := integer(s);
end;

procedure held_ops; internal;
var
  held: ws_t;
begin
  held := r.c;
  held := held - r.a;
  held := held + r.b;
  r.w := held;
end;

procedure byte_sets_$run(x: integer; y: integer); options(val_param);
var
  n: integer;
begin
  r.w := ws_t(x);
  r.a := r.w;
  r.b := rights_t(exec_$id_w(y));
  r.c := [];
  exec_$log_bool(0 in r.a);
  exec_$log_bool(3 in r.a);
  exec_$log_bool(7 in r.a);
  if 7 in r.b then
    exec_$mark(1);
  r.a := r.a + [6];
  r.b := r.b - [0];
  r.c := [1..3];
  exec_$logv_w(bp_t(r.a));
  exec_$logv_w(bp_t(r.b));
  exec_$logv_w(bp_t(r.c));
  held_ops;
  exec_$logv_w(integer(r.w));
  if r.b <> [] then
    exec_$mark(2);
  r.c := [];
  if r.c = [] then
    exec_$mark(3);
  n := as_int(r.a);
  exec_$logv_w(n);
end;
