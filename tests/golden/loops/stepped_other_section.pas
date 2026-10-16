{ Golden: codegen-patterns 7.4 ("Arrays of another data section's block";
  bead domain-pascal-m2m2), `for' loops stepping a pointer through an
  array of the unit's block in another data section (a routine in
  OS_INIT_PROC over its module's WIRED_DATA).  As for another unit's
  object: `movea.l #block,Ap', the block's start as the immediate, adjusted
  in place by from * size (`addq.l', `lea (k,Ap),Ap' above 8), the
  `movea.l' before the last count or index instruction; from 0 it is the
  pointer itself, after the count and the index.  The array's offset in
  the block goes into the displacements:
  - HINT_$INIT_CACHE 0xE313DA `movea.l #0xe7db50,A0; moveq #1,D0; lea
    (0xc,A0),A0', the cache at the block's start, `(-0x4,A0)' ..
    `(-0xc,A0)';
  - TIME_$INIT 0xE2FE94 `moveq #0x3f,D3; movea.l #0xe29198,A2; moveq
    #1,D4; lea (0xc,A2),A2', the queues at 0x1308, `pea (0x12fc,A2)';
  - IO_$INIT 0xE32924 `moveq #0x4,D2; movea.l #0xe2c368,A3', the array at
    0x60 from 0, `(0x60,A3)'; EC2_$INIT_S 0xE3099E. }
module hint, wired_proc, wired_data;
type
  entry_t = record
    timestamp: integer32;
    result: boolean;
    pad1: char;
    pad2: integer;
    uid_low: integer32;
  end;
  q_t = record
    head: integer32;
    tail: integer32;
    n: integer32;
  end;
  bus_t = record
    init: integer32;
    pad: array[1..8] of integer32;
  end;
var
  cache: array[1..2] of entry_t;       { +0 }
  cache_index: integer;                { +0x18 }
  pad: array[1..17] of integer32;
  bus: array[0..4] of bus_t;           { +0x60 }
  more: array[1..1000] of integer32;
  vtq: array[1..64] of q_t;

procedure q_$init_queue(flag: boolean; n: integer; var q: q_t);
  options(val_param, extern);

{ HINT_$INIT_CACHE 0xE313DA..0xE313F8. }
[section(os_init_proc, os_init_data)]
procedure hint_$init_cache;
var
  i: integer;
begin
  for i := 1 to 2 do begin
    cache[i].uid_low := 0;
    cache[i].result := false;
    cache[i].timestamp := 0;
  end;
end;

{ TIME_$INIT 0xE2FE94..0xE2FEBA. }
[section(os_init_proc, os_init_data)]
procedure hint_$init_q;
var
  i: integer;
begin
  for i := 1 to 64 do
    q_$init_queue(true, i, vtq[i]);
end;

{ IO_$INIT 0xE32924..0xE3293A (a store for its call). }
[section(os_init_proc, os_init_data)]
procedure hint_$init_bus;
var
  i: integer;
begin
  for i := 0 to 4 do
    bus[i].init := 0;
end;
