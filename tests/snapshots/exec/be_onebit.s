.text
.globl BE_ONEBIT_$BIT15
BE_ONEBIT_$BIT15:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(%a5)
clr.l %d0
move.w (%a5),%d0
btst #15,%d0
beq.s 1f
move.w #1,(8,%a5)
bra.s 2f
1:
clr.w (8,%a5)
2:
pea (8,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BE_ONEBIT_$BIT7
BE_ONEBIT_$BIT7:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(4,%a5)
move.w (4,%a5),%d0
btst #7,%d0
beq.s 1f
move.w #1,(8,%a5)
bra.s 2f
1:
clr.w (8,%a5)
2:
pea (8,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BE_ONEBIT_$CALL
BE_ONEBIT_$CALL:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(4,%a5)
pea (4,%a5)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
btst #2,%d0
beq.s 1f
move.w #1,(8,%a5)
bra.s 2f
1:
clr.w (8,%a5)
2:
pea (8,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BE_ONEBIT_$BYTE
BE_ONEBIT_$BYTE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.b (8,%fp),%d0
move.b %d0,(2,%a5)
clr.w %d0
move.b (2,%a5),%d0
btst #2,%d0
beq.s 1f
move.w #1,(8,%a5)
bra.s 2f
1:
clr.w (8,%a5)
2:
pea (8,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BE_ONEBIT_$VALUE
BE_ONEBIT_$VALUE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(4,%a5)
move.w (4,%a5),%d0
btst #2,%d0
sne %d0
move.b %d0,(6,%a5)
pea (6,%a5)
jsr (EXEC_$LOG_BOOL).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_ONEBIT","aw",@nobits
.balign 4
.Ldata:
.space 12
