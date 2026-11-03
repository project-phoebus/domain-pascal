.text
.globl BH_F_$BOOL
BH_F_$BOOL:
link.w %fp,#-4
pea (8,%fp)
jsr (EXEC_$ID_BOOL).l
addq.w #4,%sp
move.b %d0,%d1
bpl.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
pea (8,%fp)
jsr (EXEC_$ID_BOOL).l
addq.w #4,%sp
move.b %d0,%d1
bmi.s 2f
subq.l #2,%sp
move.w #2,-(%sp)
jsr (EXEC_$MARK).l
2:
unlk %fp
rts
.globl BH_F_$CALLW
BH_F_$CALLW:
link.w %fp,#-4
move.l %d2,-(%sp)
pea (8,%fp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w %d0,%d2
ble.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
pea (8,%fp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w %d0,%d2
bgt.s 2f
subq.l #2,%sp
move.w #2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
move.l (-8,%fp),%d2
unlk %fp
rts
.globl BH_F_$LOADW
BH_F_$LOADW:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(%a5)
jsr (EXEC_$CLOBBER).l
move.w (%a5),%d2
bmi.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w (%a5),%d2
bgt.s 2f
subq.l #2,%sp
move.w #2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BH_F_$DEREF
BH_F_$DEREF:
link.w %fp,#-0xc
movem.l %d2/%a2,-(%sp)
pea (-8,%fp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (-8,%fp)
bne.s 1f
clr.l %d2
movea.l %d0,%a2
.balignw 4,0x2048
2:
add.l (%a2),%d2
cmpi.l #0xa,%d2
ble.s 2b
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (4,%a2),-(%sp)
jsr (EXEC_$LOGV_L).l
1:
movem.l (-0x14,%fp),%d2/%a2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_H4BA_FLOW","aw",@nobits
.balign 4
.Ldata:
.space 4
