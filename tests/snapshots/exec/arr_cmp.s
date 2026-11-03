.text
.globl ARR_CMP_$LONGS
ARR_CMP_$LONGS:
link.w %fp,#-4
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.l (0xa,%fp),%d1
clr.w %d2
bra.s 2f
.balignw 4,0x2048
1:
move.w %d2,%d3
move.w %d2,%d4
ext.l %d3
lsl.w #2,%d4
move.l %d3,(4,%a5,%d4.w)
move.w %d2,%d3
move.w %d2,%d4
ext.l %d3
lsl.w #2,%d4
move.l %d3,(0x28,%a5,%d4.w)
addq.w #1,%d2
2:
cmpi.w #8,%d2
blt.s 1b
move.w %d0,%d2
lsl.w #2,%d2
move.l %d1,(0x28,%a5,%d2.w)
lea (4,%a5),%a0
lea (0x28,%a5),%a1
moveq #7,%d0
.balignw 4,0x2048
5:
cmpm.l (%a1)+,(%a0)+
dbne %d0,5b
seq %d0
move.b %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
lea (4,%a5),%a0
lea (0x28,%a5),%a1
moveq #7,%d0
.balignw 4,0x2048
8:
cmpm.l (%a1)+,(%a0)+
dbne %d0,8b
beq.s 6f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 10f
6:
subq.l #2,%sp
clr.w -(%sp)
10:
jsr (EXEC_$MARK).l
9:
movem.l (-0x14,%fp),%d2-%d4/%a5
unlk %fp
rts
.globl ARR_CMP_$NAME
ARR_CMP_$NAME:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.b (0xa,%fp),%d1
move.w %d0,(0x48,%a5)
move.b #0x41,(0x4a,%a5)
move.b #0x50,(0x4b,%a5)
move.b #0x4f,(0x4c,%a5)
move.b #0x4c,(0x4d,%a5)
move.b #0x4c,(0x4e,%a5)
move.b #0x4f,(0x4f,%a5)
move.b %d1,(0x49,%a5,%d0.w)
lea (0x48,%a5),%a0
move.l %a0,(0x54,%a5)
lea (.Lc0,%pc),%a0
movea.l (0x54,%a5),%a1
moveq #5,%d0
lea (2,%a1),%a2
.balignw 4,0x2048
2:
cmpm.b (%a2)+,(%a0)+
dbne %d0,2b
seq %d0
move.b %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
movea.l (0x54,%a5),%a0
cmpi.w #1,(%a0)
sgt %d0
lea (2,%a0),%a1
lea (.Lc0,%pc),%a0
moveq #5,%d1
.balignw 4,0x2048
5:
cmpm.b (%a0)+,(%a1)+
dbne %d1,5b
sne %d1
or.b %d1,%d0
bpl.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 7f
3:
subq.l #2,%sp
clr.w -(%sp)
7:
jsr (EXEC_$MARK).l
6:
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.Lc0:
.short 0x4150,0x4f4c,0x4c4f
.balignw 4,0x2048
.section ".bss.moddata.ARR_CMP","aw",@nobits
.balign 4
.Ldata:
.space 88
