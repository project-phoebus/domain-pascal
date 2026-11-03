.text
.globl BE_SETS_$SIGN
BE_SETS_$SIGN:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,%d2
add.w %d2,%d2
clr.w (-2,%a5,%d2.w)
cmpi.w #-0x8000,%d1
bcs.s 1f
move.w #-0x8000,(-2,%a5,%d2.w)
1:
move.w (-2,%a5,%d2.w),%d0
btst #15,%d0
beq.s 2f
move.w #1,(6,%a5)
bra.s 3f
2:
clr.w (6,%a5)
3:
pea (6,%a5)
jsr (EXEC_$LOG_W).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl BE_SETS_$HELD
BE_SETS_$HELD:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.w (6,%a5)
btst #7,%d0
beq.s 1f
move.w #1,(6,%a5)
1:
btst #5,%d0
beq.s 2f
move.w #2,(6,%a5)
2:
pea (6,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BE_SETS_$CONST
BE_SETS_$CONST:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.l #0x1202,%d1
btst %d0,%d1
beq.s 1f
move.w #1,(6,%a5)
bra.s 2f
1:
clr.w (6,%a5)
2:
pea (6,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_SETS","aw",@nobits
.balign 4
.Ldata:
.space 8
