.text
.globl CASES_$TABLE
CASES_$TABLE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(%a5)
move.w (%a5),%d0
cmpi.w #6,%d0
bcc.s 7f
add.w %d0,%d0
move.w (8f,%pc,%d0.w),%d0
jmp (8f,%pc,%d0.w)
8:
.short 2f-8b
.short 3f-8b
.short 4f-8b
.short 7f-8b
.short 5f-8b
.short 6f-8b
2:
subq.l #2,%sp
move.w #0xa,-(%sp)
bra.s 9f
3:
subq.l #2,%sp
move.w #0xb,-(%sp)
bra.s 9f
4:
subq.l #2,%sp
move.w #0xc,-(%sp)
bra.s 9f
5:
subq.l #2,%sp
move.w #0xe,-(%sp)
bra.s 9f
6:
subq.l #2,%sp
move.w #0xf,-(%sp)
bra.s 9f
7:
subq.l #2,%sp
move.w #0x63,-(%sp)
9:
jsr (EXEC_$MARK).l
1:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl CASES_$CHAIN
CASES_$CHAIN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(%a5)
move.w (%a5),%d0
subq.w #3,%d0
cmpi.w #8,%d0
bcc.s 5f
add.w %d0,%d0
move.w (6f,%pc,%d0.w),%d0
jmp (6f,%pc,%d0.w)
6:
.short 2f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 3f-6b
.short 4f-6b
.short 2f-6b
2:
subq.l #2,%sp
move.w #5,-(%sp)
bra.s 7f
3:
subq.l #2,%sp
move.w #6,-(%sp)
bra.s 7f
4:
subq.l #2,%sp
clr.w -(%sp)
bra.s 7f
5:
subq.l #2,%sp
move.w #7,-(%sp)
7:
jsr (EXEC_$MARK).l
1:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl CASES_$NEG
CASES_$NEG:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(%a5)
move.w (%a5),%d0
addq.w #4,%d0
cmpi.w #4,%d0
bcc.s 6f
add.w %d0,%d0
move.w (7f,%pc,%d0.w),%d0
jmp (7f,%pc,%d0.w)
7:
.short 5f-7b
.short 4f-7b
.short 3f-7b
.short 2f-7b
2:
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 8f
3:
subq.l #2,%sp
move.w #2,-(%sp)
bra.s 8f
4:
subq.l #2,%sp
move.w #3,-(%sp)
bra.s 8f
5:
subq.l #2,%sp
move.w #4,-(%sp)
bra.s 8f
6:
subq.l #2,%sp
clr.w -(%sp)
8:
jsr (EXEC_$MARK).l
1:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl CASES_$CHAR
CASES_$CHAR:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.b (8,%fp),%d0
move.b %d0,(2,%a5)
clr.w %d0
move.b (2,%a5),%d0
subi.w #0x64,%d0
cmpi.w #9,%d0
bcc.s 5f
add.w %d0,%d0
move.w (6f,%pc,%d0.w),%d0
jmp (6f,%pc,%d0.w)
6:
.short 2f-6b
.short 5f-6b
.short 3f-6b
.short 5f-6b
.short 3f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 4f-6b
2:
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 7f
3:
subq.l #2,%sp
move.w #2,-(%sp)
bra.s 7f
4:
subq.l #2,%sp
move.w #3,-(%sp)
bra.s 7f
5:
subq.l #2,%sp
move.w #9,-(%sp)
7:
jsr (EXEC_$MARK).l
1:
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.CASES","aw",@nobits
.balign 4
.Ldata:
.space 4
