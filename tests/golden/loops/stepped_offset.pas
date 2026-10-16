{ Golden: codegen-patterns 7.4 ("Arrays at an offset from their pointer";
  bead domain-pascal-m2m2), `for' loops stepping a pointer through an
  array that lies at an offset from the pointer variable (`p^.f[i]', a
  record parameter's field).  The pointer is formed as for an array at
  offset 0, `lea (from * size,An),Ap' off the pointer's register, and the
  array's offset goes into the displacements with the field's, less
  lo * size:
  - RING_$PROC2_CLEANUP 0xE76A80 `lea (0x8,A2),A3', the channels at 0x62 of
    u^, 8-byte elements from 1, reached at `(0x5a,A3)', `(0x5c,A3)';
  - XNS_IDP's module-local 0xE17DE0 `lea (0x4,A2),A4', pkt.data.pages at
    0x3C, 4-byte elements from 1, `tst.l (0x38,A4)';
  - MAC_$RECEIVE 0xE0BE5C `lea (A6),A0; lea (A3),A1', then `move.w
    (-0x2c,A0),(0x2,A1)': the source (the frame's) first, the arrays from
    0, pkt_desc.link_addr.addr at 2 (MAC_$DEMUX 0xE0BC8C the same, `lea
    (A2),A1', `(0x2,A1)');
  - a `with' over the element copies the pointer at the head (7.4,
    "`with`"), the references rebased by the array's offset. }
module so;
type
  chan_t = record
    flags: boolean;
    pad: char;
    asid: integer;
    sock: integer;
    ver: integer;
  end;
  unit_t = record
    hdr: array[0..48] of integer;
    channels: array[1..10] of chan_t;   { 0x62 }
  end;
  pages_t = record
    len: integer32;
    pages: array[1..4] of integer32;    { 0x04 }
  end;
  pkt_t = record
    n: integer;
    addr: array[0..2] of integer;       { 0x02 }
    pad: array[1..26] of integer;
    data: pages_t;                      { 0x3C }
  end;
  desc_t = record
    n_hops: integer;
    hops: array[0..2] of integer;       { 0x02 }
  end;

procedure ext_$close(sock: integer); options(val_param, extern);
procedure ext_$get(out d: desc_t); extern;

{ RING_$PROC2_CLEANUP 0xE76A80..0xE76AF8 (its body cut down). }
procedure so_$cleanup(var u: unit_t; asid: integer);
  options(val_param);
var
  ch: integer;
begin
  for ch := 1 to 10 do
    if u.channels[ch].flags and then (asid = u.channels[ch].asid) then begin
      u.channels[ch].flags := false;
      ext_$close(u.channels[ch].sock);
    end;
end;

{ XNS_IDP's module-local 0xE17DDC..0xE17E08 (the element's test). }
function so_$count(var pkt: pkt_t): integer;
var
  page, n: integer;
begin
  n := 0;
  for page := 1 to 4 do
    if pkt.data.pages[page] = 0 then
      n := n + 1;
  so_$count := n;
end;

{ MAC_$RECEIVE 0xE0BE4A..0xE0BE6A (a constant bound here). }
procedure so_$receive(var pkt: pkt_t);
var
  info: desc_t;
  i: integer;
begin
  ext_$get(info);
  pkt.n := info.n_hops;
  for i := 0 to 2 do
    pkt.addr[i] := info.hops[i];
end;

{ `with' over a stepped element at an offset from its pointer. }
procedure so_$reset(var u: unit_t);
var
  ch: integer;
begin
  for ch := 1 to 10 do
    with u.channels[ch] do begin
      flags := false;
      ver := 0;
    end;
end;
