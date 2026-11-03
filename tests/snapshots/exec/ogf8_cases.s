.text
.globl OGF8_CASES_$ONE
OGF8_CASES_$ONE:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.l (0xa,%fp),%d3
move.w %d2,%d0
cmpi.w #1,%d0
bne.s 3f
move.w #0xa,(%a5)
bra.s 1f
3:
move.w #0x14,(%a5)
1:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w #3,(%a5)
move.w %d2,%d0
cmpi.w #2,%d0
bne.s 4f
move.w #-1,(%a5)
4:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w %d2,%d0
bne.s 6f
move.w #1,(%a5)
6:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.l %d3,%d0
bne.s 10f
clr.w (%a5)
bra.s 8f
10:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
8:
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl OGF8_CASES_$CONST
OGF8_CASES_$CONST:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.w %d0
cmpi.w #4,%d0
bcc.s 1f
add.w %d0,%d0
move.w (6f,%pc,%d0.w),%d0
jmp (6f,%pc,%d0.w)
6:
.short 2f-6b
.short 3f-6b
.short 4f-6b
.short 5f-6b
2:
move.w #3,(%a5)
bra.s 1f
3:
move.w #4,(%a5)
bra.s 1f
4:
move.w #5,(%a5)
bra.s 1f
5:
move.w #9,(%a5)
1:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
moveq #2,%d0
tst.w %d0
beq.s 8f
cmpi.w #1,%d0
beq.s 9f
cmpi.w #2,%d0
beq.s 10f
bra.s 7f
8:
move.w #1,(%a5)
bra.s 7f
9:
move.w #2,(%a5)
bra.s 7f
10:
move.w #6,(%a5)
7:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
moveq #2,%d0
tst.w %d0
beq.s 12f
cmpi.w #1,%d0
beq.s 13f
bra.s 14f
12:
move.w #1,(%a5)
bra.s 11f
13:
move.w #2,(%a5)
bra.s 11f
14:
move.w #7,(%a5)
11:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
clr.w %d0
tst.w %d0
bne.s 15f
move.w #2,(%a5)
15:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl OGF8_CASES_$RET
OGF8_CASES_$RET:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w #9,(%a5)
move.w %d0,%d1
cmpi.w #5,%d1
bcc.s 1f
add.w %d1,%d1
move.w (7f,%pc,%d1.w),%d1
jmp (7f,%pc,%d1.w)
7:
.short 2f-7b
.short 3f-7b
.short 4f-7b
.short 5f-7b
.short 6f-7b
2:
clr.w (%a5)
bra.s 1f
3:
bra.s 8f
4:
move.w #2,(%a5)
bra.s 1f
5:
move.w #3,(%a5)
bra.s 1f
6:
move.w #4,(%a5)
1:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
8:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl OGF8_CASES_$DBF
OGF8_CASES_$DBF:
link.w %fp,#-8
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,%d2
move.w %d1,%d3
moveq #4,%d0
subq.w #4,%d3
.balignw 4,0x2048
2:
bra.s 5f
.balignw 4,0x2048
4:
subq.w #1,%d2
5:
tst.w %d2
bgt.s 4b
addq.w #1,%d3
dbf %d0,2b
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0x10,%fp),%d2-%d3
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.OGF8_CASES","aw",@nobits
.balign 4
.Ldata:
.space 4
