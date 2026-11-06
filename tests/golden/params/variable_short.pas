{ Golden: codegen-patterns 1.4, "A call with fewer arguments" (bead
  domain-pascal-vq0h; ref 5.5.5, p. 5-17): the arguments given are pushed as
  usual and popped by their total, /com/acl 0x8218..0x8236
  `error_$print_format', six of its sixteen, `lea (24,SP),SP'. }
module variable_short;

procedure q(in a, b, c, d: integer32); options(variable, extern);

procedure p(in x, y: integer32);
begin
  q(x, y);
end;
