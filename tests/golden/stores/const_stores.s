.text
.globl P
P:
link.w %fp,#0
movea.l (8,%fp),%a0
st (%a0)
clr.l (4,%a0)
move.b #0xff,(8,%a0)
move.w #-1,(0xa,%a0)
move.l #0x6000e,(0xc,%a0)
move.w #0x200,(2,%a0)
unlk %fp
rts
.balignw 4,0x2048
