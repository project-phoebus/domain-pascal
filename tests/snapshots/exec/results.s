.text
.globl RESULTS_$MAIN
RESULTS_$MAIN:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
pea (8,%fp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w %d0,(%a5)
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.l #0x11170,(4,%a5)
pea (4,%a5)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
move.l %d0,(4,%a5)
move.w (%a5),%d0
ext.l %d0
add.l (4,%a5),%d0
move.l %d0,(4,%a5)
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
pea (8,%fp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w %d0,%d1
ext.l %d1
move.l %d1,(4,%a5)
addq.l #1,(4,%a5)
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
st (0x12,%a5)
pea (0x12,%a5)
jsr (EXEC_$ID_BOOL).l
addq.w #4,%sp
move.b %d0,(8,%a5)
pea (8,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
clr.b (0x12,%a5)
pea (0x12,%a5)
jsr (EXEC_$ID_BOOL).l
addq.w #4,%sp
move.b %d0,(8,%a5)
pea (8,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.w #0x2a,(0x10,%a5)
lea (0x10,%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
move.l %a0,(0xc,%a5)
movea.l (0xc,%a5),%a0
move.w (%a0),(%a5)
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl RESULTS_$WRAP
RESULTS_$WRAP:
link.w %fp,#-4
pea (8,%fp)
jsr (EXEC_$ID_W).l
unlk %fp
rts
.globl POSITIVE
POSITIVE:
link.w %fp,#-4
tst.w (8,%fp)
sgt %d0
unlk %fp
rts
.globl RESULTS_$BOOL
RESULTS_$BOOL:
link.w %fp,#-4
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.s POSITIVE
unlk %fp
rts
.globl RESULTS_$HELD
RESULTS_$HELD:
link.w %fp,#-4
move.l %d2,-(%sp)
pea (8,%fp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w %d0,%d2
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
move.w %d2,%d0
move.l (-8,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.RESULTS","aw",@nobits
.balign 4
.Ldata:
.space 20
