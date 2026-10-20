{ Golden: codegen-patterns 11, "Overlay sections", a `define'd variable
  in an overlay section (bead domain-pascal-m15q).  ref 3.14, p. 3-52: a
  `define' of an `extern' variable of a `var (name)' part "will be stored
  in section_name".  The shape of ROUTE_$PORT 0xE2E0A0, the start of the
  map's NET_PORT_TABLE (size 0x2E0, eight 0x5C-byte port records;
  ../source route/route.ins.pas, route_wired.pas): the unit defines the
  section, NOBITS `.bss.overlay.NET_PORT_TABLE' with NET_PORT_TABLE and
  ROUTE_$PORT global labels at +0, a variable declared `define' in the
  part a global label at its offset; one with initial values in
  `.data.overlay.<SYM>', its labels among the bytes.  The variables are
  still offsets into the section's symbol, as in any other unit.  Best
  effort (not byte-identical): which unit defined NET_PORT_TABLE is not
  known (the image's NET_PORT_TABLE holds nonzero bytes). }
module route_wired, wired_proc, wired_data;
type
  port_t = record
    network: integer32;
    rest: array[1..16#2C] of integer;
  end;
var (net_port_table)
  route_$port: extern array[0..7] of port_t;
var (init_sec)
  first: integer := 1;
  ovd_$k: define integer32 := -1;
  ovd_$p: define ^integer32 := addr(route_$port[1].network);

define route_$port;
define route_$net;

function route_$net(in i: integer): integer32;
options(val_param);
begin
  route_$net := route_$port[i].network + ovd_$k;
end;
