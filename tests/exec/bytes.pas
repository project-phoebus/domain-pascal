{ Execution test: bytes (codegen-patterns 1.3, 5.2 "Byte indices"):
  `char' and boolean fields at odd offsets, an array indexed by a char
  (zero-extended: chars above 127 are not negative), a byte value
  argument in the high half of its word slot, booleans as 0 / 0xFF
  (ref 3.5.3), `ord' and `chr'.

  call: BYTES_$MAIN b:200 b:255
  call: BYTES_$MAIN b:65 b:0
  expect:
    200
    true
    mem 00 c8 ff 00
    17
    200
    mark 1
    65
    false
    mem 00 41 00 00
    17
    65
}
module bytes;
%include 'exec.ins.pas';

type
  rec = record
    pad: char;
    c: char;
    f: boolean;
    pad2: char;
  end;
var
  r: rec;
  classes: array[char] of integer;
  w: integer;

procedure bytes_$main(c: char; f: boolean); options(val_param);
begin
  r.c := c;
  exec_$log_c(r.c);
  r.f := f;
  exec_$log_bool(r.f);
  exec_$log_mem(r, 4);
  classes[c] := 17;
  w := classes[r.c];
  exec_$logv_w(w);
  w := ord(c);
  exec_$logv_w(w);
  if w > 127 then exec_$mark(1);
end;
