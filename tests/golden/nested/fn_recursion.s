.text
COUNT:
link.w %fp,#-8
movem.l %d2/%a2,-(%sp)
move.w (8,%fp),%d2
movea.l %a1,%a2
move.w (-2,%a2),%d0
tst.w %d2
ble.s 1f
subq.l #2,%sp
move.w %d2,%d0
subq.w #1,%d0
move.w %d0,-(%sp)
movea.l %a2,%a1
bsr.s COUNT
add.w %d2,%d0
add.w (-2,%a2),%d0
1:
movem.l (-0x10,%fp),%d2/%a2
unlk %fp
rts
.globl NFR_$REC
NFR_$REC:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,(-2,%fp)
subq.l #2,%sp
move.w #3,-(%sp)
movea.l %fp,%a1
bsr.s COUNT
move.w %d0,(-2,%fp)
unlk %fp
rts
.balignw 4,0x2048
