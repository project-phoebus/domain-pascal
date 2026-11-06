{ Golden: codegen-patterns 4, "Main programs" (bead domain-pascal-vq0h): `addr'
  of a character constant (ref p. 4-13): `lea (d,PC),An' and a temporary passed
  by address, /com/args 0x8260, 0x829A; its `chr(10)' cell. }
program addr_const;

const nl = chr(10);

procedure put(in p: univ_ptr; in n: integer32); extern;

begin
  put(addr(nl), 1);
end.
