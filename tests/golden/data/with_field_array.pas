{ Golden: `with' (ref "With", pp. 4-211..4-214) over an element, at a
  run-time index, of an array that is a field at +0x3C of another unit's
  record; codegen-patterns 5.2, "`with`" (bead domain-pascal-3m13).

  The element's address is formed with no displacement, the array's
  offset folded into every field's displacement.  SAU5 MT_PURGE_UCB
  0xE75770..0xE757CA (`with mt_$scb.units[u]', the 0x2E-byte units at
  +0x3C of MT_$SCB 0xE90C8E):
    link.w A6,-0x8
    movem.l A2/A3,-(SP)
    movea.l (0x8,A6),A2
    moveq #0x2e,D0
    movea.l #MT_$SCB,A0
    mulu.w (A2),D0
    lea (0x0,A0,D0w*0x1),A3
    bset.b #0x2,(0x44,A3)
    bra.b test
    movea.l A0,A0
  loop:
    pea (true,PC)
    pea (0x3c,A3)
    jsr MT_$DEQ
    addq.w #4,SP
    move.l A0,(-0x8,A6)
    pea (-0x8,A6)
    pea (A2)
    bsr.w MT_$PURGE_PB        (an extern here: `jsr')
    lea (0xc,SP),SP
  test:
    tst.l (0x3c,A3)
    bne.b loop
    bclr.b #0x2,(0x44,A3)
    movem.l (-0x10,A6),A2/A3
    unlk A6
    rts
  The same shape at 0xE75870, 0xE75B36, 0xE75C22, 0xE762BC, 0xE767C4 and
  0xE769A0 (`lea (0,An,D0.w),Am', the units' fields at (0x44,Am), (0x48,Am),
  (0x52,Am), (0x58,Am)).  The repro is ../source's
  tmp/sau5/mtrepro/withelem.pas. }
module withelem;

type
  pb_ptr = ^integer32;
  q_t = record
    head: pb_ptr;                    { 0x00 }
    tail: pb_ptr;                    { 0x04 }
  end;
  unit_t = record
    queue: q_t;                      { 0x00 }
    flags: set of 0..15;             { 0x08 }
    pad: array[1..18] of integer;    { 0x0A }
  end;                               { 0x2E bytes }
  scb_t = record
    head: array[1..30] of integer;   { 0x00 }
    units: array[0..1] of unit_t;    { 0x3C }
  end;

var
  mt_$scb: extern scb_t;

function mt_$deq(var q: q_t): pb_ptr; extern;
procedure mt_$purge_pb(in u: integer; in pb: pb_ptr; in b: boolean); extern;

procedure mt_purge_ucb(in u: integer);
begin
  with mt_$scb.units[u] do begin
    flags := flags + [10];
    while queue.head <> nil do
      mt_$purge_pb(u, mt_$deq(queue), true);
    flags := flags - [10];
  end;
end;
