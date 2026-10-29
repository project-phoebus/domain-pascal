.arch 68020
.text
.globl P
P:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (BP).l,%a0
move.w (%a0),(%a5)
move.w (R+1).l,(%a5)
move.w #7,(R+1).l
movea.l #X,%a0
moveq #3,%d0
.balignw 4,0x2048
1:
clr.w (%a0)+
dbf %d0,1b
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ODD_ACCESS","aw",@nobits
.balign 4
.Ldata:
.space 4
