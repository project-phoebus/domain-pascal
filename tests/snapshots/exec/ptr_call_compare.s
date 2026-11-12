.text
.globl PICK
PICK:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
bne.s 1f
lea (%a5),%a0
bra.s 2f
1:
lea (2,%a5),%a0
2:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl ONE
ONE:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (0xa,%fp),%d2
clr.w (-2,%fp)
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.s PICK
addq.w #4,%sp
cmpa.l %d2,%a0
bne.s 1f
move.w #1,(-2,%fp)
1:
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
clr.w (-2,%fp)
move.l (4,%a5),%d3
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.s PICK
addq.w #4,%sp
move.l %a0,%d0
cmp.l %d0,%d3
beq.s 2f
move.w #1,(-2,%fp)
2:
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.w PICK
addq.w #4,%sp
cmpa.l (4,%a5),%a0
seq %d0
move.b %d0,(8,%a5)
tst.b (8,%a5)
bpl.s 3f
pea (.Lc0,%pc)
bra.s 7f
3:
pea (.Lc1,%pc)
7:
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
4:
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.w PICK
addq.w #4,%sp
move.l %a0,%d0
cmp.l %d0,%d2
sne %d0
move.b %d0,(8,%a5)
tst.b (8,%a5)
bpl.s 5f
pea (.Lc0,%pc)
bra.s 8f
5:
pea (.Lc1,%pc)
8:
jsr (EXEC_$LOG_W).l
6:
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.Lc0:
.short 0x0001
.Lc1:
.short 0x0000
.globl PTR_CALL_COMPARE_$RUN
PTR_CALL_COMPARE_$RUN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
lea (%a5),%a0
move.l %a0,(4,%a5)
subq.l #2,%sp
pea (%a5)
move.w (8,%fp),-(%sp)
bsr.w ONE
addq.w #8,%sp
subq.l #2,%sp
pea (%a5)
move.w (0xa,%fp),-(%sp)
bsr.w ONE
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PTR_CALL_COMPARE","aw",@nobits
.balign 4
.Ldata:
.space 12
