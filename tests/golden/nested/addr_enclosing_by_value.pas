{ Golden: codegen-patterns 1.5, `addr(x)' by value of the enclosing
  routine's local from a nested routine (bead domain-pascal-fq1y;
  ../source gap 357): `pea (d,An)' off the static link's register, the
  push of the same object by address (SAU9 ETHERNET_$SVC_IOCTL's nested
  routine 0x3C483490 `pea (-0x40,A3)' to a module-local function).  Here
  the nested routine is called by its parent only, so the link is the
  dynamic link `(A6)' (section 6). }
module addr_enclosing_by_value;
type
  rec = record a: integer; b: array[1..31] of integer; end;
  recp = ^rec;

function f(p: recp; c: integer; out st: integer32): boolean; internal;
begin st := c; f := true; end;

procedure outer(var x: integer; out st: integer32);
var pcb: rec;
  procedure inner(c: integer; out st: integer32);
  begin discard(f(addr(pcb), c, st)); end;
begin inner(x, st); end;
