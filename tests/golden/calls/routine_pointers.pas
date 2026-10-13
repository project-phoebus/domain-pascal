{ Golden: codegen-patterns 1.6, calls through routine pointers (ref
  3.13.3 p. 3-50; "Invoking Procedure and Function Pointers", p. 4-145;
  options after the type, p. 4-146 and Appendix B messages 181, 182).
  The arguments are pushed as for any call, with the conventions and the
  pad of the pointer type (`val_param' on the type: values in word
  slots, booleans with `st'/`clr.w'), then the pointer is read into a
  new address register right before `jsr (An)': from memory (KBD_$RCV
  0xE1CE5C `movea.l (A2),A0'; SIO_$I_CTS_CHANGE 0xE1C71C..0xE1C728
  `subq.l #2,SP; clr.w -(SP); move.l (4,A2),-(SP); movea.l (0x38,A2),A0;
  jsr (A0); addq.w #8,SP'), or copied from a data register (IO_$INIT
  0xE329CC `move.l A0,D3; pea (A3); movea.l D3,A0; jsr (A0)').  A
  function pointer's data result comes back in D0 (TTY_$I_ERR 0xE1BE22).
  A local passed to a `var' parameter of the pointer type is read from
  its slot after the call (bead domain-pascal-5aw.16).  `addr' of a
  routine is its symbol as an immediate (MAC_OS 0xE0B9A8 `move.l
  #MAC_$DEMUX,(-8,A6)'; PROC1_ 0xE14B7A). }
module rp;
type
  xon_t = ^procedure (line: integer32; stop: boolean); options(val_param);
  status_t = ^function (line: integer32; clear: boolean): integer32; val_param;
  plain_t = ^procedure (var n: integer32; in w: integer);
  desc_t = record
    line: integer32;          { +0 }
    pad: array[1..13] of integer32;
    xon: xon_t;               { +0x38 }
    status: status_t;         { +0x3c }
  end;
var
  q: plain_t;
  last_status: extern integer32;

procedure advance(var l: integer32); extern;
function get_xon: xon_t; extern;

{ SIO_$I_CTS_CHANGE's shape: a field of a descriptor held in A2 }
procedure kick(in out d: desc_t);
begin
  d.line := 0;
  if d.xon <> nil then d.xon^(d.line, false);
  advance(d.line);
end;

{ a function pointer's result in D0 }
procedure err(in out d: desc_t);
begin
  last_status := d.status^(d.line, true);
  advance(d.line);
end;

{ the default convention: by address, the local re-read after the call }
procedure plain;
var
  n: integer32;
begin
  n := 5;
  q^(n, 7);
  n := n + 1;
  q^(n, 7);
end;

{ a pointer local held in a data register, copied to an address register
  after the pushes }
procedure held(line: integer32); val_param;
var
  f: xon_t;
begin
  f := get_xon;
  f^(line, true);
end;

procedure handler(var n: integer32; in w: integer);
begin
end;

procedure install;
begin
  q := addr(handler);
end;
