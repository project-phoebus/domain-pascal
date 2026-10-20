{ Execution test (bead domain-pascal-uaq7): best-effort forms for kinds
  the link test's ranking refused.

  The address of an element of another unit's array, or of an array
  through a pointer loaded from memory, at a displacement beyond a byte,
  as a value (codegen-patterns 5.2, "The element's address as a value"):
  `lea (0,An,Dx),Am; lea (d,Am),Ar' (DBUF_$INIT 0xE3AC48 steps a pointer
  instead).  Stored into a local, a static and a record field, passed as
  a value argument.

  link: be_u_data.mod.pas
  call: BU_$FILL
  call: BU_$ADDR_A w:2 -> l
  call: BU_$ADDR_A w:4 -> l
  call: BU_$ADDR_THRU w:3 -> l
  call: BU_$ADDR_STORE w:1 -> l
  call: BU_$ADDR_ARG w:4
  call: BU_$PCMP w:2 -> w
  call: BU_$PCMP w:1 -> w
  call: BU_$PCMP w:3 -> w
  call: BU_$ADDR_ARITH w:3 l:5 -> l
  call: BU_$ADDR_ARITH w:1 l:-9 -> l
  call: BU_$ADDR_MASK w:2 l:74565 -> l
  call: BU_$ADDR_BITS -> l
  call: BU_$PRESULT w:2 -> l
  call: BU_$CALLCMP w:2 l:65535 -> w
  call: BU_$CALLCMP w:4 l:5 -> w
  call: BU_$CALLCMP w:-1 l:0 -> w
  call: BU_$CALLCMP w:-2 l:65535 -> w
  call: BU_$INSET w:3 -> w
  call: BU_$INSET w:7 -> w
  call: BU_$INSET w:17 -> w
  call: BU_$INSET w:-1 -> w
  call: BU_$INSET w:15 -> w
  call: BU_$INSET w:0 -> w
  call: BU_$INSET w:23 -> w
  call: BU_$UZERO w:0 -> w
  call: BU_$UZERO w:65535 -> w
  call: BU_$UZERO w:8 -> w
  call: BU_$UZERO w:2 -> w
  call: BU_$SZERO w:0 l:0 -> w
  call: BU_$SZERO w:5 l:7 -> w
  call: BU_$SZERO w:32767 l:2147483647 -> w
  call: BU_$STRS
  call: BU_$BOOLS w:0 -> w
  call: BU_$BOOLS w:3 -> w
  call: BU_$BOOLS w:7 -> w
  call: BU_$BOOLS w:2 -> w
  warns: best effort (not byte-identical): comparing boolean values
  call: BU_$MUL3 l:3 l:-5 l:6 -> l
  call: BU_$CASE_EMPTY w:2 -> w
  call: BU_$CASE_EMPTY w:3 -> w
  warns: best effort (not byte-identical): an M$ARITH call with both operands computed
  warns: best effort (not byte-identical): an empty `otherwise` part
  call: BU_$WIDEN b:200 b:-3 -> l
  warns: best effort (not byte-identical): widening a byte to a longword
  call: BU_$LMOD l:-1000007 w:1000 -> l
  call: BU_$LMOD l:2000000 w:-1000 -> l
  call: BU_$LMOD l:99999 w:32767 -> l
  warns: best effort (not byte-identical): a signed `mod` by a word used as a longword
  call: BU_$FORRT w:3 w:5 -> l
  call: BU_$FORRT w:6 w:4 -> l
  call: BU_$FORRT w:1 w:1 -> l
  call: BU_$FORU w:4 -> l
  call: BU_$FORU w:0 -> l
  call: BU_$LBYTE l:1000123 -> w
  call: BU_$LBYTE l:-1000123 -> w
  warns: best effort (not byte-identical): a longword `div`/`mod` used as a byte
  warns: best effort (not byte-identical): a `for` loop with a run-time lower bound
  warns: best effort (not byte-identical): a `for` loop counting down (`downto`) with a run-time bound
  warns: best effort (not byte-identical): assigning a string constant to an array of char
  warns: best effort (not byte-identical): assigning a record or array that is not a variable
  warns: best effort (not byte-identical): an unsigned ordering comparison with 0
  warns: best effort (not byte-identical): `in` on a one-word set variable
  warns: best effort (not byte-identical): a comparison with a function's result in the right operand
  warns: best effort (not byte-identical): a pointer result other than one read from memory
  warns: best effort (not byte-identical): an address that needs an address register and a value
  warns: best effort (not byte-identical): an address minus a value
  warns: best effort (not byte-identical): an address and a constant in a sum
  warns: best effort (not byte-identical): `!` or `xor` of an address
  warns: best effort (not byte-identical): the address of an element of another unit's array
  warns: best effort (not byte-identical): comparing a pointer variable with a computed pointer
  warns: best effort (not byte-identical): an address (`addr`) in a comparison
  expect:
    => 227
    => 447
    => 307
    => 117
    814
    => 11
    => 110
    => 110
    1000
    501
    => 1001
    1000
    501
    => 519
    => 1085
    1
    6
    => 1
    => 1258
    => 111
    => 0
    => 10
    => 1010
    => 111
    => 1000
    => 10
    => 10
    => 111
    => 1000
    => 10
    => 170
    => 153
    => 169
    => 153
    => 426
    => 153
    => 153
    mem 61 62 63 20 20 20 20 20 20
    mem 2f 63 6f 6d 20 20
    32
    33
    44
    33
    => 1969
    => 1258
    => 881
    => 1770
    -105
    => -270
    => 20
    => 5
    200
    -3
    100002
    => -300000
    -7
    => 999993
    0
    => 0
    1698
    => 1001698
    36
    31
    8
    => 12
    0
    0
    5
    => 0
    3
    1
    10
    => 15
    => 70
    => 0
    123
    => 223
    -123
    => -23
}
module be_u_kinds;
%include 'exec.ins.pas';

type
  blk = record a: integer32; pad: array[1..60] of integer32; z: integer32; end;
  blks = array[1..4] of blk;
  blks_ptr = ^blks;
  blk_ptr = ^blk;
  holder = record n: integer; p: blk_ptr; end;
  node_ptr = ^node;
  node = record next: node_ptr; v: integer32; end;
  pint = 0..65535;
  sw = 0..32767;
  lint = 0..2147483647;
  crec = record c: pint; end;
  crec_ptr = ^crec;
  wset = set of 0..15;
  ub = [byte] 0..255;
  sb = [byte] -128..127;
  pair = record a, b: integer32; end;
  quad = record lo: pair; hi: pair; end;
  rawq = array[1..4] of integer32;
var
  bu_$blks: extern blks;
  bu_$bp: extern blks_ptr;
  saved: blk_ptr;
  h: holder;
  nodes: array[1..3] of node;
  head: node_ptr;
  ftab: array[-2..5] of node_ptr;
  ws: wset;

function bu_$addr_a(i: integer): integer32;
  options(val_param);
var
  p: blk_ptr;
begin
  p := addr(bu_$blks[i]);
  bu_$addr_a := p^.a + p^.z;
end;

function bu_$addr_thru(i: integer): integer32;
  options(val_param);
var
  p: blk_ptr;
begin
  p := addr(bu_$bp^[i]);
  bu_$addr_thru := p^.z;
end;

function bu_$addr_store(i: integer): integer32;
  options(val_param);
begin
  saved := addr(bu_$blks[i]);
  h.p := addr(bu_$blks[i + 1]);
  bu_$addr_store := saved^.z + h.p^.a - saved^.a;
end;

procedure log_z(p: blk_ptr);
  options(val_param);
begin
  exec_$logv_l(p^.z + p^.z);
end;

procedure bu_$addr_arg(i: integer);
  options(val_param);
begin
  log_z(addr(bu_$blks[i]));
end;

{ Comparing a pointer variable with a computed pointer: the TIME_Q_
  module-local 0xE16B70 at 0xE16BA6 `cur = addr(elem)' (elem a by-address
  parameter), DIR_$LOCK_OBJ `lock^.waiters = addr(handle)', and
  NETWORK_$REQUEST_SERVER `ecs[1] <> addr(...)'. }
