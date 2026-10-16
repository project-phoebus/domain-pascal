{ Golden: assignments that narrow an element, a pointer's target, a
  `with` record, a packed field or a function's result (codegen-patterns
  8, "Narrowing and widening"; bead domain-pascal-d59a).  The source's low
  bytes are read in place, at the higher address (big-endian):
  - an `in` array's element at a constant index into a word field:
    MMAP's module-local 0xE0C5AE at 0xE0C602 `move.w
    (0x6,A0),(-0x1ffa,A2)` (`first_page.prev_vpn := vpn_array[2]`);
  - a frame array's element at a run-time index: NETBUF_$ADD_PAGES
    0xE0EAA8 `move.w (-0x20e,A2),(-0x1ffa,A3)` (`pages[j + 1]`);
  - a `with` record's longword field: MMAP_$FREE_PAGES 0xE0CFCC `move.w
    (0xe,A0),(-0x1ffa,A2)` (`free_pool.head_vpn`);
  - a pointer's target, a word into a byte: RING's module-local 0xE76048
    at 0xE760F4 `move.b (0x33,A3),(0x4,A0)` (`hw_regs^.tmask :=
    unit_data^.tmask`);
  - an `in` record's packed word field and its element at a run-time
    index into bytes: RINGLOG_$LOGIT 0xE1A3A0 `move.b
    (0x39,A3),(0x3,A2)` (`hdr.dest.sock`), 0xE1A3F4 `move.b
    (0x1f,A3,D1.w),(0x3,A2)` (`hdr.route[hdr.route_count + 1]`; here
    without the `+ 1`, whose folding into the displacement is another
    matter);
  - a longword parameter by value into a byte: DISK_IO 0xE3D630 `move.b
    (0x13,A6),(0x6,A3)` (`req^.daddr.head := daddr`).
  A function's result is stored from its register at the narrow width:
  RIP_$TABLE_D 0xE68F24 `move.b D0,(0xe,A4)` after ROUTE_$FIND_PORT (a
  word); NETWORK 0xE0F526 `move.w D0,(0x344,A5)` after MMAP_$REMOTE_POOL
  (a longword). }
module ns;
type
  pinteger = 0..65535;
  byte_t = [byte] 0..255;
  vpn_list_t = array[1..64] of integer32;
  mmape_t = record
    pad: array[1..6] of char;
    prev_vpn: pinteger;
    next_vpn: pinteger;
  end;
  wsl_t = record
    page_count: integer32;
    pad: array[1..8] of char;
    head_vpn: integer32;
  end;
  unit_t = record
    pad: array[1..50] of char;
    tmask: pinteger;
  end;
  unit_ptr_t = ^unit_t;
  regs_t = record
    pad: array[1..4] of char;
    tmask: byte_t;
  end;
  regs_ptr_t = ^regs_t;
  inet_t = packed record
    net: integer32;
    zero: 0..16#FFFFFF;
    node: 0..16#FFFFFF;
    sock: pinteger;
  end;
  hdr_t = record
    pad: array[1..25] of char;
    route_count: byte_t;
    route: array[1..10] of pinteger;
    dest: inet_t;
  end;
  entry_t = record
    sock_02: byte_t;
    sock_03: byte_t;
  end;
  daddr_t = record
    cyl: pinteger;
    head: byte_t;
    sector: byte_t;
  end;
  req_t = record
    pad: array[1..4] of char;
    daddr: daddr_t;
  end;
  req_ptr_t = ^req_t;
var
  mmap: array[1..8] of mmape_t;
  wsl: array[0..1] of wsl_t;
  entry: entry_t;
  port: byte_t;
  remote_pool: integer;

function route_$find_port(port_type: integer; socket: integer32): integer;
  options(val_param, extern);
function mmap_$remote_pool(param: integer32): integer32; options(val_param, extern);

procedure ns_$add_to_wsl(in vpn_array: univ vpn_list_t);
begin
  mmap[1].prev_vpn := vpn_array[2];
end;

procedure ns_$add_pages(j: integer); options(val_param);
var
  pages: array[1..16] of integer32;
begin
  pages[1] := 0;
  mmap[2].prev_vpn := pages[j + 1];
end;

procedure ns_$free_pages(page: integer); options(val_param);
begin
  with wsl[1]: free_pool do
    mmap[page].prev_vpn := free_pool.head_vpn;
end;

procedure ns_$set_tmask(hw_regs: regs_ptr_t; unit_data: unit_ptr_t); options(val_param);
begin
  hw_regs^.tmask := unit_data^.tmask;
end;

procedure ns_$logit(in hdr: hdr_t);
begin
  entry.sock_03 := hdr.dest.sock;
  entry.sock_02 := hdr.route[hdr.route_count];
end;

procedure ns_$io(req: req_ptr_t; daddr: integer32); options(val_param);
begin
  req^.daddr.head := daddr;
end;

procedure ns_$table(socket: integer32); options(val_param);
begin
  port := route_$find_port(1, socket);
end;

procedure ns_$set_service(param: integer32); options(val_param);
begin
  remote_pool := mmap_$remote_pool(param);
end;
