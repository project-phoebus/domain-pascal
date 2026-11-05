.text
.globl PICK
PICK:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
ble.s 1f
move.l (%a5),(-4,%fp)
bra.s 3f
1:
move.l (4,%a5),(-4,%fp)
3:
move.l (-4,%fp),%d0
2:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl EARLY
EARLY:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d1
move.l (8,%a5),(-4,%fp)
move.l (-4,%fp),%d0
tst.w %d1
bmi.s 1f
move.l (4,%a5),(-4,%fp)
move.l (-4,%fp),%d0
1:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl SAME
SAME:
link.w %fp,#-4
move.l (8,%fp),%d0
move.l %d0,(-4,%fp)
unlk %fp
rts
.globl BE_L_RECORDS_$MAIN
BE_L_RECORDS_$MAIN:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w #1,(%a5)
move.w #2,(2,%a5)
move.w #3,(4,%a5)
move.w #4,(6,%a5)
move.w #5,(8,%a5)
move.w #6,(0xa,%a5)
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.w PICK
addq.w #4,%sp
move.l %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.w EARLY
addq.w #4,%sp
move.l %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.w #7,(-4,%fp)
move.w #8,(-2,%fp)
move.l (-4,%fp),-(%sp)
bsr.w SAME
addq.w #4,%sp
move.l %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_L_RECORDS","aw",@nobits
.balign 4
.Ldata:
.space 12
