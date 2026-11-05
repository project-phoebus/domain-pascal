.text
PUTS:
link.w %fp,#-8
movea.l (8,%fp),%a0
move.w (0xc,%fp),%d0
move.w (8,%a0),%d1
btst #5,%d1
beq.s 1f
bra.s 2f
1:
move.l (0xc,%a0),%d0
btst #3,%d0
beq.s 4f
tst.w (%a5)
beq.s 3f
4:
lsr.b #1,%d0
bcc.s 5f
move.w #1,(%a5)
bra.s 6f
5:
move.w #2,(%a5)
6:
3:
move.l (0xc,%a0),%d0
btst #1,%d0
beq.s 7f
move.w #3,(%a5)
7:
clr.w %d0
2:
unlk %fp
rts
.globl HELD
HELD:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
btst #12,%d0
beq.s 1f
move.w #4,(%a5)
1:
move.l %d0,%d1
lsr.l #4,%d1
btst #4,%d1
beq.s 2f
move.w #5,(%a5)
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ONE_BIT","aw",@nobits
.balign 4
.Ldata:
.space 4
