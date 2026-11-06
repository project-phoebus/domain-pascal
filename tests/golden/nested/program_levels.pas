{ Golden: codegen-patterns 6, "Frame: a 4-byte static-link slot" (bead
  domain-pascal-vq0h): one 4-byte slot above the locals per enclosing routine,
  /com/acl's list_it `link #-8', nested routines at the third level
  list_req `link #-0x12C' and dolist_subs `link #-0x118' with 12 bytes above
  their locals. }
program program_levels;

procedure use(var x: integer32); extern;

procedure one;
var a: integer32;

  procedure two;
  var b: integer32;

    procedure three;
    var c: integer32;
    begin
      use(c);
    end;

  begin
    three;
    use(b);
  end;

begin
  two;
  use(a);
end;

begin
  one;
end.
