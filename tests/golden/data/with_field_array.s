.text
.globl MT_PURGE_UCB
MT_PURGE_UCB:
link.w %fp,#-8
movem.l %a2-%a3,-(%sp)
movea.l (8,%fp),%a2
moveq #0x2e,%d0
movea.l #MT_$SCB,%a0
mulu.w (%a2),%d0
lea (0,%a0,%d0.w),%a3
bset #2,(0x44,%a3)
bra.s 2f
.balignw 4,0x2048
1:
pea (.Lc0,%pc)
pea (0x3c,%a3)
jsr (MT_$DEQ).l
addq.w #4,%sp
move.l %a0,(-8,%fp)
pea (-8,%fp)
pea (%a2)
jsr (MT_$PURGE_PB).l
lea (0xc,%sp),%sp
2:
tst.l (0x3c,%a3)
bne.s 1b
bclr #2,(0x44,%a3)
movem.l (-0x10,%fp),%a2-%a3
unlk %fp
rts
.Lc0:
.short 0xff00
.balignw 4,0x2048
