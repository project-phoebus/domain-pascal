.text
.globl NEXT_INNER_ELSE
NEXT_INNER_ELSE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w #3,(2,%a5)
clr.w (6,%a5)
bra.s 2f
.balignw 4,0x2048
1:
subq.w #1,(2,%a5)
cmpi.w #1,(4,%a5)
bne.s 4f
cmpi.w #1,(8,%a5)
bne.s 2f
addq.w #2,(6,%a5)
bra.s 7f
4:
addq.w #3,(6,%a5)
7:
addi.w #0xa,(6,%a5)
2:
tst.w (2,%a5)
bgt.s 1b
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl EXIT_INNER_ELSE
EXIT_INNER_ELSE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w #3,(2,%a5)
clr.w (6,%a5)
bra.s 2f
.balignw 4,0x2048
1:
subq.w #1,(2,%a5)
cmpi.w #1,(4,%a5)
bne.s 4f
cmpi.w #1,(8,%a5)
bne.s 3f
addq.w #2,(6,%a5)
bra.s 7f
4:
addq.w #3,(6,%a5)
7:
addi.w #0xa,(6,%a5)
2:
tst.w (2,%a5)
bgt.s 1b
3:
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w (2,%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl RETURN_INNER_ELSE
RETURN_INNER_ELSE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w #3,(2,%a5)
clr.w (6,%a5)
bra.s 2f
.balignw 4,0x2048
1:
subq.w #1,(2,%a5)
cmpi.w #1,(4,%a5)
bne.s 4f
cmpi.w #1,(8,%a5)
bne.s 7f
addq.w #2,(6,%a5)
bra.s 8f
4:
addq.w #3,(6,%a5)
8:
addi.w #0xa,(6,%a5)
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
tst.w (2,%a5)
bgt.s 1b
subq.l #2,%sp
move.w (2,%a5),-(%sp)
jsr (EXEC_$MARK).l
7:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl GOTO_INNER_ELSE
GOTO_INNER_ELSE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w #0x64,(6,%a5)
cmpi.w #1,(%a5)
bne.s 1f
cmpi.w #1,(8,%a5)
bne.s 2f
move.w #0x66,(6,%a5)
bra.s 3f
1:
move.w #0x67,(6,%a5)
3:
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
subq.l #2,%sp
move.w (6,%a5),%d0
addi.w #0x3e8,%d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl EXIT_ELSE_WHILE
EXIT_ELSE_WHILE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w #2,(2,%a5)
move.w (8,%a5),%d0
addq.w #5,%d0
move.w %d0,(6,%a5)
bra.s 2f
.balignw 4,0x2048
1:
subq.w #1,(2,%a5)
cmpi.w #1,(4,%a5)
beq.s 3f
bra.s 5f
.balignw 4,0x2048
4:
subq.w #1,(6,%a5)
5:
tst.w (6,%a5)
bgt.s 4b
2:
tst.w (2,%a5)
bgt.s 1b
3:
subq.l #2,%sp
move.w (2,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ELSE_WHILES
ELSE_WHILES:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%a5),(6,%a5)
move.w (8,%a5),%d0
addq.w #2,%d0
move.w %d0,(2,%a5)
cmpi.w #1,(%a5)
bne.s 4f
move.w #0x32,(6,%a5)
bra.s 2f
.balignw 4,0x2048
3:
subq.w #1,(6,%a5)
4:
tst.w (6,%a5)
bgt.s 3b
2:
cmpi.w #1,(4,%a5)
bne.s 9f
move.w #0x3c,(2,%a5)
bra.s 7f
.balignw 4,0x2048
8:
subq.w #1,(2,%a5)
9:
cmpi.w #1,(2,%a5)
bgt.s 8b
7:
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w (2,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w (8,%a5),(6,%a5)
cmpi.w #1,(%a5)
bne.s 14f
move.w #0x46,(6,%a5)
bra.s 12f
.balignw 4,0x2048
16:
subq.w #1,(6,%a5)
17:
tst.w (6,%a5)
bgt.s 16b
14:
tst.w (6,%a5)
bgt.s 17b
12:
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl GOTO_ELSE_WHILE
GOTO_ELSE_WHILE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%a5),(6,%a5)
move.w #0x50,(2,%a5)
tst.w (%a5)
beq.s 3f
cmpi.w #1,(4,%a5)
bne.s 5f
move.w #0x51,(2,%a5)
bra.s 3f
.balignw 4,0x2048
4:
subq.w #1,(6,%a5)
5:
tst.w (6,%a5)
bgt.s 4b
3:
subq.l #2,%sp
move.w (2,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BACK_GOTO
BACK_GOTO:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w #3,(2,%a5)
clr.w (6,%a5)
.balignw 4,0x2048
1:
subq.w #1,(2,%a5)
addq.w #1,(6,%a5)
tst.w (2,%a5)
ble.s 2f
cmpi.w #1,(4,%a5)
beq.s 1b
bra.s 3f
2:
addi.w #0x5a,(6,%a5)
3:
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FRES
FRES:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
moveq #1,%d0
move.w (8,%a5),(6,%a5)
cmpi.w #1,(%a5)
bne.s 7f
cmpi.w #1,(4,%a5)
bne.s 4f
moveq #2,%d0
bra.s 5f
.balignw 4,0x2048
6:
subq.w #1,(6,%a5)
7:
tst.w (6,%a5)
bgt.s 6b
5:
4:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl CASES
CASES:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.w (6,%a5)
tst.w (%a5)
bne.s 1f
move.w (0xc,%a5),%d0
cmpi.w #4,%d0
bcc.s 8f
add.w %d0,%d0
move.w (7f,%pc,%d0.w),%d0
jmp (7f,%pc,%d0.w)
7:
.short 3f-7b
.short 4f-7b
.short 5f-7b
.short 6f-7b
3:
move.w #1,(6,%a5)
bra.s 8f
4:
move.w #2,(6,%a5)
bra.s 8f
5:
move.w #3,(6,%a5)
bra.s 8f
6:
move.w #5,(6,%a5)
bra.s 8f
1:
move.w #9,(6,%a5)
8:
subq.l #2,%sp
moveq #0x6e,%d0
add.w (6,%a5),%d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
clr.w (6,%a5)
tst.w (%a5)
bne.s 9f
move.w (0xc,%a5),%d0
beq.s 11f
cmpi.w #0x46,%d0
beq.s 12f
bra.s 13f
11:
move.w #1,(6,%a5)
bra.s 13f
12:
move.w #2,(6,%a5)
bra.s 13f
9:
move.w #3,(6,%a5)
13:
subq.l #2,%sp
moveq #0x78,%d0
add.w (6,%a5),%d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w (8,%a5),(6,%a5)
move.w (0xc,%a5),%d0
beq.s 15f
cmpi.w #0x46,%d0
beq.s 16f
bra.s 19f
15:
move.w #1,(6,%a5)
bra.s 14f
16:
move.w #2,(6,%a5)
bra.s 14f
.balignw 4,0x2048
18:
subq.w #1,(6,%a5)
19:
tst.w (6,%a5)
bgt.s 18b
14:
subq.l #2,%sp
move.w (6,%a5),%d0
addi.w #0x82,%d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w (8,%a5),(6,%a5)
move.w (0xc,%a5),%d0
beq.s 22f
cmpi.w #0x46,%d0
beq.s 25f
bra.s 21f
22:
move.w #1,(6,%a5)
bra.s 21f
.balignw 4,0x2048
24:
subq.w #1,(6,%a5)
25:
tst.w (6,%a5)
bgt.s 24b
21:
subq.l #2,%sp
move.w (6,%a5),%d0
addi.w #0x8c,%d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl CASE_RETURN
CASE_RETURN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w #0x95,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w #0x96,(6,%a5)
move.w (0xc,%a5),%d0
beq.s 5f
cmpi.w #7,%d0
beq.s 3f
cmpi.w #0x64,%d0
beq.s 4f
bra.s 1f
3:
move.w #0x97,(6,%a5)
bra.s 1f
4:
move.w #0x98,(6,%a5)
1:
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
5:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl IFS
IFS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
cmpi.w #1,(%a5)
bne.s 1f
cmpi.w #1,(4,%a5)
bne.s 2f
cmpi.w #1,(8,%a5)
bne.s 3f
move.w #1,(6,%a5)
bra.s 6f
3:
move.w #2,(6,%a5)
bra.s 6f
2:
move.w #3,(6,%a5)
bra.s 6f
1:
move.w #4,(6,%a5)
6:
subq.l #2,%sp
move.w (6,%a5),%d0
addi.w #0xa0,%d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w #0xaa,(6,%a5)
cmpi.w #1,(%a5)
bne.s 7f
cmpi.w #1,(4,%a5)
beq.s 9f
cmpi.w #1,(8,%a5)
bne.s 10f
9:
move.w #0xab,(6,%a5)
bra.s 10f
7:
move.w #0xac,(6,%a5)
10:
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w #0xaf,(6,%a5)
cmpi.w #1,(%a5)
bne.s 11f
cmpi.w #1,(4,%a5)
bne.s 13f
cmpi.w #1,(8,%a5)
bne.s 13f
move.w #0xb0,(6,%a5)
bra.s 13f
11:
move.w #0xb1,(6,%a5)
13:
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl IF_RETURN
IF_RETURN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w #0xb3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w #0xb4,(6,%a5)
cmpi.w #1,(%a5)
bne.s 1f
cmpi.w #1,(4,%a5)
beq.s 2f
move.w #0xb5,(6,%a5)
bra.s 3f
1:
move.w #0xb6,(6,%a5)
3:
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl UNPOPPED
UNPOPPED:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
cmpi.w #1,(%a5)
bne.s 1f
move.w #0xbe,(6,%a5)
move.w (4,%a5),%d0
cmp.w (8,%a5),%d0
beq.s 3f
subq.l #2,%sp
move.w #0xbf,-(%sp)
bra.s 4f
1:
subq.l #2,%sp
move.w #0xc0,-(%sp)
4:
jsr (EXEC_$MARK).l
3:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl LOOP_ELSE_WHILE
LOOP_ELSE_WHILE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w #2,(2,%a5)
clr.w (6,%a5)
bra.s 2f
.balignw 4,0x2048
1:
subq.w #1,(2,%a5)
cmpi.w #1,(4,%a5)
bne.s 7f
addq.w #1,(6,%a5)
bra.s 5f
.balignw 4,0x2048
6:
addq.w #1,(6,%a5)
7:
cmpi.w #5,(6,%a5)
blt.s 6b
5:
2:
tst.w (2,%a5)
bgt.s 1b
subq.l #2,%sp
move.w (6,%a5),%d0
addi.w #0xc8,%d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl WHILE_WHILE
WHILE_WHILE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%a5),%d0
addq.w #1,%d0
move.w %d0,(2,%a5)
clr.w (6,%a5)
bra.s 2f
.balignw 4,0x2048
4:
subq.w #1,(2,%a5)
addq.w #1,(6,%a5)
5:
tst.w (2,%a5)
bgt.s 4b
2:
tst.w (2,%a5)
bgt.s 5b
subq.l #2,%sp
move.w (6,%a5),%d0
addi.w #0xd2,%d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
INNER:
link.w %fp,#-4
movea.l (%fp),%a0
cmpi.w #1,(%a5)
bne.s 5f
cmpi.w #1,(4,%a5)
bne.s 3f
move.w #0xdd,(-2,%a0)
bra.s 3f
.balignw 4,0x2048
4:
subq.w #1,(-2,%a0)
5:
cmpi.w #0xdc,(-2,%a0)
bgt.s 4b
3:
unlk %fp
rts
.globl OUTER
OUTER:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w #0xdf,(-2,%fp)
bsr.s INNER
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl THREADING_PATHS_$MAIN
THREADING_PATHS_$MAIN:
link.w %fp,#0
movem.l %a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),(%a5)
movea.l (0xc,%fp),%a1
move.w (%a1),(4,%a5)
movea.l (0x10,%fp),%a2
move.w (%a2),(8,%a5)
movea.l (0x14,%fp),%a3
move.w (%a3),(0xc,%a5)
bsr.w NEXT_INNER_ELSE
bsr.w EXIT_INNER_ELSE
bsr.w RETURN_INNER_ELSE
bsr.w GOTO_INNER_ELSE
bsr.w EXIT_ELSE_WHILE
bsr.w ELSE_WHILES
bsr.w GOTO_ELSE_WHILE
bsr.w BACK_GOTO
bsr.w FRES
move.w %d0,(0xa,%a5)
subq.l #2,%sp
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
bsr.w CASES
bsr.w CASE_RETURN
bsr.w IFS
bsr.w IF_RETURN
bsr.w UNPOPPED
bsr.w LOOP_ELSE_WHILE
bsr.w WHILE_WHILE
bsr.w OUTER
movem.l (-0xc,%fp),%a2-%a3/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.THREADING_PATHS","aw",@nobits
.balign 4
.Ldata:
.space 16
