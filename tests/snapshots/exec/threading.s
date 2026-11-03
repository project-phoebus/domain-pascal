.text
.globl NESTED_ELSE
NESTED_ELSE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
cmpi.w #1,(%a5)
bne.s 1f
cmpi.w #1,(4,%a5)
bne.s 2f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 5f
2:
subq.l #2,%sp
move.w #2,-(%sp)
bra.s 5f
1:
subq.l #2,%sp
move.w #3,-(%sp)
5:
jsr (EXEC_$MARK).l
4:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl INNER_NO_ELSE
INNER_NO_ELSE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w #0xa,(6,%a5)
cmpi.w #1,(%a5)
bne.s 1f
cmpi.w #1,(4,%a5)
bne.s 2f
move.w #0xb,(6,%a5)
2:
cmpi.w #0xa,(6,%a5)
bne.s 4f
move.w #0xc,(6,%a5)
bra.s 4f
1:
move.w #0xd,(6,%a5)
4:
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl RETURN_AFTER
RETURN_AFTER:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w #0x14,(6,%a5)
cmpi.w #1,(%a5)
bne.s 1f
cmpi.w #1,(4,%a5)
bne.s 3f
move.w #0x15,(6,%a5)
bra.s 3f
1:
move.w #0x16,(6,%a5)
3:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ELSE_WHILE
ELSE_WHILE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (4,%a5),(2,%a5)
tst.w (%a5)
bne.s 4f
clr.w (2,%a5)
bra.s 2f
.balignw 4,0x2048
3:
subq.w #1,(2,%a5)
4:
tst.w (2,%a5)
bgt.s 3b
2:
cmpi.w #1,(%a5)
bne.s 6f
cmpi.w #1,(4,%a5)
bne.s 7f
move.w #0x20,(2,%a5)
bra.s 10f
7:
moveq #0x1f,%d0
sub.w (2,%a5),%d0
move.w %d0,(6,%a5)
bra.s 9f
6:
move.w #0x1e,(2,%a5)
10:
move.w (2,%a5),(6,%a5)
9:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl GOTO_JOIN
GOTO_JOIN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w #0x28,(2,%a5)
tst.w (%a5)
beq.s 3f
cmpi.w #1,(4,%a5)
beq.s 2f
move.w #0x29,(2,%a5)
bra.s 3f
2:
move.w #0x28,(2,%a5)
3:
move.w (2,%a5),(6,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl THREADING_$MAIN
THREADING_$MAIN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),(%a5)
movea.l (0xc,%fp),%a1
move.w (%a1),(4,%a5)
bsr.w NESTED_ELSE
bsr.w INNER_NO_ELSE
bsr.w RETURN_AFTER
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
bsr.w ELSE_WHILE
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
bsr.w GOTO_JOIN
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.THREADING","aw",@nobits
.balign 4
.Ldata:
.space 8
