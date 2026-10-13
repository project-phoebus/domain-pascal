.text
GET:
link.w %fp,#-8
movea.l (%fp),%a0
move.w (8,%fp),%d0
add.w (-2,%a0),%d0
unlk %fp
rts
.globl NFE_$EXPR
NFE_$EXPR:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,(-2,%fp)
subq.l #2,%sp
move.w #2,-(%sp)
bsr.s GET
addq.w #4,%sp
move.w %d0,%d1
addq.w #3,%d1
move.w %d1,(-2,%fp)
subq.l #2,%sp
move.w #1,-(%sp)
bsr.s GET
addq.w #4,%sp
cmpi.w #3,%d0
ble.s 1f
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXT_$P).l
1:
unlk %fp
rts
.balignw 4,0x2048
