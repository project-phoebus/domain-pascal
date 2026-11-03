.text
.globl GOTOS_$FORWARD
GOTOS_$FORWARD:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(%a5)
tst.w (%a5)
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w #2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
subq.l #2,%sp
move.w #3,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl GOTOS_$BACKWARD
GOTOS_$BACKWARD:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.w (%a5)
move.w %d0,(8,%a5)
.balignw 4,0x2048
1:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
addq.w #1,(%a5)
move.w (%a5),%d0
cmp.w (8,%a5),%d0
ble.s 1b
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl GOTOS_$OUT_WHILE
GOTOS_$OUT_WHILE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(8,%a5)
clr.w (%a5)
bra.s 2f
.balignw 4,0x2048
1:
move.w (8,%a5),%d0
cmp.w (%a5),%d0
beq.s 4f
addq.w #1,(%a5)
2:
cmpi.w #0xa,(%a5)
blt.s 1b
subq.l #2,%sp
move.w #0x28,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
4:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl GOTOS_$OUT_FOR
GOTOS_$OUT_FOR:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(8,%a5)
moveq #9,%d2
moveq #1,%d3
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
cmp.w (8,%a5),%d3
beq.s 4f
addq.w #1,%d3
dbf %d2,2b
subq.l #2,%sp
move.w #0x2a,-(%sp)
bra.s 6f
4:
subq.l #2,%sp
move.w #0x29,-(%sp)
6:
jsr (EXEC_$MARK).l
5:
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl GOTOS_$ARMS
GOTOS_$ARMS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,(%a5)
tst.w %d1
beq.s 1f
st (0xa,%a5)
bra.s 2f
1:
clr.b (0xa,%a5)
2:
move.w (%a5),%d0
cmpi.w #4,%d0
bcc.s 3f
add.w %d0,%d0
move.w (8f,%pc,%d0.w),%d0
jmp (8f,%pc,%d0.w)
8:
.short 4f-8b
.short 5f-8b
.short 6f-8b
.short 7f-8b
4:
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 10f
5:
tst.b (0xa,%a5)
bpl.s 9f
subq.l #2,%sp
move.w #2,-(%sp)
bra.s 10f
6:
subq.l #2,%sp
move.w #3,-(%sp)
bra.s 10f
7:
9:
subq.l #2,%sp
move.w #4,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w #5,-(%sp)
10:
jsr (EXEC_$MARK).l
3:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl GOTOS_$RESULT
GOTOS_$RESULT:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d1
moveq #1,%d0
move.w %d1,(%a5)
tst.w (%a5)
bne.s 1f
subq.l #2,%sp
move.w #3,-(%sp)
jsr (EXEC_$MARK).l
moveq #2,%d0
1:
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.GOTOS","aw",@nobits
.balign 4
.Ldata:
.space 12
