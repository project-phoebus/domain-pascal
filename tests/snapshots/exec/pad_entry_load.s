.text
.globl PAD_ENTRY_LOAD_$TWO
PAD_ENTRY_LOAD_$TWO:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d2
subq.l #2,%sp
move.w (0xa,%fp),%d3
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
move.w %d2,%d0
add.w %d0,%d0
move.w %d0,%d1
lsl.w #2,%d1
add.w %d1,%d0
add.w %d3,%d0
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl PAD_ENTRY_LOAD_$IO_LIKE
PAD_ENTRY_LOAD_$IO_LIKE:
link.w %fp,#0
move.l %d2,-(%sp)
subq.l #2,%sp
move.w (8,%fp),%d2
move.w (0xa,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d2,%d0
add.w %d0,%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
move.l (-4,%fp),%d2
unlk %fp
rts
.globl PAD_ENTRY_LOAD_$SLOTS
PAD_ENTRY_LOAD_$SLOTS:
link.w %fp,#0
movem.l %a2-%a3,-(%sp)
movea.l (8,%fp),%a2
subq.l #2,%sp
movea.l (0xc,%fp),%a3
move.w #2,-(%sp)
jsr (EXEC_$MARK).l
move.w (%a3),%d0
add.w (%a2),%d0
move.w %d0,(%a3)
move.w (%a2),%d0
add.w (%a3),%d0
move.w %d0,(%a2)
movem.l (-8,%fp),%a2-%a3
unlk %fp
rts
.globl PAD_ENTRY_LOAD_$READS
PAD_ENTRY_LOAD_$READS:
link.w %fp,#0
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d2,%d0
add.w %d3,%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-8,%fp),%d2-%d3
unlk %fp
rts
INNER:
link.w %fp,#-4
pea (%a2)
subq.l #2,%sp
movea.l (%fp),%a2
move.w #3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #1,(-2,%a2)
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl PAD_ENTRY_LOAD_$OUTER
PAD_ENTRY_LOAD_$OUTER:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,(-2,%fp)
bsr.s INNER
bsr.s INNER
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
unlk %fp
rts
.balignw 4,0x2048
