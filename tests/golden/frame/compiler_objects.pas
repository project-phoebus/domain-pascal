{ codegen-patterns 2, "Where the compiler's objects go" (bead
  domain-pascal-67c): the declared locals in declaration order, and each
  object the compiler makes, a value parameter's copy or the function
  result variable, in front of the first local that takes fewer frame
  bytes than it (a byte counts as its word), or after them all; the
  larger first, and the copies of one size at rising addresses in
  parameter order.

  - next_long_id: PKT_$NEXT_LONG_ID 0xE124DC (`link #-8'): a 4-byte
    result held in D2, its slot above the word token at (-6,A6) (`move.w
    D0w,(-0x6,A6)'); NETBUF_$RTNVA 0xE0ED26 has the same frame;
  - next_id: PKT_$NEXT_ID 0xE1248E (`link #-4'): a word result below the
    word token at (-2,A6);
  - send: MSG_$SEND 0xE599FC (`link #-0x34'), its copies at entry
    (0xE59A0A..0xE59A3C): the longword parameter 1 at -0x28 and the six
    words, parameters 2..9, at -0x34 .. -0x2A (parameter 2 lowest), below
    its locals: the 30-byte template at -0x20 and 4 bytes nothing
    addresses at -0x24 (here a longword local);
  - ptrace: XPD_$SET_PTRACE_OPTS 0xE5AF9E: the 14-byte copy at -0x10
    above the UID's at -0x18 (0xE5AFA4..0xE5AFBE), both over the
    longword local at -0x1C (`clr.l (-28,A6)', 0xE5AFC0). }
module compiler_objects;
type
  uid_$t = record high, low: integer32; end;
  opts_t = record a, b, c: integer32; d: integer; end;
  tmpl_t = record w: array[1..15] of integer; end;   { 30 bytes }
var
  lock: integer32;
  next_long: integer32;
  next_short: integer;

function ml_$spin_lock(var l: integer32): integer; extern;
procedure ml_$spin_unlock(var l: integer32; token: integer);
  options(extern, val_param);
procedure msg_$sendi(var r: tmpl_t; a: integer32; b, c, d, e, f, g: integer;
                     var l: integer32);
  options(extern, val_param);
procedure xpd_$take(var u: uid_$t; var o: opts_t; var n: integer32); extern;

function pkt_$next_long_id: integer32;
var
  token: integer;
begin
  token := ml_$spin_lock(lock);
  pkt_$next_long_id := next_long;
  next_long := next_long + 1;
  ml_$spin_unlock(lock, token);
end;

function pkt_$next_id: integer;
var
  token: integer;
begin
  token := ml_$spin_lock(lock);
  pkt_$next_id := next_short;
  next_short := next_short + 1;
  ml_$spin_unlock(lock, token);
end;

procedure msg_$send(a: integer32; b, c, d, e, f, g: integer);
var
  tmpl: tmpl_t;
  l: integer32;
begin
  msg_$sendi(tmpl, a, b, c, d, e, f, g, l);
end;

procedure xpd_$set_ptrace_opts(u: uid_$t; o: opts_t);
var
  n: integer32;
begin
  n := 0;
  xpd_$take(u, o, n);
end;
