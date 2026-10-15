{ Golden: codegen-patterns 1.5, "Zero" (bead domain-pascal-73sx): `nil'
  passed by value is the pointer 0, pushed as any longword 0, `clr.l
  -(SP)': the port pointer of the module-local NET_IO_$PUT_IN_SOCK_COMMON
  at NET_IO_$PUT_IN_SOCK_INT 0xE0E506 (and NET_IO_$PUT_IN_SOCK 0xE0E4CC),
  pushed last, after word values; `true' beside it is `st -(SP)'
  (0xE0E4FC).  A `nil' next to another constant argument has no site and
  is refused. }
module nio;
type
  lptr = ^integer32;

procedure nio_$common(port: lptr; net_type: integer; socket: lptr;
                      int: boolean; hdr: integer32; sock: integer);
  options(val_param, extern);

procedure nio_$put_int(in net_type: integer; in socket: lptr;
                       in hdr: integer32; in sock: integer);
  options(val_param);
begin
  nio_$common(nil, net_type, socket, true, hdr, sock);
end;
