{ Golden: codegen-patterns 7.4, `while' loops: a `bra' to the test at the
  bottom, the body, the test branching back to the head while the
  condition holds (0xE01A8E..0xE01A98; the DISK_ module-local 0xE3CCCE).
  The head after the `bra' is a loop head like any other: in an aligning
  routine it gets the 0x2048 filler when it would start at 2 mod 4
  (MAC_OS_$CLOSE 0xE0B4CC..0xE0B4D0; survey: 29 heads with the filler, 27
  4-aligned, 2 at 2 mod 4, both of loops over 0x100 bytes).  An `and
  then' test leaves the loop when its first operand fails and branches
  back when the second holds (NAME_$VALIDATE's module-local
  0xE4A028..0xE4A034).  A walk pointer held in an address register is
  tested with `cmpa.w #0,An' (0xE3CCE4).  `exit' branches past the test,
  `next' to it, in `repeat' loops too; `if c then exit' is one
  conditional branch (no conditional branch over a `bra' in compiled
  code, codegen-patterns 7.1). }
module whl;
type
  node_ptr = ^node;
  node = record
    next: node_ptr;
    val: integer32;
  end;
var
  w: extern integer;
  n: extern integer;
  b: extern boolean;
  buf: array[1..64] of char;
  head: extern node_ptr;

procedure whl_$count; internal;
begin
  w := 0;
  while w < 10 do
    w := w + 1;
end;

procedure whl_$scan; internal;
begin
  n := 1;
  while (n <= 64) and then (buf[n] = '/') do
    n := n + 1;
end;

procedure whl_$walk(p: node_ptr; var total: integer32); internal;
var
  q: node_ptr;
begin
  q := p;
  while q <> nil do begin
    total := q^.val;
    q := q^.next;
  end;
end;

procedure whl_$exits; internal;
begin
  while w <> 0 do begin
    if b then exit;
    if n = 3 then next;
    w := w - 1;
  end;
  n := 0;
end;

procedure whl_$repeat; internal;
begin
  repeat
    if b then exit;
    if n = 3 then next;
    w := w - 1;
  until w = 0;
  n := 0;
end;
