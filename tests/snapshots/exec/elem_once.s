.text
.globl EB_$FILL_ONCE
EB_$FILL_ONCE:
link.w %fp,#0
jsr (EO_$FILL).l
unlk %fp
rts
.globl EO_$GET
EO_$GET:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #EO_$BIG,%a0
move.w %d0,%d1
lsl.w #5,%d1
move.w %d1,%d0
lsl.w #3,%d0
add.w %d0,%d1
lea (0,%a0,%d1.w),%a1
move.l (-0x11c,%a1),%d0
unlk %fp
rts
.globl EO_$PUT
EO_$PUT:
link.w %fp,#0
move.w (8,%fp),%d0
movea.l #EO_$BIG,%a0
move.w %d0,%d1
lsl.w #5,%d1
move.w %d1,%d0
lsl.w #3,%d0
add.w %d0,%d1
lea (0,%a0,%d1.w),%a1
move.l #0x309,(-0x11c,%a1)
unlk %fp
rts
.globl EO_$ADDR
EO_$ADDR:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
movea.l #EO_$RECS,%a0
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d0
add.w %d0,%d0
add.w %d0,%d1
lea (-0xc,%a0,%d1.w),%a1
move.l %a1,(%a5)
movea.l (%a5),%a0
move.l (4,%a0),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl EO_$TWO
EO_$TWO:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d1
move.w (0xa,%fp),%d0
move.w %d1,%d2
movea.l #EO_$RECS,%a0
lsl.w #2,%d2
move.w %d2,%d1
add.w %d1,%d1
add.w %d1,%d2
lea (-0xc,%a0,%d2.w),%a1
move.l %a1,(%a5)
move.w %d0,%d1
movea.l #EO_$QUITS,%a0
lsl.w #2,%d1
move.w %d1,%d0
add.w %d0,%d0
add.w %d0,%d1
lea (-0xc,%a0,%d1.w),%a0
move.l %a0,(4,%a5)
movea.l (%a5),%a0
movea.l (4,%a5),%a1
move.l (%a0),%d0
add.l (8,%a1),%d0
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl EO_$AFTER
EO_$AFTER:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
jsr (EO_$FILL).l
move.w %d2,%d0
movea.l #EO_$RECS,%a0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
lea (-0xc,%a0,%d0.w),%a1
move.l %a1,(%a5)
movea.l (%a5),%a0
move.l (8,%a0),%d0
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ELEM_ONCE","aw",@nobits
.balign 4
.Ldata:
.space 8
