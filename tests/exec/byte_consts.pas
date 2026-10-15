{ Execution test: byte constants passed by value (codegen-patterns 1.5;
  bead domain-pascal-g5xv).  A byte constant other than `true' and 0 is
  pushed as a word with the byte in the high half of its slot, `move.w
  #n<<8,-(SP)' (TTY 0xE1AFC0 `move.w #-0x200' for chr(254); SIO_$I_TSTART
  0xE1C7D6 `move.w #0x1300').  The callees read the byte at the slot's
  first address (codegen-patterns 1.3): a val_param char (EXEC_$LOGV_C),
  an internal routine taking a char, a `[byte]' 0..255 (a variable:
  two byte constants side by side are refused, merged or not is not
  known) and a `[byte]' -128..127, and a char next to `true' (SIO_$INIT 0xE32E60 `st -(SP)',
  0xE32E62 `move.w #0x1b00').  chr(0) clears the slot (0xE1AFD4).
  Expected values from Pascal semantics: ord of each character (ref
  4.1, `chr', `ord'), the signed byte -2 as -2.

  call: BYTE_CONSTS_$MAIN
  expect:
    254
    65
    0
    19
    65
    200
    -2
    27
    true
    90
    true
    10
    120
    128
    0
    -128
}
module byte_consts;
%include 'exec.ins.pas';

type
  ubyte = [byte] 0..255;
  sbyte = [byte] -128..127;

var
  w: integer;
  v: ubyte;

procedure put(c: char; u: ubyte; s: sbyte); internal;
begin
  w := ord(c);
  exec_$logv_w(w);
  w := u;
  exec_$logv_w(w);
  w := s;
  exec_$logv_w(w);
end;

procedure esc(c: char; f: boolean); internal;
begin
  w := ord(c);
  exec_$logv_w(w);
  exec_$log_bool(f);
end;

{ `true' before the char (pushed after it): separate pushes, no merge. }
procedure esc2(f: boolean; c: char); internal;
begin
  w := ord(c);
  exec_$logv_w(w);
  exec_$log_bool(f);
end;

procedure byte_consts_$main;
const
  lf = chr(10);
  ch = 'x';
begin
  exec_$logv_c(chr(254));
  exec_$logv_c('A');
  exec_$logv_c(chr(0));
  exec_$logv_c(chr(19));
  v := 200;
  put('A', v, -2);
  esc(chr(27), true);
  esc2(true, 'Z');
  exec_$logv_c(lf);
  exec_$logv_c(ch);
  v := 0;
  put(chr(128), v, -128);
end;
