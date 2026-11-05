.text
.globl RECORD_RESULTS_$MAKE
RECORD_RESULTS_$MAKE:
link.w %fp,#-8
move.l (8,%fp),%d0
move.l %d0,(-4,%fp)
move.l %d0,(-8,%fp)
unlk %fp
rts
.globl PASS_ON
PASS_ON:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.l (%a0),(-4,%fp)
move.l (-4,%fp),%d0
unlk %fp
rts
.globl LOG_STATUS
LOG_STATUS:
link.w %fp,#0
move.l (8,%fp),-(%sp)
jsr (EXEC_$LOG_L).l
unlk %fp
rts
.globl RECORD_RESULTS_$FIELDS
RECORD_RESULTS_$FIELDS:
link.w %fp,#-8
move.l (8,%fp),-(%sp)
bsr.s RECORD_RESULTS_$MAKE
addq.w #4,%sp
move.l %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
tst.w (-4,%fp)
bpl.s 1f
moveq #1,%d0
move.l %d0,(-8,%fp)
bra.s 2f
1:
clr.l (-8,%fp)
2:
pea (-8,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
moveq #0x7f,%d0
and.b (-4,%fp),%d0
ext.l %d0
move.l %d0,(-8,%fp)
pea (-8,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.globl RECORD_RESULTS_$CHAIN
RECORD_RESULTS_$CHAIN:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),-(%sp)
bsr.w RECORD_RESULTS_$MAKE
addq.w #4,%sp
move.l %d0,(-4,%fp)
cmpi.l #0x120035,%d0
bne.s 1f
pea (-5).w
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
1:
pea (-4,%fp)
bsr.w PASS_ON
addq.w #4,%sp
move.l %d0,(-8,%fp)
pea (-8,%fp)
bsr.w LOG_STATUS
addq.w #4,%sp
cmpi.w #3,(-6,%fp)
bne.s 2f
pea (5).w
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
2:
pea (-8,%fp)
bsr.w PASS_ON
move.l %d0,(%a5)
addq.l #1,(4,%a5)
movea.l (0xc,%fp),%a0
move.l %d0,(%a0)
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl RECORD_RESULTS_$LAST
RECORD_RESULTS_$LAST:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (%a5)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (4,%a5)
jsr (EXEC_$LOG_L).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.RECORD_RESULTS","aw",@progbits
.balign 4
.Ldata:
.space 8
