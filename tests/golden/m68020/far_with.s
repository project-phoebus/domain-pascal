.arch 68020
.text
.globl T
T:
link.w %fp,#-4
move.l (8,%fp),%d0
movea.l #M-1020000,%a0
move.l %d0,%d1
lsl.l #4,%d1
lea (0,%a0,%d1.l),%a0
tst.w (-0x6f94,%a0)
bpl.s 1f
btst #6,(-0x6f94,%a0)
bne.s 1f
tst.w (-0x6f9e,%a0)
beq.s 1f
movea.l (0xc,%fp),%a1
move.w (-0x6f9e,%a0),(%a1)
1:
unlk %fp
rts
.balignw 4,0x2048
