.text
.globl BE_ANDOR_$VALUE
BE_ANDOR_$VALUE:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,(4,%a5)
move.w %d1,(%a5)
tst.w (4,%a5)
sgt %d2
pea (%a5)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
cmpi.w #3,%d0
sgt %d0
and.b %d0,%d2
move.b %d2,(6,%a5)
pea (6,%a5)
jsr (EXEC_$LOG_BOOL).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl BE_ANDOR_$STATIC
BE_ANDOR_$STATIC:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,(4,%a5)
move.w %d1,(%a5)
clr.w (2,%a5)
tst.w %d0
ble.s 1f
pea (4,%a5)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
cmp.w (%a5),%d0
seq %d2
tst.b %d2
bpl.s 1f
move.w #1,(2,%a5)
1:
pea (2,%a5)
jsr (EXEC_$LOG_W).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_ANDOR","aw",@nobits
.balign 4
.Ldata:
.space 8
