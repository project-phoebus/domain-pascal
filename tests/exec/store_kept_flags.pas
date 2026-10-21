{ Execution test (bead domain-pascal-x6a1): a boolean stored into an
  element whose index takes more instructions than the value, the value
  a test left to the flags of an instruction before the statement (no
  `tst'): the entry load of a parameter (codegen-patterns 12.2), a
  `subq' of the variable in memory, the store of a local into its slot,
  the operation that formed a register-held local, the `move' of an
  assignment to one (codegen-patterns 7.1).  The store's destination
  first (codegen-patterns 12.4) would put the index's `add' between
  those flags and the `s<cc>', which then read the `add''s: the index
  is never 0, so every `= 0' would be false and `< 0' false.  Best
  effort: the value's instructions first.  Each element is reset to the
  other value before the store (by the call before, for the entry load)
  and read back.

  warns: best effort (not byte-identical): a store whose destination is set up by more instructions than the value, the value tested on flags set before it
  call: STORE_KEPT_FLAGS_$ENTRY w:1 w:0
  call: STORE_KEPT_FLAGS_$SHOW w:6
  call: STORE_KEPT_FLAGS_$ENTRY w:1 w:4
  call: STORE_KEPT_FLAGS_$SHOW w:6
  call: STORE_KEPT_FLAGS_$SHOWS
  call: STORE_KEPT_FLAGS_$SUBQ w:1
  call: STORE_KEPT_FLAGS_$SHOW w:6
  call: STORE_KEPT_FLAGS_$SUBQ w:1
  call: STORE_KEPT_FLAGS_$SHOW w:6
  call: STORE_KEPT_FLAGS_$LOC w:3 w:-3
  call: STORE_KEPT_FLAGS_$SHOW w:14
  call: STORE_KEPT_FLAGS_$LOC w:3 w:-2
  call: STORE_KEPT_FLAGS_$SHOW w:14
  call: STORE_KEPT_FLAGS_$WEB w:4 w:-4
  call: STORE_KEPT_FLAGS_$SHOW w:18
  call: STORE_KEPT_FLAGS_$WEB w:4 w:1
  call: STORE_KEPT_FLAGS_$SHOW w:18
  call: STORE_KEPT_FLAGS_$SHOWS
  call: STORE_KEPT_FLAGS_$ASG w:5
  call: STORE_KEPT_FLAGS_$SHOW w:22
  call: STORE_KEPT_FLAGS_$SETK @w:7
  call: STORE_KEPT_FLAGS_$ASG w:5
  call: STORE_KEPT_FLAGS_$SHOW w:22
  call: STORE_KEPT_FLAGS_$SHOWS
  expect:
    1
    0
    5
    0
    1
    0
    1
    1
    0
    1
    0
    5
    1
    0
    7
}
module store_kept_flags;
%include 'exec.ins.pas';

var
  h: integer := 2;
  m: integer := 1;
  k: integer := 0;
  s: integer := 0;
  g: array[1..100] of boolean;

{ the entry load of `i': `move.w (0xa,A6),D1; seq' }
procedure store_kept_flags_$entry(p: integer; i: integer); options(val_param);
begin
  g[p * 4 + h] := i = 0;
  s := p + i;
end;

{ `subq.w #1,m; smi' }
procedure store_kept_flags_$subq(p: integer); options(val_param);
begin
  g[p * 4 + h] := true;
  m := m - 1;
  g[p * 4 + h] := m < 0;
end;

{ the local's store `move.w D2,(-2,A6); seq' (its address passed later) }
procedure store_kept_flags_$loc(p: integer; q: integer); options(val_param);
var x: integer;
begin
  g[p * 4 + h] := false;
  x := p + q;
  g[p * 4 + h] := x = 0;
  exec_$log_w(x);
end;

{ a register-held local formed by `add.w': `add.w D1,D2; seq' }
procedure store_kept_flags_$web(p: integer; q: integer); options(val_param);
var l: integer;
begin
  g[p * 4 + h] := false;
  l := p + q;
  g[p * 4 + h] := l = 0;
  s := l;
end;

{ a register-held local assigned a load: `move.w k,D1; seq' }
procedure store_kept_flags_$asg(p: integer); options(val_param);
var l: integer;
begin
  g[p * 4 + h] := k <> 0;
  l := k;
  g[p * 4 + h] := l = 0;
  s := l;
end;

procedure store_kept_flags_$setk(in v: integer);
begin
  k := v;
end;

procedure store_kept_flags_$show(i: integer); options(val_param);
begin
  if g[i] then exec_$logv_w(1) else exec_$logv_w(0);
end;

procedure store_kept_flags_$shows;
begin
  exec_$logv_w(s);
end;
