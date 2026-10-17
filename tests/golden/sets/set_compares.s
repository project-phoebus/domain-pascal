.text
.globl PRIO
PRIO:
link.w %fp,#-4
move.l (CUR).l,(-4,%fp)
movea.l (-4,%fp),%a0
moveq #0xb,%d0
and.w (0x54,%a0),%d0
cmpi.w #8,%d0
bne.s 1f
move.l (-4,%fp),-(%sp)
jsr (REORDER).l
1:
unlk %fp
rts
.globl TYPE4
TYPE4:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.w #-0x7ffc,%d0
and.w (%a0),%d0
cmpi.w #4,%d0
seq %d0
unlk %fp
rts
.globl HELD
HELD:
link.w %fp,#-4
move.l %d2,-(%sp)
movea.l (8,%fp),%a0
move.w (%a0),%d2
jsr (GO).l
cmpi.w #0x14,%d2
bne.s 1f
jsr (GO).l
1:
move.l (-8,%fp),%d2
unlk %fp
rts
.globl SIO
SIO:
link.w %fp,#-8
movea.l (8,%fp),%a0
lea (%a0),%a1
move.l (%a1)+,(-8,%fp)
move.l (%a1)+,(-4,%fp)
movea.l (0xc,%fp),%a1
btst #5,(3,%a1)
beq.s 1f
moveq #1,%d0
moveq #1,%d1
and.l (-8,%fp),%d0
and.l (%a0),%d1
cmp.l %d1,%d0
beq.s 2f
move.w #1,(GW).l
2:
1:
unlk %fp
rts
.globl READ
READ:
link.w %fp,#0
movem.l %a2-%a3,-(%sp)
movea.l (8,%fp),%a2
movea.l (0xc,%fp),%a3
tst.w (%a2)
bne.s 1f
pea (%a3)
pea (.Lc0,%pc)
pea (%a2)
jsr (K_GET).l
move.w #1,(GW).l
bra.s 2f
1:
pea (%a3)
pea (.Lc1,%pc)
pea (%a2)
jsr (K_GET).l
2:
movem.l (-8,%fp),%a2-%a3
unlk %fp
rts
.Lc0:
.short 0x0001
.Lc1:
.short 0x0000
.globl READ_COND
READ_COND:
link.w %fp,#0
move.l (0xc,%fp),-(%sp)
pea (.Lc0,%pc)
move.l (8,%fp),-(%sp)
jsr (K_GET).l
unlk %fp
rts
.globl INQ_PARAMS
INQ_PARAMS:
link.w %fp,#0
movem.l %a2-%a3,-(%sp)
movea.l (8,%fp),%a2
movea.l (0xc,%fp),%a3
pea (%a3)
pea (.Lc1,%pc)
pea (%a2)
jsr (K_GET).l
lea (0xc,%sp),%sp
pea (%a3)
pea (.Lc2,%pc)
pea (%a2)
jsr (INQ).l
lea (0xc,%sp),%sp
pea (%a3)
pea (.Lc3,%pc)
pea (%a2)
jsr (INQ).l
movem.l (-8,%fp),%a2-%a3
unlk %fp
rts
.balignw 4,0x2048
.Lc2:
.short 0x0000,0x0001
.Lc3:
.short 0x0000,0x2000
.balignw 4,0x2048
