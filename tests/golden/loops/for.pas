{ Golden: codegen-patterns 7.4, `for' loops with an index the body uses.
  The `dbf' count is loaded first (`moveq #n-1', `move.w #n-1' above
  127), then the index (`clr.w' for 0, `moveq' otherwise): MST_$ALLOC_ASID
  0xE42D50..0xE42D58, DISK_$UNASSIGN_ALL 0xE6BE24..0xE6BE26.  The index is
  stepped at the bottom before `dbf' (`addq.w #1'; `subq.w #1' for
  `downto', 0xE3C99A) and never stored to its frame slot.  A byte array
  is indexed with the index register itself (KBD's module-local 0xE1CA70
  `cmp.b (0,A5,D1.w),D2').  Across a call the count and the index take
  callee-saved registers (DISK_$UNASSIGN_ALL D2, D3).  A run-time upper
  bound from 1, in a body that calls: `subq.w #1; bmi' past the loop and
  the count copied into its own register (DTTY_$WRITE_STRING
  0xE1D6AA..0xE1D6B2).  `exit' leaves past the `dbf', `next' goes to the
  step. }
module fl;
var
  tab: array[0..7] of char;
  w: extern integer;
  len: extern integer;

procedure ext_$put(x: integer); options(val_param, extern);

procedure fl_$find; internal;
var
  i: integer;
begin
  for i := 0 to 7 do
    if tab[i] = 'x' then begin
      w := i;
      exit;
    end;
end;

procedure fl_$down; internal;
var
  i: integer;
begin
  for i := 100 downto 3 do begin
    if i = 50 then next;
    w := i;
  end;
end;

procedure fl_$calls; internal;
var
  i: integer;
begin
  for i := 1 to len do
    ext_$put(i);
end;

procedure fl_$count; internal;
var
  i: integer;
begin
  for i := -1 to 300 do
    ext_$put(5);
end;
