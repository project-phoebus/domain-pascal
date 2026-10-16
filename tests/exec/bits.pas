{ Execution test: bit operators and functions (ref p. 4-33 `&', `!',
  `~'; 4-221 xor; 4-109 lshft, 4-171 rshft (logical), 4-29 arshft
  (arithmetic)) on words and longwords, negative operands included.
  (`rshft' of a signed word: tests/exec/shift_widen.pas.)

  call: BITS_$W w:-24 w:3
  call: BITS_$L l:-1048576 w:4
  expect:
    0x00000000
    0xffffffeb
    0x00000017
    0xffffffe8
    0xffffff40
    0xfffffffd
    0xffffffeb
    0xfff00000
    0xfff00004
    0x000fffff
    0xff000000
    0x0fff0000
    0xffff0000
    0xfff00004
}
module bits;
%include 'exec.ins.pas';

var
  w: integer;
  l: integer32;
  m: integer32;

procedure bits_$w(a: integer; n: integer); options(val_param);
begin
  w := a & n;
  l := w;
  exec_$log_x(l);
  w := a ! n;
  l := w;
  exec_$log_x(l);
  w := ~a;
  l := w;
  exec_$log_x(l);
  w := a & (-8);
  l := w;
  exec_$log_x(l);
  w := lshft(a, 3);
  l := w;
  exec_$log_x(l);
  w := arshft(a, 3);
  l := w;
  exec_$log_x(l);
  w := xor(a, n);
  l := w;
  exec_$log_x(l);
end;

procedure bits_$l(a: integer32; n: integer); options(val_param);
begin
  m := a & 16#fff0ffff;
  exec_$log_x(m);
  m := a ! n;
  exec_$log_x(m);
  m := ~a;
  exec_$log_x(m);
  m := lshft(a, n);
  exec_$log_x(m);
  m := rshft(a, n);
  exec_$log_x(m);
  m := arshft(a, n);
  exec_$log_x(m);
  m := xor(a, n);
  exec_$log_x(m);
end;
