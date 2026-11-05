.text
.globl PROBE
PROBE:
link.w %fp,#-4
move.l %d2,-(%sp)
subq.l #2,%sp
move.w (8,%fp),%d2
move.w #0x32,-(%sp)
jsr (EXEC_$MARK).l
cmpi.w #1,%d2
sgt %d0
move.l (-8,%fp),%d2
unlk %fp
rts
.globl SHORTCIRCUIT_$MAIN
SHORTCIRCUIT_$MAIN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(%a5)
tst.w (%a5)
beq.s 1f
subq.l #2,%sp
move.w %d0,-(%sp)
bsr.s PROBE
addq.w #4,%sp
tst.b %d0
bpl.s 1f
st (2,%a5)
bra.s 2f
1:
clr.b (2,%a5)
2:
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
tst.w (%a5)
bne.s 4f
subq.l #2,%sp
move.w (%a5),-(%sp)
bsr.s PROBE
addq.w #4,%sp
tst.b %d0
bpl.s 3f
4:
st (2,%a5)
bra.s 5f
3:
clr.b (2,%a5)
5:
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SHORTCIRCUIT","aw",@nobits
.balign 4
.Ldata:
.space 4
