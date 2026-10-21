.text
.globl NW_$DEMUX
NW_$DEMUX:
link.w %fp,#-0x18
movea.l (8,%fp),%a0
move.w (0x1a,%a0),(-4,%fp)
subq.l #2,%sp
move.w (-4,%fp),-(%sp)
jsr (Q).l
unlk %fp
rts
.globl NW_$LOG
NW_$LOG:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.b (1,%a0),(2,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl NW_$UNLOCK
NW_$UNLOCK:
link.w %fp,#-4
jsr (SPIN_LOCK).l
clr.l %d1
move.w %d0,%d1
move.l %d1,(-4,%fp)
jsr (WORK).l
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (Q).l
unlk %fp
rts
.globl NW_$HINT
NW_$HINT:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.l (8,%fp),%d2
moveq #0x1f,%d0
and.w %d2,%d0
move.w %d0,-(%sp)
move.l %d2,%d0
lsr.l #5,%d0
move.w %d0,-(%sp)
jsr (Q2).l
addq.w #4,%sp
moveq #0x3f,%d3
and.w %d2,%d3
move.w %d3,(-2,%fp)
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (Q).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (Q).l
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl NW_$INIT
NW_$INIT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (REAL_PAGES).l,%d0
lsr.l #8,%d0
lsr.l #2,%d0
lsl.l #4,%d0
move.w %d0,(%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl NW_$SEND
NW_$SEND:
link.w %fp,#0
movea.l (8,%fp),%a0
move.w (%a0),%d0
movea.l (0xc,%fp),%a0
ext.l %d0
movea.l (%a0),%a0
add.l (%a0),%d0
movea.l (8,%fp),%a0
move.w %d0,(%a0)
unlk %fp
rts
.globl NW_$WIDEN
NW_$WIDEN:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
subq.l #2,%sp
clr.l %d0
move.w (0x1c,%a2),%d0
move.l %d0,-(%sp)
move.w #1,-(%sp)
jsr (QL).l
addq.w #8,%sp
clr.w %d0
move.b (0x1b,%a2),%d0
move.w %d0,-(%sp)
clr.w %d0
move.b (0x1a,%a2),%d0
move.w %d0,-(%sp)
jsr (Q2).l
movea.l (-4,%fp),%a2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.NW","aw",@nobits
.balign 4
.Ldata:
.space 4
