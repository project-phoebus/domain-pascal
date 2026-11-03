.text
.globl BE_CMP_$CONST
BE_CMP_$CONST:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,(%a5)
move.w %d1,(4,%a5)
moveq #2,%d0
move.w (%a5),%d1
move.w (4,%a5),%d2
ext.l %d1
ext.l %d2
add.l %d2,%d1
cmp.l %d1,%d0
bge.s 1f
move.w #1,(2,%a5)
bra.s 2f
1:
clr.w (2,%a5)
2:
pea (2,%a5)
jsr (EXEC_$LOG_W).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl BE_CMP_$CALL_RHS
BE_CMP_$CALL_RHS:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,(%a5)
move.w %d1,(4,%a5)
move.w (%a5),%d2
pea (4,%a5)
addq.w #1,%d2
jsr (EXEC_$ID_W).l
addq.w #4,%sp
cmp.w %d0,%d2
bne.s 1f
move.w #1,(2,%a5)
bra.s 2f
1:
clr.w (2,%a5)
2:
pea (2,%a5)
jsr (EXEC_$LOG_W).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl BE_CMP_$CALL_LHS
BE_CMP_$CALL_LHS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,(%a5)
move.w %d1,(4,%a5)
pea (%a5)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w (4,%a5),%d1
addq.w #1,%d1
cmp.w %d1,%d0
bne.s 1f
move.w #1,(2,%a5)
bra.s 2f
1:
clr.w (2,%a5)
2:
pea (2,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BE_CMP_$AND_GT
BE_CMP_$AND_GT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
moveq #7,%d1
and.w %d0,%d1
tst.w %d1
ble.s 1f
move.w #1,(2,%a5)
bra.s 2f
1:
clr.w (2,%a5)
2:
pea (2,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BE_CMP_$MUL_GT
BE_CMP_$MUL_GT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
ext.l %d1
move.l %d1,%d0
add.l %d0,%d0
add.l %d0,%d1
tst.l %d1
ble.s 1f
move.w #1,(2,%a5)
bra.s 2f
1:
clr.w (2,%a5)
2:
pea (2,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_CMP","aw",@nobits
.balign 4
.Ldata:
.space 8
