.text
.globl AS_IO
AS_IO:
link.w %fp,#0
movea.l (8,%fp),%a0
clr.l %d0
move.l #0x7fe,%d1
move.w (%a0),%d0
btst %d0,%d1
bne.s 1f
movea.l (0xc,%fp),%a1
move.l #0x8000f,(%a1)
1:
unlk %fp
rts
.globl DISMOUNT
DISMOUNT:
link.w %fp,#0
move.l %d2,-(%sp)
move.w (8,%fp),%d2
clr.l %d0
move.l #0x7fe,%d1
move.w %d2,%d0
btst %d0,%d1
beq.s 1f
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (DISK_$INVALIDATE).l
move.w %d2,(GW).l
1:
move.l (-4,%fp),%d2
unlk %fp
rts
.globl INT_IN
INT_IN:
link.w %fp,#-4
move.w (8,%fp),%d0
moveq #0x7e,%d1
btst %d0,%d1
sne %d1
move.b %d1,%d0
unlk %fp
rts
.globl ENUM_IN
ENUM_IN:
link.w %fp,#0
move.w (8,%fp),%d0
clr.w (GW).l
moveq #0x7a,%d1
btst %d0,%d1
beq.s 1f
move.w #1,(GW).l
1:
unlk %fp
rts
.balignw 4,0x2048
