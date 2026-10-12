{ Golden: codegen-patterns 12.2 (a body of several statements loads the
  by-value parameters it reads at entry, in parameter order: data values
  into D0, D1, the dereferenced pointer into A0), 13 (clr.l for a zero
  store, (An) for offset 0), 1.3 (a boolean by value in the high byte of
  its word slot) and 2 (link.w #0, no save).  Mirrors TIME_$Q_INIT_QUEUE
  0xE16C5E..0xE16C7E. }
module time_q;
type
  queue_t = record
    head: integer32;
    lock: integer32;
    is_vt: boolean;
    queue_id: integer;
  end;
  queue_ptr = ^queue_t;
procedure q_init(is_vt: boolean; queue_id: integer; queue: queue_ptr);
  options(val_param);
begin
  queue^.head := 0;
  queue^.lock := 0;
  queue^.is_vt := is_vt;
  queue^.queue_id := queue_id;
end;
