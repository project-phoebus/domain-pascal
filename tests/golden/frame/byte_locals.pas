{ codegen-patterns 2, "Frame layout": every frame object is at least word
  aligned.  A lone byte local takes the high byte of a word of its own
  (FILE_$REMOVE_WHEN_UNLOCKED 0xE5FC8E: `pea (-2,A6)' and `move.b
  (-2,A6),D0' in `link #-4'); byte locals are never packed two to a word
  (PROC2_$WAIT 0xE3FDD0: bytes at -150, -148, ..., -140); a longword
  after them is longword aligned.  Only the offsets are the point here
  (the image's copy-out after the call is not reproduced). }
module m;

procedure m_$probe1(var flag: boolean); extern;
procedure m_$probe3(var flag: boolean; var ch: char; var n: integer32); extern;

procedure m_$one;
var
  b: boolean;           { (-2,A6) }
begin
  m_$probe1(b);
end;

procedure m_$three;
var
  b: boolean;           { (-2,A6) }
  c: char;              { (-4,A6) }
  n: integer32;         { (-8,A6) }
begin
  m_$probe3(b, c, n);
end;
