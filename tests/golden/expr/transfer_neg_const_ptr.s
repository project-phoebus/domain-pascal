.text
.globl TN_$CLEAR
TN_$CLEAR:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l #-1,%a1
movea.l (%a0),%a0
move.l %a1,(2,%a0)
pea (.Lc0,%pc)
jsr (TN_$TAKE).l
addq.w #4,%sp
pea (-1).w
jsr (TN_$TAKE_V).l
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0xffff,0xffff
.globl TN_$IS_NONE
TN_$IS_NONE:
link.w %fp,#-4
movea.l (8,%fp),%a0
movea.l (%a0),%a0
cmpi.l #-1,(2,%a0)
seq %d0
unlk %fp
rts
.globl TN_$IS_NONE2
TN_$IS_NONE2:
link.w %fp,#-4
movea.l (8,%fp),%a0
movea.l (%a0),%a0
cmpi.l #-1,(2,%a0)
seq %d0
unlk %fp
rts
.balignw 4,0x2048
