.text
.globl BLK_$GET_AVG
BLK_$GET_AVG:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
lea (%a5),%a0
movea.l (8,%fp),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BLK_$PUT_SMALL
BLK_$PUT_SMALL:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
lea (0xc,%a5),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.w (%a0)+,(%a1)+
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BLK_$GET_BIG
BLK_$GET_BIG:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
lea (0x1c,%a5),%a0
movea.l (8,%fp),%a1
moveq #0x23,%d0
.balignw 4,0x2048
1:
move.l (%a0)+,(%a1)+
dbf %d0,1b
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BLK_$GET_MID
BLK_$GET_MID:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
lea (0xac,%a5),%a0
movea.l (8,%fp),%a1
moveq #4,%d0
.balignw 4,0x2048
1:
move.l (%a0)+,(%a1)+
dbf %d0,1b
move.w (%a0)+,(%a1)+
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BLK","aw",@nobits
.balign 4
.Ldata:
.space 196
