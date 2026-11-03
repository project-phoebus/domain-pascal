.text
.globl LSP_$INQ
LSP_$INQ:
link.w %fp,#-4
movem.l %a2-%a3,-(%sp)
movea.l (0x10,%fp),%a2
pea (%a2)
pea (8,%fp)
jsr (LSP_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
movea.l %d0,%a0
movea.l (0xc,%fp),%a3
lea (4,%a0),%a1
move.l (%a1)+,(%a3)
move.l (%a1)+,(4,%a3)
1:
movem.l (-0xc,%fp),%a2-%a3
unlk %fp
rts
.globl LSP_$RUN
LSP_$RUN:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
pea (-4,%fp)
pea (%a5)
move.l (8,%fp),-(%sp)
bsr.s LSP_$INQ
lea (0xc,%sp),%sp
move.l (-4,%fp),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl LSP_$AFTER_TEST
LSP_$AFTER_TEST:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(8,%a5)
movea.l (0xa,%fp),%a0
move.w #1,(%a0)
movea.l (0xe,%fp),%a1
move.w #2,(%a1)
cmpi.w #3,(8,%a5)
bne.s 1f
movea.l (0x12,%fp),%a2
clr.w (%a2)
1:
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.LSP","aw",@progbits
.balign 4
.Ldata:
.byte 0x00,0x00,0x00,0x07,0x00,0x00,0x00,0x08
.space 4
