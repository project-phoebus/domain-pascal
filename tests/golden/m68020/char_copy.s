.arch 68020
.text
.globl P
P:
link.w %fp,#-0x10
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
lea (-0x10,%fp),%a1
lea (8,%a0),%a2
move.l (%a1)+,(%a2)+
move.l (%a1)+,(%a2)+
move.l (%a1)+,(%a2)+
lea (0x10,%a5),%a0
movea.l #D13,%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.b (%a0)+,(%a1)+
lea (0x1e,%a5),%a0
movea.l #D26,%a1
moveq #5,%d0
.balignw 4,0x2048
1:
move.l (%a0)+,(%a1)+
dbf %d0,1b
move.w (%a0)+,(%a1)+
lea (0x38,%a5),%a0
movea.l #D25,%a1
moveq #5,%d0
.balignw 4,0x2048
2:
move.l (%a0)+,(%a1)+
dbf %d0,2b
move.b (%a0)+,(%a1)+
move.l (%a5),(D4).l
movem.l (-0x18,%fp),%a2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.CHAR_COPY","aw",@nobits
.balign 4
.Ldata:
.space 84
