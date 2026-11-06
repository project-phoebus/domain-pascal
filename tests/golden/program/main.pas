{ Golden: codegen-patterns 4, "Main programs" (bead domain-pascal-vq0h),
  mirroring /com/acl: procedures with no A5 load and `bsr` (0x89B4, called at
  0x8DCA), a small value parameter by value (0x8DBE `move.w (-6,A6),-(SP)',
  read at 0x89D0 `cmpi.w #3,(16,A6)') and a large one by address with a local
  copy (0x89BE), and the body last as the exported entry (0x8ED8), its data
  base loaded after saving A5 (0x8EE0). }
program main_t;

type pair = record a, b: integer32; end;

var x: integer32;

procedure pgm_$exit; extern;

procedure p(q: pair; n: integer);
begin
  x := q.b + n;
end;

var c: pair;

begin
  p(c, 3);
  pgm_$exit;
end.
