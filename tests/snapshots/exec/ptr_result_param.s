.text
.globl F
F:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
move.l (0xc,%fp),%d1
tst.l %d0
bne.s 1f
movea.l %d1,%a0
bra.s 2f
1:
move.w #1,(%a5)
movea.l %d0,%a0
2:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl PTR_RESULT_PARAM_$RUN
PTR_RESULT_PARAM_$RUN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w #0xa,(2,%a5)
move.w #0x14,(4,%a5)
clr.w (%a5)
subq.l #2,%sp
pea (4,%a5)
clr.l -(%sp)
bsr.s F
addq.w #8,%sp
move.w (%a0),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
pea (4,%a5)
pea (2,%a5)
bsr.s F
addq.w #8,%sp
move.w (%a0),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PTR_RESULT_PARAM","aw",@nobits
.balign 4
.Ldata:
.space 8
