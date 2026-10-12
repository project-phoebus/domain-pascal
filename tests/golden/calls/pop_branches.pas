{ Golden: codegen-patterns 1.1, the pop rule with branches.  The caller
  does not pop a call's arguments when every path from the call to the
  epilogue is free of pushes, calls and conditional branches (a `bra'
  to a join may occur): 1135 statement-level calls in compiled code,
  none popped (0xE01166 `jsr ML_$UNLOCK; bra.s 0xE01176', a join
  followed by stores and the epilogue).  It pops when a push, a call or
  a conditional branch (an `if', a loop test, a `dbf') follows on some
  path: 2729 + 1742 + 240 calls, all popped (0xE00BCA `jsr; addq.w
  #8,SP', then an `if').  A `return' ends the path. }
module popb;
var
  w: extern integer;
  r: extern integer;
  page: extern array[0..15] of integer32;

procedure ext_$p(x: integer); options(val_param, extern);

{ popped: an `if' follows }
procedure popb_$if_follows; internal;
begin
  ext_$p(1);
  if w = 0 then r := 1;
end;

{ not popped in the `then' part: a `bra' to the join, then stores }
procedure popb_$in_parts; internal;
begin
  if w = 0 then begin
    ext_$p(2);
  end else begin
    r := 1;
  end;
  r := 2;
end;

{ the `then' part's call is followed by a call after the join: popped }
procedure popb_$call_after_join; internal;
begin
  if w = 0 then ext_$p(3);
  ext_$p(4);
end;

{ not popped: `return' ends the path }
procedure popb_$return; internal;
begin
  if w = 0 then begin
    ext_$p(5);
    return;
  end;
  ext_$p(6);
  r := 1;
end;

{ popped: a `dbf' loop follows }
procedure popb_$loop_follows; internal;
var
  i: integer;
begin
  ext_$p(7);
  for i := 0 to 15 do page[i] := 0;
end;
