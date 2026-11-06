{ Golden: codegen-patterns 5.2, "String constants" (bead domain-pascal-vq0h):
  /com/acl 0x8292 `rights := '-----'' through a pointer, 0x82C4 an out
  parameter's 14 characters by bytes, 0x82FE and 0x832E a local's 22 and 9
  characters by longwords, NAME_$INIT 0xE31738 `path_buf := '/com''. }
module string_constants;

type r96 = array[1..96] of char;
     name_t = array[1..140] of char;

var path_buf: array[1..256] of char;

procedure p(in out rights: r96; out ent: name_t);
var special: r96;
begin
  rights := '-----';
  ent := '[from process]';
  special := '[specified by process]';
  special := '[ignored]';
  path_buf := '/com';
end;
