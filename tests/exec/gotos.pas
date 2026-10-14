{ Execution test: `goto' and labels (ref 4.2 "Goto", p. 4-98;
  tests/golden/control/goto.pas): forward and backward, out of a `while'
  and a `for', into a `case' arm, a function result set before a
  `goto' to the end.

  call: GOTOS_$FORWARD w:0
  call: GOTOS_$FORWARD w:1
  call: GOTOS_$BACKWARD w:3
  call: GOTOS_$OUT_WHILE w:4
  call: GOTOS_$OUT_WHILE w:-4
  call: GOTOS_$OUT_FOR w:3
  call: GOTOS_$OUT_FOR w:20
  call: GOTOS_$ARMS w:1 w:0
  call: GOTOS_$ARMS w:1 w:1
  call: GOTOS_$ARMS w:3 w:0
  call: GOTOS_$RESULT w:0 -> w
  call: GOTOS_$RESULT w:1 -> w
  expect:
    mark 1
    mark 2
    mark 3
    mark 3
    mark 0
    mark 1
    mark 2
    mark 3
    mark 4
    mark 40
    mark 10
    mark 1
    mark 2
    mark 3
    mark 41
    mark 1
    mark 2
    mark 3
    mark 4
    mark 5
    mark 6
    mark 7
    mark 8
    mark 9
    mark 10
    mark 42
    mark 4
    mark 5
    mark 2
    mark 4
    mark 5
    mark 3
    => 2
    => 1
}
module gotos;
%include 'exec.ins.pas';

var
  w: integer;
  pad: integer32;
  r: integer;
  b: boolean;

procedure gotos_$forward(k: integer); options(val_param);
label
  done;
begin
  w := k;
  if w <> 0 then goto done;
  exec_$mark(1);
  exec_$mark(2);
done:
  exec_$mark(3);
end;

procedure gotos_$backward(k: integer); options(val_param);
label
  again;
begin
  w := 0;
  r := k;
again:
  exec_$mark(w);
  w := w + 1;
  if w <= r then goto again;
end;

procedure gotos_$out_while(k: integer); options(val_param);
label
  found;
begin
  r := k;
  w := 0;
  while w < 10 do begin
    if r = w then goto found;
    w := w + 1;
  end;
  exec_$mark(40);
found:
  exec_$mark(w);
end;

procedure gotos_$out_for(k: integer); options(val_param);
label
  bad;
var
  i: integer;
begin
  r := k;
  for i := 1 to 10 do begin
    exec_$mark(i);
    if i = r then goto bad;
  end;
  exec_$mark(42);
  return;
bad:
  exec_$mark(41);
end;

procedure gotos_$arms(k: integer; f: integer); options(val_param);
label
  common;
begin
  w := k;
  if f <> 0 then b := true else b := false;
  case w of
    0: exec_$mark(1);
    1: if b then exec_$mark(2) else goto common;
    2: exec_$mark(3);
    3: common: begin exec_$mark(4); exec_$mark(5) end;
  end;
end;

function gotos_$result(k: integer): integer; options(val_param);
label
  done;
begin
  gotos_$result := 1;
  w := k;
  if w <> 0 then goto done;
  exec_$mark(3);
  gotos_$result := 2;
done:
end;
