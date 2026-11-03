.text
.globl TOP_BIT_ZERO_$REL
TOP_BIT_ZERO_$REL:
link.w %fp,#0
move.l %d2,-(%sp)
move.w (8,%fp),%d2
bgt.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
tst.w %d2
ble.s 2f
subq.l #2,%sp
move.w #2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
tst.w %d2
bpl.s 3f
subq.l #2,%sp
move.w #3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
3:
tst.w %d2
bmi.s 4f
subq.l #2,%sp
move.w #4,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
4:
subq.l #2,%sp
move.w #9,-(%sp)
jsr (EXEC_$MARK).l
move.l (-4,%fp),%d2
unlk %fp
rts
.globl TOP_BIT_ZERO_$LE
TOP_BIT_ZERO_$LE:
link.w %fp,#-4
tst.w (8,%fp)
sle %d0
unlk %fp
rts
.globl TOP_BIT_ZERO_$GT
TOP_BIT_ZERO_$GT:
link.w %fp,#-4
tst.w (8,%fp)
sgt %d0
unlk %fp
rts
.globl TOP_BIT_ZERO_$CHECK
TOP_BIT_ZERO_$CHECK:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
tst.w (%a0)
ble.s 2f
cmpi.w #0x4000,(%a0)
bls.s 1f
2:
move.l #0x220003,(%a1)
bra.s 3f
1:
clr.l (%a1)
3:
unlk %fp
rts
.globl TOP_BIT_ZERO_$FIELD
TOP_BIT_ZERO_$FIELD:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(2,%a5)
tst.w (2,%a5)
ble.s 1f
subq.l #2,%sp
move.w #0x16,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
move.w #0x15,-(%sp)
3:
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
subq.l #2,%sp
move.w #0x1d,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.TOP_BIT_ZERO","aw",@nobits
.balign 4
.Ldata:
.space 4
