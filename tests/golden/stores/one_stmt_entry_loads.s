.text
.globl ENABLE_CRASH
ENABLE_CRASH:
link.w %fp,#0
movea.l (8,%fp),%a0
move.b (0xc,%fp),%d0
move.b (0xe,%fp),%d1
bpl.s 1f
move.b %d0,(0x4d8,%a0)
clr.w %d1
move.b %d0,%d1
add.w %d1,%d1
lea (0,%a0,%d1.w),%a0
move.w #0x11,(0xa4,%a0)
bra.s 2f
1:
tst.b (0x4d8,%a0)
beq.s 3f
clr.w %d1
move.b %d0,%d1
add.w %d1,%d1
lea (0,%a0,%d1.w),%a1
move.w #0x12,(0xa4,%a1)
3:
clr.b (0x4d8,%a0)
2:
unlk %fp
rts
.globl TST_FIRST
TST_FIRST:
link.w %fp,#0
move.b (0xc,%fp),%d0
move.b (0xe,%fp),%d1
tst.b %d0
bpl.s 1f
movea.l (8,%fp),%a0
move.b %d1,(0x4d8,%a0)
1:
unlk %fp
rts
.balignw 4,0x2048