function is_second(var n: node): integer;
begin
  is_second := 0;
  if head^.next = addr(n) then is_second := 1;
end;

function bu_$pcmp(k: integer): integer;
  options(val_param);
var
  p: node_ptr;
  loc: node;
  ptrs: array[1..2] of node_ptr;
  r: integer;
begin
  head := addr(nodes[1]);
  nodes[1].next := addr(nodes[2]);
  nodes[2].next := addr(nodes[3]);
  nodes[3].next := nil;
  r := is_second(nodes[k]);
  p := addr(loc);
  if p = addr(loc) then r := r + 10;
  ptrs[1] := addr(nodes[k]);
  if ptrs[1] <> addr(nodes[2]) then r := r + 100;
  bu_$pcmp := r;
end;

{ `addr' in arithmetic (codegen-patterns 5.2, "`addr(x)` as a value"):
  a value plus an address that needs a `lea', in NETBUF_$GETVA's order
  (0xE0ECC8..0xE0ECE2, the address first, then the masked value, `add.l
  A1,D3'), the address on the left, a value minus an address, an address
  minus a value, an address and a constant, `&', `!', `xor'.  The
  results are differences of addresses, so they do not depend on where
  the data lies. }
function bu_$addr_arith(i: integer; k: integer32): integer32;
  options(val_param);
var
  base, d1, d2: integer32;
begin
  base := integer32(addr(bu_$blks[1]));
  d1 := (k + integer32(addr(bu_$blks[i]))) - base;
  d2 := (integer32(addr(bu_$blks[i])) + k) - base;
  exec_$logv_l(d1 - d2 + 1000);
  d1 := (k - integer32(addr(bu_$blks[i]))) + base;
  d2 := integer32(addr(bu_$blks[i])) - k - base;
  exec_$logv_l(d1 + d2 + 501);
  bu_$addr_arith := d2 + 510;
end;

function bu_$addr_mask(i: integer; v: integer32): integer32;
  options(val_param);
var
  va: integer32;
begin
  va := (v & 16#3FF) + integer32(addr(bu_$blks[i]));
  bu_$addr_mask := va - integer32(addr(bu_$blks[1]));
end;

function bu_$addr_bits: integer32;
var
  loc: integer32;
  r: integer32;
begin
  loc := 0;
  r := (integer32(addr(loc)) & 1) + (integer32(addr(loc)) ! 1) - integer32(addr(loc));
  exec_$logv_l(r);
  r := integer32(addr(h.p)) + 4 - integer32(addr(h));
  exec_$logv_l(r);
  bu_$addr_bits := xor(integer32(addr(loc)), 1) - integer32(addr(loc)) + loc;
end;

{ A pointer function of one statement whose result is computed
  (codegen-patterns 3, "Pointer results"): an address formed straight
  into A0 (GET_WIRED 0xE1D8E8 `lea (A5),A0', `get_wired := addr(wired)'),
  of this unit's static, of an element of another unit's array, of a
  record a by-value pointer parameter points to; a by-value pointer
  parameter itself; a pointer computed by a type transfer (0xE4D5A6
  `move.l D0,(-12,A6); movea.l D0,A0'). }
function own_static: blk_ptr;
begin
  own_static := addr(saved);
end;

function elem_of(i: integer): blk_ptr;
  options(val_param);
begin
  elem_of := addr(bu_$blks[i]);
end;

function same_ptr(p: blk_ptr): blk_ptr;
  options(val_param);
begin
  same_ptr := p;
end;

function next_blk(p: blk_ptr): blk_ptr;
  options(val_param);
begin
  next_blk := blk_ptr(integer32(p) + sizeof(blk));
end;

function field_of(p: node_ptr): node_ptr;
  options(val_param);
begin
  field_of := addr(p^.next^);
end;

function bu_$presult(i: integer): integer32;
  options(val_param);
var
  p: blk_ptr;
  q: blk_ptr;
  n: node_ptr;
  r: integer32;
begin
  r := 0;
  q := own_static;
  if q = addr(saved) then r := 1;
  p := elem_of(i);
  r := r + p^.z;
  q := same_ptr(p);
  r := r + q^.a;
  p := next_blk(q);
  r := r + p^.a;
  nodes[1].next := addr(nodes[3]);
  nodes[3].v := 1000;
  n := field_of(addr(nodes[1]));
  r := r + n^.v;
  bu_$presult := r;
end;

{ A comparison with a function's result in the right operand of a plain
  `and'/`or' (codegen-patterns 7.1; ref "And", p. 4-18): an integer
  result against a 0..65535 field through a pointer (XNS_IDP_$OS_DEMUX
  0xE184FA; widened by the IR, so -1 and 65535 differ), a pointer
  function's result against `nil' (NET_IO_$CREATE_PORT's
  `route_$find_portp(...) = nil'), a result against a computed value. }
function ck(v: integer): integer;
  options(val_param);
begin
  ck := v + 1;
end;

function find(k: integer): node_ptr;
  options(val_param);
begin
  find := ftab[k];
end;

function bu_$callcmp(k: integer; c: integer32): integer;
  options(val_param);
var
  cr: crec;
  pp: crec_ptr;
  r: integer;
  j: integer;
begin
  j := -2;
  while j <= 5 do begin
    ftab[j] := nil;
    j := j + 1;
  end;
  ftab[3] := addr(nodes[1]);
  ftab[4] := addr(nodes[1]);
  cr.c := c;
  pp := addr(cr);
  r := 0;
  if (k > 0) and (ck(k) <> pp^.c) then r := r + 1;
  if (k > 5) or (find(k) = nil) then r := r + 10;
  if (k <> 7) and (ck(k) = k * 2 - 1) then r := r + 100;
  if (k < 0) and (ck(k) <> pp^.c) then r := r + 1000;
  bu_$callcmp := r;
end;

{ `in' on a one-word set variable with an element that is not constant
  (codegen-patterns 5.1): the element's range guarded as the AST
  module-local 0xE01C0A does (`moveq #15; cmp.w; bcs; move.w; btst;
  bhi'), so an element outside 0..15 is not a member (ref 3.9.3), though
  the register `btst' takes the bit mod 32.  A local set, this unit's
  static, one through a pointer, as a condition, under `not' and as a
  value. }
function bu_$inset(k: integer): integer;
  options(val_param);
var
  s: wset;
  p: ^wset;
  r: integer;
  b: boolean;
begin
  s := [1, 3, 15];
  ws := [0, 7];
  p := addr(ws);
  r := 0;
  if k in s then r := r + 1;
  if not (k in ws) then r := r + 10;
  b := k in s;
  if b then r := r + 100;
  if k in p^ then r := r + 1000;
  bu_$inset := r;
end;

{ An unsigned ordering comparison with 0 of a type that uses the top bit
  (0..65535): `x > 0' is `x <> 0', `x <= 0' is `x = 0', `x < 0' never
  and `x >= 0' always holds (`tst', then `bcs'/`bcc' on the clear C).  A
  variable and a computed value, 65535 among the arguments (a negative
  word).  Types whose values fit the signed range: a sign test, below
  (bu_$szero). }
function bu_$uzero(u: pint): integer;
  options(val_param);
var
  r: integer;
  m: pint;
begin
  r := 0;
  m := u;
  if m > 0 then r := r + 1;
  if m <= 0 then r := r + 2;
  if m < 0 then r := r + 4;
  if m >= 0 then r := r + 8;
  if (u & 6) > 0 then r := r + 16;
  if (u & 6) <= 0 then r := r + 32;
  if (u & 6) < 0 then r := r + 64;
  if (u & 6) >= 0 then r := r + 128;
  bu_$uzero := r;
end;

{ An ordering against 0 of an unsigned type whose values fit the signed
  range: a sign test, as BAT_$FREE 0xE3B5B0..0xE3B5B6 (`sub.l; bmi.s',
  `rel < 0' of a linteger) and PKT_$BLD_INTERNET_HDR 0xE122D6 (`tst.w;
  ble', `template_len > 0') make it.  A 0..32767 word, a 0..2147483647
  longword, a computed value. }
function bu_$szero(w: sw; l: lint): integer;
  options(val_param);
var
  r: integer;
  m: sw;
  n: lint;
begin
  r := 0;
  m := w;
  n := l;
  if m > 0 then r := r + 1;
  if m <= 0 then r := r + 2;
  if m < 0 then r := r + 4;
  if m >= 0 then r := r + 8;
  if n > 0 then r := r + 16;
  if n <= 0 then r := r + 32;
  if n < 0 then r := r + 64;
  if n >= 0 then r := r + 128;
  if (n - 1) < 0 then r := r + 256;
  bu_$szero := r;
end;

{ Assigning a string constant to a longer array of char (NAME_$INIT's
  `path_buf := '/com''): the text, then blanks to the end (ref
  "Variable-Length String Operations", p. 4-204), an odd size too; and a
  record or array selected from a type transfer of a variable
  (REM_FILE_$CREATE_TYPE's `rem_file_$loc_view_t(loc).addr'), read in
  place (ref p. 4-197). }
procedure bu_$strs;
var
  nm: array[1..9] of char;
  big: array[1..300] of char;
  q: quad;
  pr: pair;
  raw: rawq;
  i: integer;
begin
  nm := 'xxxxxxxxx';
  nm := 'abc';
  exec_$log_mem(nm, 9);
  big := '/com';
  exec_$log_mem(big, 6);
  exec_$log_c(big[300]);
  i := 1;
  while i <= 4 do begin
    raw[i] := i * 11;
    i := i + 1;
  end;
  pr := quad(raw).hi;
  exec_$logv_l(pr.a);
  exec_$logv_l(pr.b);
  q.lo := quad(raw).lo;
  exec_$logv_l(q.lo.a + q.lo.b);
end;

{ Comparing boolean values (SMD_$SHOW_CURSOR 0xE6E32E `cmp.b', a
  variable with a field; SIO2681_$SET_LINE 0xE1D30C `cmp.b D5,D1', two
  comparisons): as bytes, 0 and 0xFF; the orderings by `false < true',
  the order of the type's values; against the constants `true' and
  `false'. }
function bu_$bools(k: integer): integer;
  options(val_param);
var
  a, b: boolean;
  r: integer;
begin
  a := k > 0;
  b := k > 5;
  r := 0;
  if a = b then r := r + 1;
  if a <> b then r := r + 2;
  if a < b then r := r + 4;
  if a > b then r := r + 8;
  if a <= b then r := r + 16;
  if a >= b then r := r + 32;
  if a = true then r := r + 64;
  if b = false then r := r + 128;
  if (k > 0) = (k > 5) then r := r + 256;
  if (k < 3) <> b then r := r + 512;
  if b < true then r := r + 1024;
  bu_$bools := r;
end;

{ An M$ARITH call with both operands computed, one calling (WIN's
  `heads * cyl * blocks_per_track', codegen-patterns 8): the statement's
  calls with their results saved out of D0. }
function bu_$mul3(a, b, c: integer32): integer32;
  options(val_param);
var
  t: integer32;
begin
  t := a * b * (c + 1);
  exec_$logv_l(t);
  bu_$mul3 := (a * b) * (c * a);
end;

{ An empty `otherwise' part (FIM's module-local build_df): nothing runs
  when no label matches (ref "Otherwise", p. 4-36). }
function bu_$case_empty(k: integer): integer;
  options(val_param);
var
  r: integer;
begin
  r := 5;
  case k of
    1: r := 10;
    2: r := 20;
    otherwise
  end;
  bu_$case_empty := r;
end;

{ Widening a byte to a longword (SUMA_$RCV's `rshft(data_byte, 2) &
  16#F'): an unsigned byte zero-extended, a signed one sign-extended,
  from memory and computed. }
function bu_$widen(x: ub; y: sb): integer32;
  options(val_param);
var
  l: integer32;
  bx: ub;
  by: sb;
begin
  bx := x;
  by := y;
  l := bx;
  exec_$logv_l(l);
  l := by;
  exec_$logv_l(l);
  l := (rshft(bx, 2) & 16#F) + 100000;
  exec_$logv_l(l);
  bu_$widen := by * 100000;
end;

{ A signed `mod' by a word used as a longword (TIME_$ADJUST_TIME_OF_DAY's
  `delta_ticks mod skew <> 0'): `M$OIS$WLW''s word remainder
  sign-extended; `mod' takes the dividend's sign (ref p. 4-115). }
function bu_$lmod(x: integer32; w: integer): integer32;
  options(val_param);
var
  l: integer32;
begin
  l := x mod w;
  exec_$logv_l(l);
  if x mod w <> 0 then l := l + 1000000;
  bu_$lmod := l;
end;

{ `for' loops over a word index with a run-time lower bound (KBD's
  module-local 0xE1CA10) and `downto' with run-time bounds, in the
  general form (`to - from', `blt', a `dbf' count; codegen-patterns 7.4,
  "Byte and longword indices"): an array read by the index, a body that
  calls, empty ranges (ref p. 4-90: no pass). }
function bu_$forrt(a, b: integer): integer32;
  options(val_param);
var
  i: integer;
  s: integer32;
  arr: array[1..10] of integer;
begin
  for i := 1 to 10 do arr[i] := i * 3;
  s := 0;
  for i := a to b do s := s + arr[i];
  exec_$logv_l(s);
  s := 0;
  for i := b downto a do s := s * 2 + i;
  exec_$logv_l(s);
  s := 0;
  for i := 10 downto a do s := s + 1;
  exec_$logv_l(s);
  s := 0;
  for i := a to 5 do begin
    exec_$clobber;
    s := s + i;
  end;
  bu_$forrt := s;
end;

{ An unsigned word index from a constant to a run-time bound, read only
  as an array's index (the general form, [`for_wide`]: the index's
  register set up, not its frame slot read). }
function bu_$foru(n: pint): integer32;
  options(val_param);
var
  u: pint;
  s: integer32;
  arr: array[0..9] of integer;
begin
  for u := 0 to 9 do arr[u] := u * 7;
  s := 0;
  for u := 1 to n do s := s + arr[u];
  bu_$foru := s;
end;

{ A longword `div`/`mod` used as a byte: the longword result's low
  byte, as an assignment narrows (`mod` takes the dividend's sign, ref
  p. 4-115). }
function bu_$lbyte(l: integer32): integer;
  options(val_param);
var
  b: sb;
  c: char;
begin
  b := l mod 1000;
  exec_$logv_l(b);
  if l > 0 then c := chr(l div 10000) else c := chr(-(l div 10000));
  bu_$lbyte := b + ord(c);
end;
