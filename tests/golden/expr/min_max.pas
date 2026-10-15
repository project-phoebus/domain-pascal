{ Golden: `min' and `max' of run-time integers (ref "Min", p. 4-113,
  "Max", p. 4-111, 4.3.3 pp. 4-6..4-7; codegen-patterns 10.1; bead
  domain-pascal-bzkf): the first operand in the result's register, the
  second compared with it and moved over it.
  - add_pages: NETBUF_$ADD_PAGES 0xE0E95A..0xE0E982, `min(hdr_pages, 176 -
    netbuf_hdr_alloc)' (two `integer's: both widened, the difference in 32
    bits, `cmp.l D6,D3; ble.b; move.l D6,D3') and `min(data_pages,
    netbuf_$dat_lim - netbuf_$dat_cnt)' (a difference of longwords).
  - get_impure: MMAP_$GET_IMPURE 0xE0D624..0xE0D630, `min(100,
    wsl.page_count)' unsigned, `moveq #0x64,D2; cmp.l (0x30,A0),D2; bls;
    move.l (0x30,A0),D2', stored.
  - dat_copy: PKT_$DAT_COPY 0xE12850..0xE12858, `min(remaining, 1024)',
    `cmpi.w #0x400,D5; ble.b; move.w #0x400,D5'.
  - copy_packet: NET_IO_$COPY_PACKET 0xE0E612..0xE0E61A, `min(1024,
    left)' of a `0..65535' word, `move.w #0x400,D0; cmp.w D3,D0; bls.b;
    move.w D3,D0'.
  - del_pages: NETBUF_$DEL_PAGES 0xE0EB64..0xE0EB68, `max(x, 0)' on the
    subtraction's flags, `sub.l D4,D1; bpl.b; clr.l D1'. }
module min_max;
type
  pinteger = 0..65535;
  linteger = 0..2147483647;
  wsl_t = record
    a: integer32;
    page_count: linteger;
  end;
  wsl_ptr = ^wsl_t;
var
  hdr_alloc: integer;
  dat_lim, dat_cnt: integer32;
  total: integer;
  scan: linteger;
  chunk: integer;
  len: pinteger;
  free: integer;
procedure add_pages(hdr_pages, data_pages: integer); options(val_param);
var
  hdr_take, dat_take: integer;
begin
  hdr_take := min(hdr_pages, 176 - hdr_alloc);
  dat_take := min(data_pages, dat_lim - dat_cnt);
  total := hdr_take + dat_take;
end;
procedure get_impure(wsl: wsl_ptr); options(val_param);
begin
  scan := min(100, wsl^.page_count);
end;
procedure dat_copy(remaining: integer); options(val_param);
begin
  chunk := min(remaining, 1024);
end;
procedure copy_packet(left: pinteger); options(val_param);
begin
  len := min(1024, left);
end;
procedure del_pages(hdr_pages: integer); options(val_param);
begin
  free := max(hdr_pages - (hdr_alloc - 176), 0);
end;
