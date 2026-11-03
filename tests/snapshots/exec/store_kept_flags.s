.text
.globl STORE_KEPT_FLAGS_$ENTRY
STORE_KEPT_FLAGS_$ENTRY:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
seq %d2
move.w %d0,%d3
lsl.w #2,%d3
add.w (%a5),%d3
move.b %d2,(7,%a5,%d3.w)
move.w %d0,%d2
add.w %d1,%d2
move.w %d2,(6,%a5)
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl STORE_KEPT_FLAGS_$SUBQ
STORE_KEPT_FLAGS_$SUBQ:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d0
add.w (%a5),%d0
st (7,%a5,%d0.w)
subq.w #1,(2,%a5)
smi %d0
move.w %d1,%d2
add.w (%a5),%d2
move.b %d0,(7,%a5,%d2.w)
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl STORE_KEPT_FLAGS_$LOC
STORE_KEPT_FLAGS_$LOC:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,%d2
lsl.w #2,%d2
move.w %d2,%d3
add.w (%a5),%d3
clr.b (7,%a5,%d3.w)
move.w %d0,%d3
add.w %d1,%d3
move.w %d3,(-2,%fp)
seq %d0
move.w %d2,%d1
add.w (%a5),%d1
move.b %d0,(7,%a5,%d1.w)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl STORE_KEPT_FLAGS_$WEB
STORE_KEPT_FLAGS_$WEB:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,%d2
lsl.w #2,%d2
move.w %d2,%d3
add.w (%a5),%d3
clr.b (7,%a5,%d3.w)
move.w %d0,%d3
add.w %d1,%d3
seq %d0
move.w %d2,%d1
add.w (%a5),%d1
move.b %d0,(7,%a5,%d1.w)
move.w %d3,(6,%a5)
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl STORE_KEPT_FLAGS_$ASG
STORE_KEPT_FLAGS_$ASG:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d0
add.w (%a5),%d0
tst.w (4,%a5)
sne %d2
move.b %d2,(7,%a5,%d0.w)
move.w (4,%a5),%d0
seq %d2
move.w %d1,%d3
add.w (%a5),%d3
move.b %d2,(7,%a5,%d3.w)
move.w %d0,(6,%a5)
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl STORE_KEPT_FLAGS_$SETK
STORE_KEPT_FLAGS_$SETK:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),(4,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl STORE_KEPT_FLAGS_$SHOW
STORE_KEPT_FLAGS_$SHOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
tst.b (7,%a5,%d0.w)
bpl.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl STORE_KEPT_FLAGS_$SHOWS
STORE_KEPT_FLAGS_$SHOWS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.STORE_KEPT_FLAGS","aw",@progbits
.balign 4
.Ldata:
.byte 0x00,0x02,0x00,0x01
.space 104
