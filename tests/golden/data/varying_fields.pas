{ ref 3.11.2 (p. 3-45): a varying string's `length' (a word at offset 0)
  and `body' (a packed array[1..max] of char at offset 2) are referenced
  "using the same syntax and semantics you would use for a normal record";
  bead domain-pascal-cze5.  No image site: the SR10.2 SAU2 kernel has no
  varying string reference catalogued; the forms are the record field and
  array element forms of codegen-patterns 5.2. }
module varying_fields;
type
  vs_t = varying[20] of char;
procedure p(var v: vs_t; var n: integer; var c: char);
begin
  n := v.length;
  v.length := 3;
  c := v.body[v.length];
  v.body[1] := 'a';
  if v.length > 0 then n := n + 1;
end;
