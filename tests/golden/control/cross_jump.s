.text
.globl P_$TAILS
P_$TAILS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a0
tst.b (%a0)
bpl.s 1f
pea (.Lc0,%pc)
bra.s 3f
1:
pea (.Lc1,%pc)
3:
jsr (LOG_IT).l
addq.w #4,%sp
addq.l #1,(%a5)
2:
pea (.Lc2,%pc)
jsr (LOG_IT).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.Lc0:
.short 0x0001
.Lc2:
.short 0x0003
.Lc1:
.short 0x0002
.globl P_$ARMS
P_$ARMS:
link.w %fp,#-4
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
move.w (%a1),%d0
cmpi.w #1,%d0
beq.s 2f
cmpi.w #2,%d0
beq.s 3f
cmpi.w #3,%d0
beq.s 3f
bra.s 5f
2:
moveq #0xa,%d0
bra.s 1f
3:
movea.l (%a0),%a0
move.w #5,(4,%a0)
moveq #0x14,%d0
bra.s 1f
5:
clr.w %d0
1:
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.CROSS_JUMP","aw",@nobits
.balign 4
.Ldata:
.space 4
