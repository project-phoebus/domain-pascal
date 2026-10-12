.text
.globl FILL_$PAGE
FILL_$PAGE:
link.w %fp,#-4
movea.l #PAGE,%a0
move.w #0xff,%d0
.balignw 4,0x2048
1:
clr.l (%a0)+
dbf %d0,1b
unlk %fp
rts
.globl FILL_$SMALL
FILL_$SMALL:
link.w %fp,#-4
jsr (EXT_$P).l
movea.l #WORDS,%a0
moveq #0x1f,%d0
.balignw 4,0x2048
1:
clr.w (%a0)+
dbf %d0,1b
jsr (EXT_$P).l
movea.l #FLAGS,%a0
moveq #2,%d0
.balignw 4,0x2048
2:
clr.b (%a0)+
dbf %d0,2b
unlk %fp
rts
.balignw 4,0x2048
