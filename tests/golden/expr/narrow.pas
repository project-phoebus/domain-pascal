{ Golden: integer values of another size than their target
  (codegen-patterns 8, "Narrowing and widening"; bead domain-pascal-8td).
  - A longword field of a record passed by address, stored into a word:
    its low word read in place, APP_$DEMUX 0xE00AF6 `move.w
    (0x1a,A0),(-0x14,A6)` (`rec.hdr_len := pkt.data_len`, the field at
    0x18).
  - A word `in` parameter (by address) stored into a byte: PACCT_$LOG
    0xE5AB26 `movea.l (0x10,A6),A0; move.b (1,A0),(-0x18e,A6)`.
  - A longword local in the frame passed to a word value parameter: its
    low word (MMAP_$FREE_PAGES 0xE0CF2C `move.w (-0x12,A6),(-0x1ffa,A4)`
    for a longword local at -0x14).  The local is a word result
    zero-extended (0xE1768A `clr.l D5; move.w D0,D5` after ML_$SPIN_LOCK,
    codegen-patterns 8).
  - A longword held in a register passed to a word value parameter:
    LOG_$ADD 0xE1775E `move.w D5,-(SP)`.
  - `rshft` of a longword and `&` of it passed as words: DBUF_$GET_BLOCK
    0xE3A5EC..0xE3A5F6 `moveq #31,D0; and.w D3,D0; move.w D0,-(SP);
    move.l D3,D1; lsr.l #5,D1; move.w D1,-(SP)` (the `&` at word width,
    the shift at longword width).
  - A product of a longword `rshft` stored into a word: DBUF_$INIT
    0xE3ABE8..0xE3ABF4 `move.l abs,D0; lsr.l #8,D0; lsr.l #2,D0; lsl.l
    #4,D0; move.w D0,(0x91a,A5)`.
  - A word plus a longword stored into the word: MAC_OS_$SEND
    0xE0B650..0xE0B658 `move.w (-62,A6),D0; ext.l D0; add.l (A1),D0;
    move.w D0,(-62,A6)`.
  - Widened for value parameters: a `0..65535` field to an integer32,
    APP_$RECEIVE 0xE008E6..0xE008EC `clr.l D1; move.w (994,A3),D1; move.l
    D1,-(SP)`; a `[byte] 0..255` field to an integer, SOCK_$CLOSE
    0xE16008..0xE1600E `clr.w D1; move.b (27,A2),D1; move.w D1,-(SP)`. }
module nw;
type
  pinteger = 0..65535;
  byte_t = [byte] 0..255;
  pkt_t = record
    pad: array[1..24] of char;
    data_len: integer32;
  end;
  info_t = record
    pad: array[1..20] of char;
    hdr_len: pinteger;
  end;
  sock_t = record
    pad: array[1..26] of char;
    hdr_pages: byte_t;
    data_pages: byte_t;
    port: pinteger;
  end;
  sock_ptr_t = ^sock_t;
  lp_t = ^integer32;
var
  real_pages: extern integer32;
  count: integer;
  stat: byte_t;

procedure q(w: integer); options(val_param, extern);
procedure q2(a, b: integer); options(val_param, extern);
procedure ql(w: integer; l: integer32); options(val_param, extern);
function spin_lock: pinteger; extern;
procedure work; extern;

procedure nw_$demux(in pkt: pkt_t);
var
  rec: info_t;
begin
  rec.hdr_len := pkt.data_len;
  q(rec.hdr_len);
end;

procedure nw_$log(in exit_status: integer);
begin
  stat := exit_status;
end;

procedure nw_$unlock;
var
  token: integer32;
begin
  token := spin_lock;
  work;
  q(token);
end;

procedure nw_$hint(block_hint: integer32); options(val_param);
var
  w: integer;
begin
  q2(rshft(block_hint, 5), block_hint & 16#1F);
  w := block_hint & 16#3F;
  q(w);
  q(block_hint);
end;

procedure nw_$init;
begin
  count := rshft(real_pages, 10) * 16;
end;

procedure nw_$send(var n: integer; in p: lp_t);
begin
  n := n + p^;
end;

procedure nw_$widen(s: sock_ptr_t); options(val_param);
begin
  ql(1, s^.port);
  q2(s^.hdr_pages, s^.data_pages);
end;
