{ Golden: codegen-patterns 1.5, 7: in a routine that does not align its
  loop heads (`dpc --loop-align=off', const_pool_noalign.args) the
  longword cells follow the word cells with no filler (ACL_$SET_ACL_CHECK
  0xE47878: the word 0000, then the longword 0000000f at 0xE4787A, 2 mod
  4), and the module end is padded with a zero word. }
module cpoolna;
procedure ext_$rights(a: integer; in b: integer32); extern;

procedure cpoolna_$check;
begin
  ext_$rights(0, 16#f);
end;
