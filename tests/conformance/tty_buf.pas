{ tty_$i_buf_insert and tty_$i_buf_put, module-local at 0xE1AF0A (0x38
  bytes) and 0xE1AF42 (0x60 bytes; tools/imgdis.sh E1AF0A 98), the
  routines after tty_$i_advance_ec in TTY's WIRED_PROC segment (bead
  domain-pascal-iyd).  Reconstructed from ../source's Domain Pascal
  translation (domain_os/tty/i_buf_insert.pas, i_buf_put.pas,
  tty.ins.pas and ml/ml.ins.pas at ../source 3898a2ce, by Claude Opus
  5.5), reduced to what the unit needs.  Semantics: ../source
  domain_os/tty/i_buf_insert.c, i_buf_put.c.

  Patterns exercised (docs/codegen-patterns.md):
  - 12.5  a local held in a register though assigned in both arms of an
          `if', one of them a constant: `new_tail' in D1 (0xE1AF20
          `moveq #1,D1', 0xE1AF24 `move.w (2,A0),D1; addq.w #1,D1'), and
          in D0 in tty_$i_buf_put (0xE1AF6C, 0xE1AF70); its frame word
          stays allocated (`link #-8': the slot of a routine that is not
          global, then -6 and -8);
  - 12.5  a local read only as a pushed argument stays in its frame
          slot: the spin-lock token, stored at 0xE1AF60 `move.w
          D0,(-6,A6)' and pushed from the slot at 0xE1AF88, not from D0,
          which `new_tail' took in between;
  - 12.2  parameters loaded at entry: the character in D0, D2 across
          the call; the buffer in A0, A2;
  - 12.4  the index of the element store in the next free register, D2
          (saved) in tty_$i_buf_insert, D1 in tty_$i_buf_put.

  Reconstructed (not known from the original): every name (the C
  tree's and ../source's); the buffer as a `var' record, the positions
  as 1..256; the spin lock headings as in pkt_next_id.pas. }
module tty, wired_proc, wired_data;
const
  tty_$buf_size = 16#100;
type
  tty_$buf_pos_t = 1..tty_$buf_size;
  tty_$cbuf_t = record
    head: tty_$buf_pos_t;
    tail: tty_$buf_pos_t;
    size: integer;
    data: array[1..tty_$buf_size] of char;
  end;

var
  tty_$spin_lock: extern integer32;

function ml_$spin_lock(var lock: integer32): integer; extern;
procedure ml_$spin_unlock(var lock: integer32; token: integer);
  options(extern, val_param);

procedure tty_$i_buf_insert(ch: char; var buf: tty_$cbuf_t); internal;
var
  new_tail: tty_$buf_pos_t;
begin
  if buf.tail = tty_$buf_size then
    new_tail := 1
  else
    new_tail := buf.tail + 1;
  if new_tail <> buf.head then begin
    buf.data[buf.tail] := ch;
    buf.tail := new_tail;
  end;
end;

procedure tty_$i_buf_put(ch: char; var buf: tty_$cbuf_t); internal;
var
  token: integer;
  new_tail: tty_$buf_pos_t;
begin
  token := ml_$spin_lock(tty_$spin_lock);
  if buf.tail = tty_$buf_size then
    new_tail := 1
  else
    new_tail := buf.tail + 1;
  if new_tail <> buf.head then begin
    buf.data[buf.tail] := ch;
    buf.tail := new_tail;
  end;
  ml_$spin_unlock(tty_$spin_lock, token);
end;
