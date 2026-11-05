.text
UPD:
link.w %fp,#-4
pea (%a2)
movea.l %a1,%a2
move.l (0xc,%fp),-(%sp)
move.l (8,%fp),-(%sp)
move.l (8,%a2),-(%sp)
jsr (AST_$INVALIDATE_PAGE).l
lea (0xc,%sp),%sp
movea.l (8,%a2),%a0
move.w (4,%a0),-(%sp)
move.w #5,-(%sp)
jsr (NETLOG_$LOG_IT).l
addq.w #4,%sp
move.l (0xc,%fp),-(%sp)
jsr (MMAP_$AVAIL).l
movea.l (-8,%fp),%a2
unlk %fp
rts
BATCH:
link.w %fp,#-4
pea (%a2)
movea.l (%fp),%a2
subq.l #2,%sp
move.w (-2,%a2),-(%sp)
jsr (ML_$LOCK).l
addq.w #4,%sp
subq.l #2,%sp
move.w (-2,%a2),-(%sp)
pea (5).w
move.l (0xc,%a2),-(%sp)
movea.l %a2,%a1
bsr.s UPD
clr.w (-2,%a2)
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl PMAP_$FLUSH
PMAP_$FLUSH:
link.w %fp,#-4
movem.l %d2/%a2,-(%sp)
move.w (0x12,%fp),%d0
move.w (0x14,%fp),%d1
movea.l (0x16,%fp),%a2
move.w %d0,(-2,%fp)
clr.l (%a2)
move.w %d1,%d2
tst.w %d0
ble.s 1f
bsr.s BATCH
1:
subq.l #2,%sp
move.w (0x10,%fp),-(%sp)
pea (7).w
move.l (0xc,%fp),-(%sp)
movea.l %fp,%a1
bsr.w UPD
move.w %d2,%d0
ext.l %d0
move.l %d0,(%a2)
movem.l (-0xc,%fp),%d2/%a2
unlk %fp
rts
.balignw 4,0x2048
