{ Golden: codegen-patterns 7, `for' loops over an array whose index is
  used for nothing else: module-local 0xE00EB0, 0xE00ECC..0xE00EDA
  `movea.l #AST_$ZERO_BUFF,A0; move.w #0xFF,D0; movea.l A0,A0; clr.l
  (A0)+; dbf'.  The first element's address, then the count less one
  (`moveq' up to 127, else `move.w'), the 0x2048 filler before the head,
  `clr.<s> (A0)+' at the element's size.  A 1-based array folds the
  lower bound into the start (codegen-patterns 5.2).  A statement after
  a call takes the lowest free scratch registers again. }
module fill;
var
  page: extern array[0..255] of integer32;
  words: extern array[1..32] of integer;
  flags: extern array[1..3] of boolean;

procedure ext_$p; extern;

procedure fill_$page;
var
  i: integer;
begin
  for i := 0 to 255 do page[i] := 0;
end;

procedure fill_$small;
var
  i: integer;
begin
  ext_$p;
  for i := 1 to 32 do words[i] := 0;
  ext_$p;
  for i := 1 to 3 do flags[i] := false;
end;
