.text
GET:
link.w %fp,#-8
movem.l %d2/%a2,-(%sp)
movea.l (%fp),%a2
subq.l #2,%sp
move.w (8,%fp),-(%sp)
jsr (EXT_$F).l
addq.w #4,%sp
move.w %d0,%d2
subq.l #2,%sp
move.w (-2,%a2),-(%sp)
jsr (EXT_$P).l
move.w %d2,%d0
movem.l (-0x10,%fp),%d2/%a2
unlk %fp
rts
.globl NFH_$HELD
NFH_$HELD:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,(-2,%fp)
subq.l #2,%sp
move.w #3,-(%sp)
bsr.s GET
move.w %d0,(-2,%fp)
unlk %fp
rts
.balignw 4,0x2048
