{ Golden: codegen-patterns 1.5, "A longword held in an address register"
  (bead domain-pascal-6wsg): a pointer passed by value (`val_param')
  from an address register is pushed with `pea (An)', never `move.l
  An,-(SP)' (no such argument push in compiled code; 99 by-value pointer
  pushes `pea (An)').  SIO_$I_XMIT_DONE 0xE1C6B4: its parameter held in
  A2, passed on to SIO_$I_TSTART at 0xE1C6C4 `pea (A2)'.  A pointer in
  memory is pushed with `move.l': a frame slot, `move.l (-8,A6),-(SP)'
  (DBUF_$SET_BUFF's callers 0xE38A0A, 0xE3916A, 0xE39E08), the unit's
  data, `move.l (3384,A5),-(SP)' (0xE3B2B8, 0xE3B608).  A pointer local
  just stored from A0 and pushed by the next statement is pushed from
  A0, `pea (A0)', by the register-contents cache (docs/register-model.md,
  F1; `crate::cache'; no site of this exact shape). }
module sio;
type
  desc_t = record
    pad: array[1..58] of integer;
    state: set of 0..15;
  end;
  desc_ptr_t = ^desc_t;

var
  cur: desc_ptr_t;

procedure sio_$i_tstart(desc: desc_ptr_t);
  options(extern, val_param);
function sio_$i_next: desc_ptr_t;
  options(extern);

function sio_$i_xmit_done(desc: desc_ptr_t): boolean;
  options(val_param);
begin
  desc^.state := desc^.state - [0];
  sio_$i_tstart(desc);
  sio_$i_xmit_done := 0 in desc^.state;
end;

procedure sio_$start_cur;
begin
  sio_$i_tstart(cur);
end;

procedure sio_$start_next;
var
  d: desc_ptr_t;
begin
  d := sio_$i_next;
  sio_$i_tstart(d);
  sio_$i_tstart(d);
end;
