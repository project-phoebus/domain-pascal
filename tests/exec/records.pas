{ Execution test: record and array copies (codegen-patterns 5.2, 7:
  unrolled `move.l (A0)+,(A1)+' for small blocks, a `dbf' loop for large
  ones) of initialised data (codegen-patterns 11), through pointers and var parameters, with field
  compares after the copy.  (dpc does not yet compile `with'.)

  call: RECORDS_$MAIN w:3
  expect:
    mem 00 00 00 03 00 04 00 05 ff ff ff ff
    mem 00 00 00 03 00 04 00 05 ff ff ff ff
    mem 00 07 00 08 00 09 00 0a 00 0b 00 0c 00 0d
    mem 00 07 00 08 00 09 00 0a 00 0b 00 0c 00 0d
    true
    mem 00 03 00 03 00 03 00 03 00 03 00 03 00 03 00 03 00 03 00 03 00 03 00 03 00 03 00 03 00 03 00 03 00 03 00 03 00 03 00 03
    3
    mark 40
    false
}
module records;
%include 'exec.ins.pas';

type
  small = record
    l: integer32;
    a: integer;
    b: integer;
    c: integer32;
  end;
  seven = record
    w: array[1..7] of integer;
  end;
  big = array[1..72] of integer;
  smallp = ^small;
var
  s1, s2: small;
  e1: seven := [w := [7, 8, 9, 10, 11, 12, 13]];
  sep1: integer32;
  e2: seven;
  b1: big := [72 of 3];
  b2: big;
  same: boolean;

procedure copy_in(var dst: small; in src: small);
begin
  dst := src;
end;

procedure records_$main(k: integer); options(val_param);
var
  p: smallp;
begin
  s1.l := k;
  exec_$clobber;
  s1.a := 4;
  exec_$clobber;
  s1.b := 5;
  exec_$clobber;
  s1.c := -1;
  copy_in(s2, s1);
  exec_$log_mem(s2, 12);
  p := exec_$id_p(addr(s1));
  exec_$log_mem(p^, 12);
  e2 := e1;
  exec_$log_mem(e1, 14);
  exec_$log_mem(e2, 14);
  if e2.w[7] = e1.w[7] then same := true else same := false;
  exec_$log_bool(same);
  b2 := b1;
  exec_$log_mem(b2[53], 40);
  exec_$logv_l(s2.l);
  s2.a := s2.a * 10;
  exec_$mark(s2.a);
  if s2.a = s1.a then same := true else same := false;
  exec_$log_bool(same);
end;
