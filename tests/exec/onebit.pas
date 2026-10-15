{ Execution test (bead domain-pascal-3xwx): one-bit integer tests
  (codegen-patterns 7.1, "One bit"; 8, `odd').  A word or longword
  variable in memory is loaded whole and tested with `btst #k'
  (tty_$i_put_chars 0xE1B01E `move.w (8,A0),D1; btst #5,D1'); `odd' of
  one is `lsr.b #1' and the carry (0xE1B0F6 `move.l (12,A0),D1; lsr.b
  #1,D1; bcc.s'); a computed value is tested in its register
  (DIR_$SET_PROTECTION 0xE522D4 `lsr.w #4,D1; btst #4,D1').  Bits 0, 1,
  6, 8, 14 of a word and 0, 6, 8, 16, 30 of a longword, `<> 0' and `= 0',
  with negative values (bits 7, 15 and 31 alone are refused: no `btst
  #7'/`#15'/`#31' in compiled code).

  call: ONEBIT_$SET w:1 l:0x40000001
  call: ONEBIT_$TEST
  call: ONEBIT_$SET w:-1 l:0xffffffff
  call: ONEBIT_$TEST
  call: ONEBIT_$SET w:0x4140 l:0x00010140
  call: ONEBIT_$TEST
  call: ONEBIT_$SET w:-32768 l:0x80000000
  call: ONEBIT_$TEST
  call: ONEBIT_$SET w:-2 l:0xfffffffe
  call: ONEBIT_$TEST
  call: ONEBIT_$SET w:-32767 l:0x0001000f
  call: ONEBIT_$REC
  call: ONEBIT_$SET w:16 l:0x00100020
  call: ONEBIT_$REC
  expect:
    mark 1
    mark 20
    mark 31
    mark 60
    mark 99
    mark 1
    mark 6
    mark 8
    mark 14
    mark 31
    mark 36
    mark 38
    mark 46
    mark 60
    mark 99
    mark 6
    mark 8
    mark 14
    mark 20
    mark 36
    mark 38
    mark 46
    mark 70
    mark 99
    mark 20
    mark 70
    mark 99
    mark 6
    mark 8
    mark 14
    mark 36
    mark 38
    mark 46
    mark 60
    mark 70
    mark 99
    mark 101
    mark 199
    mark 105
    mark 116
    mark 199
}
module onebit;
%include 'exec.ins.pas';

type
  rec = record
    wf: integer;
    f: integer32;
  end;
  recp = ^rec;

var
  w: integer;
  l: integer32;
  r: rec;
  p: recp;

procedure onebit_$set(a: integer; b: integer32); options(val_param);
begin
  w := a;
  l := b;
  r.wf := a;
  r.f := b;
  p := addr(r);
end;

procedure onebit_$test;
begin
  if odd(w) then exec_$mark(1);
  if (w & 16#40) <> 0 then exec_$mark(6);
  if (w & 16#100) <> 0 then exec_$mark(8);
  if (16#4000 & w) <> 0 then exec_$mark(14);
  if (w & 2) = 0 then exec_$mark(20);
  if odd(l) then exec_$mark(31);
  if (l & 16#40) <> 0 then exec_$mark(36);
  if (l & 16#100) <> 0 then exec_$mark(38);
  if (l & 16#10000) <> 0 then exec_$mark(46);
  if (l & 16#40000000) <> 0 then exec_$mark(60);
  if not odd(l) then exec_$mark(70);
  exec_$mark(99);
end;

procedure onebit_$rec;
begin
  if odd(p^.wf) then exec_$mark(101);
  if (p^.f & 16#20) <> 0 then exec_$mark(105);
  if (rshft(p^.f, 4) & 16#10000) <> 0 then exec_$mark(116);
  exec_$mark(199);
end;
