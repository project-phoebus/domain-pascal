.text
.globl DEC_FLAGS_$ELSE
DEC_FLAGS_$ELSE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),(%a5)
subq.w #1,(%a5)
bpl.s 1f
clr.w (%a5)
bra.s 2f
1:
bne.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 5f
3:
subq.l #2,%sp
move.w #2,-(%sp)
5:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
4:
2:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl DEC_FLAGS_$JOIN
DEC_FLAGS_$JOIN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),(%a5)
subq.w #1,(%a5)
bpl.s 1f
clr.w (%a5)
1:
bne.s 2f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 4f
2:
subq.l #2,%sp
move.w #2,-(%sp)
4:
jsr (EXEC_$LOGV_W).l
3:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl DEC_FLAGS_$FIELD
DEC_FLAGS_$FIELD:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
lea (8,%a5),%a0
move.l %a0,(0x10,%a5)
movea.l %a0,%a2
movea.l (8,%fp),%a1
move.w (%a1),(4,%a2)
subq.w #1,(4,%a2)
bpl.s 1f
clr.w (4,%a2)
bra.s 2f
1:
bne.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 5f
3:
subq.l #2,%sp
move.w #2,-(%sp)
5:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
4:
2:
subq.l #2,%sp
move.w (4,%a2),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl DEC_FLAGS_$ADDL
DEC_FLAGS_$ADDL:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0),(4,%a5)
addq.l #3,(4,%a5)
bmi.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl DEC_FLAGS_$TSTGT
DEC_FLAGS_$TSTGT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),(%a5)
tst.w (%a5)
ble.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$LOGV_W).l
bra.s 2f
1:
bne.s 3f
subq.l #2,%sp
move.w #2,-(%sp)
bra.s 5f
3:
subq.l #2,%sp
move.w #3,-(%sp)
5:
jsr (EXEC_$LOGV_W).l
4:
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl DEC_FLAGS_$TSTEQ
DEC_FLAGS_$TSTEQ:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),(%a5)
tst.w (%a5)
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$LOGV_W).l
bra.s 2f
1:
ble.s 3f
subq.l #2,%sp
move.w #2,-(%sp)
bra.s 5f
3:
subq.l #2,%sp
move.w #3,-(%sp)
5:
jsr (EXEC_$LOGV_W).l
4:
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.DEC_FLAGS","aw",@nobits
.balign 4
.Ldata:
.space 20
