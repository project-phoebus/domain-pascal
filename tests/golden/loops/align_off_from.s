.text
.globl ALO_$MOUNT
ALO_$MOUNT:
link.w %fp,#-4
movea.l #PAGE,%a0
move.w #0xff,%d0
.balignw 4,0x2048
1:
clr.l (%a0)+
dbf %d0,1b
unlk %fp
rts
ALO_$WAIT:
link.w %fp,#-4
.balignw 4,0x2048
1:
move.w #1,(W).l
cmpi.w #3,(W).l
bne.s 1b
unlk %fp
rts
.globl ALO_$DISMOUNT
ALO_$DISMOUNT:
link.w %fp,#-4
movea.l #PAGE,%a0
move.w #0xff,%d0
1:
clr.l (%a0)+
dbf %d0,1b
unlk %fp
rts
ALO_$IDLE:
link.w %fp,#-4
1:
move.w #1,(W).l
cmpi.w #3,(W).l
bne.s 1b
unlk %fp
rts
.balignw 4,0x0000
