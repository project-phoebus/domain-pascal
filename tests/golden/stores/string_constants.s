.text
.globl P
P:
link.w %fp,#-0x60
pea (%a5)
lea (.Ldata).l,%a5
move.l #0x2d2d2d2d,%d0
movea.l (8,%fp),%a0
moveq #4,%d1
.balignw 4,0x2048
1:
move.b %d0,(%a0)+
dbf %d1,1b
moveq #0x20,%d0
moveq #0x5a,%d1
.balignw 4,0x2048
2:
move.b %d0,(%a0)+
dbf %d1,2b
lea (.Lc0,%pc),%a0
movea.l (0xc,%fp),%a1
moveq #0xd,%d0
.balignw 4,0x2048
3:
move.b (%a0)+,(%a1)+
dbf %d0,3b
moveq #0x20,%d0
moveq #0x7d,%d1
.balignw 4,0x2048
4:
move.b %d0,(%a1)+
dbf %d1,4b
lea (.Lc1,%pc),%a0
lea (-0x60,%fp),%a1
moveq #4,%d0
.balignw 4,0x2048
5:
move.l (%a0)+,(%a1)+
dbf %d0,5b
move.w (%a0)+,(%a1)+
move.l #0x20202020,%d0
moveq #0x11,%d1
.balignw 4,0x2048
6:
move.l %d0,(%a1)+
dbf %d1,6b
move.w %d0,(%a1)+
lea (.Lc2,%pc),%a0
lea (-0x60,%fp),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.b (%a0)+,(%a1)+
move.l #0x20202020,%d0
move.b %d0,(%a1)+
moveq #0x14,%d1
.balignw 4,0x2048
7:
move.l %d0,(%a1)+
dbf %d1,7b
move.w %d0,(%a1)+
lea (%a5),%a0
move.l #0x2f636f6d,(%a0)+
move.l #0x20202020,%d0
moveq #0x3e,%d1
.balignw 4,0x2048
8:
move.l %d0,(%a0)+
dbf %d1,8b
movea.l (-0x64,%fp),%a5
unlk %fp
rts
.Lc0:
.short 0x5b66,0x726f,0x6d20,0x7072,0x6f63,0x6573,0x735d
.Lc2:
.short 0x5b69,0x676e,0x6f72,0x6564,0x5d00
.Lc1:
.short 0x5b73,0x7065,0x6369,0x6669,0x6564,0x2062,0x7920,0x7072,0x6f63,0x6573,0x735d
.balignw 4,0x2048
.section ".bss.moddata.STRING_CONSTANTS","aw",@nobits
.balign 4
.Ldata:
.space 256
