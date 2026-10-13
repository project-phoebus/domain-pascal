.text
.globl BUSY_WAIT
BUSY_WAIT:
link.w %fp,#0
movea.l (0xc,%fp),%a0
clr.l (%a0)
unlk %fp
rts
.globl ALLOC_CONTIG
ALLOC_CONTIG:
link.w %fp,#0
movea.l (0xa,%fp),%a0
clr.l (%a0)
movea.l (0xe,%fp),%a1
move.l #0x6000e,(%a1)
unlk %fp
rts
.globl RECEIVE
RECEIVE:
link.w %fp,#0
pea (%a2)
movea.l (0x1c,%fp),%a0
move.l #0x2c000a,(%a0)
movea.l (0x18,%fp),%a1
clr.w (%a1)
movea.l (0x10,%fp),%a2
clr.w (%a2)
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl GET_STATS
GET_STATS:
link.w %fp,#0
movea.l (8,%fp),%a0
move.l (SENT).l,(%a0)
move.l (RECEIVED).l,(4,%a0)
move.l (DROPPED).l,(8,%a0)
movea.l (0xc,%fp),%a1
clr.l (%a1)
unlk %fp
rts
.globl AFTER_COPY
AFTER_COPY:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
movea.l (0x10,%fp),%a0
clr.l (%a0)
unlk %fp
rts
.globl CHECK_UNIT
CHECK_UNIT:
link.w %fp,#0
move.w (8,%fp),%d0
movea.l (0xa,%fp),%a0
clr.l (%a0)
cmpi.w #0x1f,%d0
ble.s 1f
move.l #0x100008,(%a0)
bra.s 2f
1:
tst.w (ACQUIRED).l
bne.s 3f
move.l #0x100009,(%a0)
3:
2:
unlk %fp
rts
.globl CALL_THEN_TEST
CALL_THEN_TEST:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
pea (%a2)
pea (1).w
jsr (EXT_$OP).l
addq.w #8,%sp
tst.l (%a2)
bne.s 1f
move.l #0x30004,(%a2)
1:
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl RECEIVE_CHECK
RECEIVE_CHECK:
link.w %fp,#-4
movea.l (0xc,%fp),%a0
move.l #0x2c000a,(%a0)
clr.b %d0
unlk %fp
rts
.balignw 4,0x2048
