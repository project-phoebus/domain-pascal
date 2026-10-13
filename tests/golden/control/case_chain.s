.text
.globl SYSBUS
SYSBUS:
link.w %fp,#0
movea.l (DEV).l,%a0
move.w (4,%a0),%d0
beq.s 2f
cmpi.w #1,%d0
beq.s 3f
cmpi.w #2,%d0
beq.s 4f
bra.s 1f
2:
move.w #1,(X1).l
bra.s 1f
3:
move.w #2,(X2).l
bra.s 1f
4:
move.w #3,(X3).l
1:
unlk %fp
rts
.globl GET_EC
GET_EC:
link.w %fp,#0
move.w (K).l,%d0
cmpi.w #1,%d0
beq.s 2f
tst.w %d0
beq.s 3f
bra.s 4f
2:
move.w #1,(X1).l
bra.s 1f
3:
move.w #2,(X1).l
bra.s 1f
4:
move.l #0xd0005,(ST).l
1:
unlk %fp
rts
.globl KEY
KEY:
link.w %fp,#0
movea.l (8,%fp),%a0
clr.w %d0
move.b (C).l,%d0
cmpi.w #0xc0,%d0
beq.s 2f
cmpi.w #0x91,%d0
beq.s 3f
cmpi.w #0x92,%d0
beq.s 4f
cmpi.w #0x95,%d0
beq.s 5f
cmpi.w #0x96,%d0
beq.s 6f
cmpi.w #0x99,%d0
beq.s 7f
cmpi.w #0x98,%d0
beq.s 7f
cmpi.w #0x97,%d0
beq.s 7f
bra.s 8f
2:
move.w #0x16,(%a0)
bra.s 1f
3:
move.w #0x18,(%a0)
bra.s 1f
4:
move.w #4,(%a0)
bra.s 1f
5:
move.w #8,(%a0)
bra.s 1f
6:
move.w #0xd,(%a0)
bra.s 1f
7:
move.w #9,(%a0)
bra.s 1f
8:
clr.w (%a0)
1:
unlk %fp
rts
.globl STATUS_CHAIN
STATUS_CHAIN:
link.w %fp,#0
move.l (ST).l,%d0
cmpi.l #0xf0010,%d0
beq.s 2f
cmpi.l #0xf0003,%d0
beq.s 3f
cmpi.l #0xf0011,%d0
beq.s 4f
bra.s 1f
2:
move.w #1,(X1).l
bra.s 1f
3:
move.w #2,(X1).l
bra.s 1f
4:
move.w #3,(X1).l
1:
unlk %fp
rts
SIGNAL:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,(X1).l
move.w %d0,%d1
cmpi.w #3,%d1
beq.s 2f
cmpi.w #2,%d1
beq.s 3f
cmpi.w #0x15,%d1
beq.s 4f
cmpi.w #1,%d1
beq.s 5f
cmpi.w #0x1a,%d1
beq.s 6f
bra.s 1f
2:
clr.w (X2).l
bra.s 1f
3:
move.w #1,(X2).l
bra.s 1f
4:
move.w #2,(X2).l
bra.s 1f
5:
move.w #3,(X2).l
bra.s 1f
6:
move.w #4,(X2).l
1:
unlk %fp
rts
.balignw 4,0x2048
