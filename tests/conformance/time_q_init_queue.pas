{ TIME_$Q_INIT_QUEUE, 0xE16C5E, 0x22 bytes, and TIME_$Q_FLUSH_QUEUE,
  0xE16C80, 0xE bytes: 0xE16C5E..0xE16C8E (tools/imgdis.sh E16C5E 30).
  Semantics: ../source/domain_os/time/q_init_queue.c, q_flush_queue.c;
  layout: time_queue_t in ../source/domain_os/time/time.h (its
  _Static_asserts: head 0, lock 4, flags 8, pad 9, queue_id 0xA, size
  0xC).

  The unit is a fragment of the TIME_Q_ module (segment 0xE16AE8) that
  starts at 2 mod 4, so tools/conform.py compiles it with
  `dpc --fragment-origin 0xE16C5E' (codegen-patterns 7, 11).

  Patterns exercised (docs/codegen-patterns.md):
  - 1.2/1.3 val_param: a boolean by value in the high byte of a word slot
         at (8,A6), a word at (0xa,A6), a pointer at (0xc,A6);
  - 2    link.w A6,#0 with no locals, no register save, unlk; rts;
  - 12.2 a body of several statements loads the by-value parameters it
         reads at entry, in parameter order (D0, D1, A0), before the first
         statement; the statements use the registers;
  - 13   stores through a pointer: clr.l for zero, move.b / move.w from
         registers, `(An)' for offset 0;
  - Q_FLUSH_QUEUE: a one-statement body evaluates in its own order
         (movea.l, then clr.l).

  Reconstructed (not known from the original): the type and field names;
  `head' as integer32 (the C has a uint32_t "32-bit VA, 0 = empty"; a
  pointer with `nil' would need nil's representation, which the reference
  does not give); `queue_id' as integer (the C has uint16_t; only its
  width matters to these stores); `queue' as a by-value pointer (a `var'
  record parameter is reached through its slot instead, which the image
  cannot tell apart for a one-statement body); val_param on both
  routines.  The pad byte at 9 is the default word alignment of
  `queue_id' (ref 3.10.5). }
module time_q;
type
  time_$queue_t = record
    head: integer32;          { 0x0 }
    lock: integer32;          { 0x4: ML_$SPIN_LOCK cell }
    is_vt: boolean;           { 0x8: true = a virtual-timer queue }
    queue_id: integer;        { 0xA }
  end;
  time_$queue_ptr_t = ^time_$queue_t;

procedure time_$q_init_queue(is_vt: boolean; queue_id: integer;
                             queue: time_$queue_ptr_t); options(val_param);
begin
  queue^.head := 0;
  queue^.lock := 0;
  queue^.is_vt := is_vt;
  queue^.queue_id := queue_id;
end;

procedure time_$q_flush_queue(queue: time_$queue_ptr_t); options(val_param);
begin
  queue^.head := 0;
end;
