{ Golden: codegen-patterns 4, "Main programs" (bead domain-pascal-vq0h):
  another unit's variables through cells below the data base, the first named
  at (-4,A5), mirroring /com/acl: an 8-byte copy from one, `movea.l (-4,A5),A0;
  move.l (A0)+,(-24,A6)' (0x89D8, 0x89E2), and passed by address, `move.l
  (-12,A5),-(SP)' (0x8DC6). }
program cells_t;

type pair = record a, b: integer32; end;

var file_acl: extern pair;
    dir_acl: extern pair;
    nil_uid: extern pair;

procedure use(in v: pair; n: integer); extern;

procedure p(kind: integer);
var t: pair;
begin
  t := file_acl;
  t := dir_acl;
  use(nil_uid, kind);
end;

begin
  p(3);
end.
