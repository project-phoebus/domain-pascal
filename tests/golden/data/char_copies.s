.text
.globl READ_TZ
READ_TZ:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
clr.w (%a0)
lea (-4,%fp),%a0
lea (2,%a5),%a1
move.b (%a0)+,(%a1)+
move.b (%a0)+,(%a1)+
move.b (%a0)+,(%a1)+
move.b (%a0)+,(%a1)+
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl WRITE_TZ
WRITE_TZ:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a0
movea.l (8,%fp),%a1
lea (%a5),%a2
move.l (%a1)+,(%a2)+
move.l (%a1)+,(%a2)+
move.l (%a1)+,(%a2)+
move.w (%a5),(%a0)
lea (2,%a5),%a1
lea (2,%a0),%a2
move.b (%a1)+,(%a2)+
move.b (%a1)+,(%a2)+
move.b (%a1)+,(%a2)+
move.b (%a1)+,(%a2)+
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl COPY_IDS
COPY_IDS:
link.w %fp,#0
pea (%a2)
movea.l (0xc,%fp),%a0
movea.l (8,%fp),%a1
lea (%a0),%a2
moveq #0x17,%d0
.balignw 4,0x2048
1:
move.b (%a1)+,(%a2)+
dbf %d0,1b
clr.l (0x18,%a0)
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl GET_NAME
GET_NAME:
link.w %fp,#0
movea.l #NAMES,%a0
movea.l (8,%fp),%a1
moveq #0x1f,%d0
.balignw 4,0x2048
1:
move.b (%a0)+,(%a1)+
dbf %d0,1b
unlk %fp
rts
.globl SMALL
SMALL:
link.w %fp,#-0x24
lea (-0x10,%fp),%a0
lea (-8,%fp),%a1
moveq #7,%d0
.balignw 4,0x2048
1:
move.b (%a0)+,(%a1)+
dbf %d0,1b
lea (-0x24,%fp),%a0
lea (-0x1a,%fp),%a1
moveq #9,%d0
.balignw 4,0x2048
2:
move.b (%a0)+,(%a1)+
dbf %d0,2b
movea.l (8,%fp),%a0
clr.w (%a0)
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.CHARCOPY","aw",@nobits
.balign 4
.Ldata:
.space 12
