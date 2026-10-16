.text
NEAR_UP:
link.w %fp,#-4
move.w (PROC1_$CURRENT).l,%d0
add.w %d0,%d0
move.w (-2,%a5,%d0.w),%d1
addq.w #1,%d1
move.w %d1,(-2,%a5,%d0.w)
unlk %fp
rts
NEAR_DOWN:
link.w %fp,#-4
move.w (PROC1_$CURRENT).l,%d0
add.w %d0,%d0
move.w (-2,%a5,%d0.w),%d1
subq.w #1,%d1
move.w %d1,(-2,%a5,%d0.w)
unlk %fp
rts
FAR_UP:
link.w %fp,#-4
move.w (PROC1_$CURRENT).l,%d0
add.w %d0,%d0
lea (0,%a5,%d0.w),%a0
addq.w #1,(0x84e,%a0)
unlk %fp
rts
FAR_DOWN:
link.w %fp,#-4
move.w (PROC1_$CURRENT).l,%d0
add.w %d0,%d0
lea (0,%a5,%d0.w),%a0
subq.w #1,(0x84e,%a0)
unlk %fp
rts
OTHER_UNIT_FIELD:
link.w %fp,#-4
move.w (PROC1_$CURRENT).l,%d0
movea.l #PROC1_$STATS,%a0
lsl.w #4,%d0
addq.l #1,(-4,%a0,%d0.w)
unlk %fp
rts
OTHER_UNIT_BYTE:
link.w %fp,#-4
move.w (PROC1_$CURRENT).l,%d0
movea.l #BVALS,%a0
subq.b #1,(-1,%a0,%d0.w)
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ELEMENT_UPDATE","aw",@nobits
.balign 4
.Ldata:
.space 2256
