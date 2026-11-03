.set __dpc_fragment_origin,0xe16aa8
.text
.globl TIME_$GET_ADJUST
TIME_$GET_ADJUST:
link.w %fp,#-4
movea.l (8,%fp),%a1
ori.w #0x700,%sr
move.l (TIME_$CURRENT_DELTA).l,%d1
andi.w #0xf8ff,%sr
move.l #0x3d090,-(%sp)
move.l %d1,-(%sp)
jsr (M$DIS$LLL).l
addq.w #8,%sp
move.l %d0,(%a1)
move.l #0x3d090,-(%sp)
move.l %d1,-(%sp)
jsr (M$OIS$LLL).l
lsl.l #2,%d0
move.l %d0,(4,%a1)
unlk %fp
rts
.balignw 4,0x2048
