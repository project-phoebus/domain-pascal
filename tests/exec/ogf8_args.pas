{ Execution test (bead domain-pascal-ogf8): arguments and parameters.
  - A 4-byte integer constant passed to a 1-byte `univ' parameter: a
    4-byte cell, its first byte read (PROC2's 0xE419E8, cell 0xE41A1C
    `ffff ffff' for XPD_$WRITE; ref 5.3.3 p. 5-11).
  - The empty string passed to a `univ' parameter: a cell of no bytes
    (DIR_$DO_OP 0xE4C310).
  - An integer expression passed to a 16-byte `univ' parameter: a
    temporary of the expression's size (NETLOG_$SEND_PAGE 0xE71D2E).
  - A pointer parameter passed by value and assigned: kept in its slot
    (DISK_$ADD_QUE 0xE3C790).
  - A function's result compared with a variable, the call on either
    side, in an `if' and a `repeat' (best effort: the branch on the
    compare).

  warns: best effort (not byte-identical): a comparison of a function's result with a variable
  call: OGF8_ARGS_$UNIV l:3
  call: OGF8_ARGS_$PTR w:3
  call: OGF8_ARGS_$CMP l:5 l:5
  call: OGF8_ARGS_$CMP l:5 l:6
  call: OGF8_ARGS_$CMP l:7 l:6
  expect:
    mem ff
    mem 12
    mem 00
    mem
    3072
    -1024
    30
    20
    10
    0
    101
    108
    71
    mark 5
    110
    108
    76
    mark 5
    110
    103
    71
    mark 7
}
module ogf8_args;
%include 'exec.ins.pas';

type
  b1_t = array[1..1] of char;
  d16_t = array[1..4] of integer32;
  ip = ^integer32;

var
  vals: array[1..3] of integer32 := [10, 20, 30];
  l: integer32;
  k: integer;
  wv: integer;

procedure put1(in b: univ b1_t);
begin
  exec_$log_mem(b, 1);
end;

procedure put16(in d: univ d16_t);
begin
  exec_$log_l(d[1]);
end;

procedure ogf8_args_$univ(v: integer32); options(val_param);
begin
  put1(integer32(-1));
  put1(integer32(16#12345678));
  put1(integer32(0));
  exec_$log_mem('', 0);
  l := v;
  put16(lshft(l, 10));
  put16(-l * 341 - 1);
end;

procedure walk(p: ip; n: integer); options(val_param);
begin
  while n > 0 do begin
    exec_$log_l(p^);
    if p^ = 10 then
      p := nil
    else
      p := ip(integer32(p) - 4);
    n := n - 1;
  end;
  if p = nil then
    exec_$logv_l(0);
end;

procedure ogf8_args_$ptr(n: integer); options(val_param);
begin
  walk(addr(vals[3]), n);
end;

procedure ogf8_args_$cmp(a: integer32; b: integer32); options(val_param);
var
  la: integer32;
begin
  la := a;
  l := b;
  if exec_$id_l(la) = l then
    exec_$logv_c('e')
  else
    exec_$logv_c('n');
  if l < exec_$id_l(la) then
    exec_$logv_c('g')
  else
    exec_$logv_c('l');
  if exec_$id_l(la) >= l then
    exec_$logv_c('G')
  else
    exec_$logv_c('L');
  k := 0;
  wv := a;
  repeat
    k := k + 1;
  until exec_$id_w(k) >= wv;
  exec_$mark(k);
end;
