.text
.globl OK
OK:
link.w %fp,#-8
movem.l %d2-%d3,-(%sp)
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
moveq #1,%d0
moveq #0x27,%d1
moveq #1,%d2
.balignw 4,0x2048
2:
move.b (-1,%a0,%d2.w),%d3
cmp.b (-1,%a1,%d2.w),%d3
beq.s 4f
clr.l %d0
4:
addq.w #1,%d2
dbf %d1,2b
movem.l (-0x10,%fp),%d2-%d3
unlk %fp
rts
.globl CHECK
CHECK:
link.w %fp,#-8
move.l %d2,-(%sp)
movea.l (0x10,%fp),%a0
move.l (%a0),%d2
move.l (0xc,%fp),-(%sp)
move.l (8,%fp),-(%sp)
bsr.s OK
addq.w #8,%sp
move.l %d0,(-4,%fp)
cmpi.l #0x1234,%d2
beq.s 1f
clr.l (-4,%fp)
1:
pea (-4,%fp)
jsr (EXEC_$LOG_L).l
move.l (-0xc,%fp),%d2
unlk %fp
rts
.globl BY_POINTER
BY_POINTER:
link.w %fp,#-4
pea (%a2)
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
move.w (%a1),%d0
move.w %d0,%d1
subq.w #1,%d1
cmpi.w #5,%d1
bcc.w 7f
add.w %d1,%d1
move.w (8f,%pc,%d1.w),%d1
jmp (8f,%pc,%d1.w)
8:
.short 2f-8b
.short 3f-8b
.short 4f-8b
.short 5f-8b
.short 6f-8b
2:
lea (%a0),%a1
move.b #0x78,(%a1)+
moveq #0x20,%d0
moveq #0x26,%d1
.balignw 4,0x2048
9:
move.b %d0,(%a1)+
dbf %d1,9b
bra.s 1f
3:
lea (%a0),%a1
move.b #0x78,(%a1)+
move.b #0x79,(%a1)+
moveq #0x20,%d0
moveq #0x25,%d1
.balignw 4,0x2048
10:
move.b %d0,(%a1)+
dbf %d1,10b
bra.s 1f
4:
lea (.Lc0,%pc),%a1
lea (%a0),%a2
move.b (%a1)+,(%a2)+
move.b (%a1)+,(%a2)+
move.b (%a1)+,(%a2)+
moveq #0x20,%d0
moveq #0x24,%d1
.balignw 4,0x2048
11:
move.b %d0,(%a2)+
dbf %d1,11b
bra.s 1f
5:
lea (.Lc1,%pc),%a1
lea (%a0),%a2
moveq #5,%d0
.balignw 4,0x2048
12:
move.b (%a1)+,(%a2)+
dbf %d0,12b
moveq #0x20,%d0
moveq #0x21,%d1
.balignw 4,0x2048
13:
move.b %d0,(%a2)+
dbf %d1,13b
bra.s 1f
6:
move.l #0x2d2d2d2d,%d0
lea (%a0),%a1
moveq #4,%d1
.balignw 4,0x2048
14:
move.b %d0,(%a1)+
dbf %d1,14b
moveq #0x20,%d0
moveq #0x22,%d1
.balignw 4,0x2048
15:
move.b %d0,(%a1)+
dbf %d1,15b
bra.s 1f
7:
move.l #0x20202020,%d0
lea (%a0),%a1
moveq #0x27,%d1
.balignw 4,0x2048
16:
move.b %d0,(%a1)+
dbf %d1,16b
1:
movea.l (-8,%fp),%a2
unlk %fp
rts
.Lc1:
.short 0x6162,0x6364,0x6566
.balignw 4,0x2048
.Lc0:
.short 0x6162,0x6300
.globl EXEC_ALL
EXEC_ALL:
link.w %fp,#-0x58
pea (%a5)
lea (.Ldata).l,%a5
move.l #0x1234,(0x28,%a5)
move.l #0x1234,(-0x30,%fp)
lea (%a5),%a0
move.b #0x78,(%a0)+
move.l #0x20202020,%d0
move.b %d0,(%a0)+
moveq #8,%d1
.balignw 4,0x2048
1:
move.l %d0,(%a0)+
dbf %d1,1b
move.w %d0,(%a0)+
lea (.Lc2,%pc),%a0
lea (-0x28,%fp),%a1
moveq #9,%d0
.balignw 4,0x2048
2:
move.l (%a0)+,(%a1)+
dbf %d0,2b
pea (0x28,%a5)
pea (-0x28,%fp)
pea (%a5)
bsr.w CHECK
lea (0xc,%sp),%sp
lea (%a5),%a0
move.w #0x7879,(%a0)+
move.l #0x20202020,%d0
moveq #8,%d1
.balignw 4,0x2048
3:
move.l %d0,(%a0)+
dbf %d1,3b
move.w %d0,(%a0)+
lea (.Lc3,%pc),%a0
lea (-0x28,%fp),%a1
moveq #9,%d0
.balignw 4,0x2048
4:
move.l (%a0)+,(%a1)+
dbf %d0,4b
pea (0x28,%a5)
pea (-0x28,%fp)
pea (%a5)
bsr.w CHECK
lea (0xc,%sp),%sp
lea (-0x58,%fp),%a0
move.w #0x6162,(%a0)+
move.b #0x63,(%a0)+
move.l #0x20202020,%d0
move.b %d0,(%a0)+
moveq #8,%d1
.balignw 4,0x2048
5:
move.l %d0,(%a0)+
dbf %d1,5b
lea (.Lc4,%pc),%a0
lea (-0x28,%fp),%a1
moveq #9,%d0
.balignw 4,0x2048
6:
move.l (%a0)+,(%a1)+
dbf %d0,6b
pea (-0x30,%fp)
pea (-0x28,%fp)
pea (-0x58,%fp)
bsr.w CHECK
lea (0xc,%sp),%sp
lea (-0x58,%fp),%a0
move.l #0x61626364,(%a0)+
move.b #0x65,(%a0)+
move.l #0x20202020,%d0
move.b %d0,(%a0)+
moveq #7,%d1
.balignw 4,0x2048
7:
move.l %d0,(%a0)+
dbf %d1,7b
move.w %d0,(%a0)+
lea (.Lc5,%pc),%a0
lea (-0x28,%fp),%a1
moveq #9,%d0
.balignw 4,0x2048
8:
move.l (%a0)+,(%a1)+
dbf %d0,8b
pea (-0x30,%fp)
pea (-0x28,%fp)
pea (-0x58,%fp)
bsr.w CHECK
lea (0xc,%sp),%sp
lea (.Lc6,%pc),%a0
lea (%a5),%a1
move.l (%a0)+,(%a1)+
move.w (%a0)+,(%a1)+
move.b (%a0)+,(%a1)+
move.l #0x20202020,%d0
move.b %d0,(%a1)+
moveq #7,%d1
.balignw 4,0x2048
9:
move.l %d0,(%a1)+
dbf %d1,9b
lea (.Lc7,%pc),%a0
lea (-0x28,%fp),%a1
moveq #9,%d0
.balignw 4,0x2048
10:
move.l (%a0)+,(%a1)+
dbf %d0,10b
pea (0x28,%a5)
pea (-0x28,%fp)
pea (%a5)
bsr.w CHECK
lea (0xc,%sp),%sp
lea (.Lc8,%pc),%a0
lea (%a5),%a1
moveq #4,%d0
.balignw 4,0x2048
11:
move.l (%a0)+,(%a1)+
dbf %d0,11b
move.w (%a0)+,(%a1)+
move.b (%a0)+,(%a1)+
move.l #0x20202020,%d0
move.b %d0,(%a1)+
move.l %d0,(%a1)+
move.l %d0,(%a1)+
move.l %d0,(%a1)+
move.l %d0,(%a1)+
lea (.Lc9,%pc),%a0
lea (-0x28,%fp),%a1
moveq #9,%d0
.balignw 4,0x2048
12:
move.l (%a0)+,(%a1)+
dbf %d0,12b
pea (0x28,%a5)
pea (-0x28,%fp)
pea (%a5)
bsr.w CHECK
lea (0xc,%sp),%sp
move.l #0x20202020,%d0
lea (-0x58,%fp),%a0
moveq #9,%d1
.balignw 4,0x2048
13:
move.l %d0,(%a0)+
dbf %d1,13b
move.l #0x20202020,%d0
lea (-0x28,%fp),%a0
moveq #9,%d1
.balignw 4,0x2048
14:
move.l %d0,(%a0)+
dbf %d1,14b
pea (-0x30,%fp)
pea (-0x28,%fp)
pea (-0x58,%fp)
bsr.w CHECK
lea (0xc,%sp),%sp
pea (.Lc10,%pc)
pea (%a5)
bsr.w BY_POINTER
addq.w #8,%sp
lea (.Lc2,%pc),%a0
lea (-0x28,%fp),%a1
moveq #9,%d0
.balignw 4,0x2048
15:
move.l (%a0)+,(%a1)+
dbf %d0,15b
pea (0x28,%a5)
pea (-0x28,%fp)
pea (%a5)
bsr.w CHECK
lea (0xc,%sp),%sp
pea (.Lc11,%pc)
pea (%a5)
bsr.w BY_POINTER
addq.w #8,%sp
lea (.Lc3,%pc),%a0
lea (-0x28,%fp),%a1
moveq #9,%d0
.balignw 4,0x2048
16:
move.l (%a0)+,(%a1)+
dbf %d0,16b
pea (0x28,%a5)
pea (-0x28,%fp)
pea (%a5)
bsr.w CHECK
lea (0xc,%sp),%sp
pea (.Lc12,%pc)
pea (%a5)
bsr.w BY_POINTER
addq.w #8,%sp
lea (.Lc4,%pc),%a0
lea (-0x28,%fp),%a1
moveq #9,%d0
.balignw 4,0x2048
17:
move.l (%a0)+,(%a1)+
dbf %d0,17b
pea (0x28,%a5)
pea (-0x28,%fp)
pea (%a5)
bsr.w CHECK
lea (0xc,%sp),%sp
pea (.Lc13,%pc)
pea (%a5)
bsr.w BY_POINTER
addq.w #8,%sp
lea (.Lc14,%pc),%a0
lea (-0x28,%fp),%a1
moveq #9,%d0
.balignw 4,0x2048
18:
move.l (%a0)+,(%a1)+
dbf %d0,18b
pea (0x28,%a5)
pea (-0x28,%fp)
pea (%a5)
bsr.w CHECK
lea (0xc,%sp),%sp
pea (.Lc15,%pc)
pea (%a5)
bsr.w BY_POINTER
addq.w #8,%sp
lea (.Lc16,%pc),%a0
lea (-0x28,%fp),%a1
moveq #9,%d0
.balignw 4,0x2048
19:
move.l (%a0)+,(%a1)+
dbf %d0,19b
pea (0x28,%a5)
pea (-0x28,%fp)
pea (%a5)
bsr.w CHECK
lea (0xc,%sp),%sp
pea (.Lc17,%pc)
pea (%a5)
bsr.w BY_POINTER
addq.w #8,%sp
move.l #0x20202020,%d0
lea (-0x28,%fp),%a0
moveq #9,%d1
.balignw 4,0x2048
20:
move.l %d0,(%a0)+
dbf %d1,20b
pea (0x28,%a5)
pea (-0x28,%fp)
pea (%a5)
bsr.w CHECK
movea.l (-0x5c,%fp),%a5
unlk %fp
rts
.Lc10:
.short 0x0001
.Lc17:
.short 0x0006
.Lc15:
.short 0x0005
.Lc13:
.short 0x0004
.Lc12:
.short 0x0003
.Lc11:
.short 0x0002
.balignw 4,0x2048
.Lc2:
.short 0x7820,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020
.Lc16:
.short 0x2d2d,0x2d2d,0x2d20,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020
.Lc14:
.short 0x6162,0x6364,0x6566,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020
.Lc9:
.short 0x6162,0x6364,0x6566,0x6768,0x696a,0x6b6c,0x6d6e,0x6f70,0x7172,0x7374,0x7576,0x7720,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020
.Lc8:
.short 0x6162,0x6364,0x6566,0x6768,0x696a,0x6b6c,0x6d6e,0x6f70,0x7172,0x7374,0x7576,0x7700
.Lc7:
.short 0x6162,0x6364,0x6566,0x6720,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020
.Lc6:
.short 0x6162,0x6364,0x6566,0x6700
.Lc5:
.short 0x6162,0x6364,0x6520,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020
.Lc4:
.short 0x6162,0x6320,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020
.Lc3:
.short 0x7879,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020
.globl STRS_$RUN
STRS_$RUN:
link.w %fp,#0
bsr.w EXEC_ALL
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.STRS","aw",@nobits
.balign 4
.Ldata:
.space 44
