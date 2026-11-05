.text
.globl CALL_RESULT
CALL_RESULT:
link.w %fp,#-4
subq.l #2,%sp
move.w (8,%fp),-(%sp)
jsr (GETL).l
addq.w #4,%sp
move.l %d0,(-4,%fp)
beq.s 1f
move.l %d0,-(%sp)
jsr (USEL).l
addq.w #4,%sp
pea (-4,%fp)
jsr (TAKEL).l
1:
unlk %fp
rts
.globl BOOLEAN_FLAG
BOOLEAN_FLAG:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),-(%sp)
jsr (READY).l
addq.w #4,%sp
move.b %d0,(-2,%fp)
bpl.s 1f
move.w #1,(%a5)
1:
subq.l #2,%sp
move.b %d0,-(%sp)
jsr (USB).l
addq.w #4,%sp
pea (-2,%fp)
jsr (TAKEB).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl NEGATIVE
NEGATIVE:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),-(%sp)
jsr (GETW).l
addq.w #4,%sp
move.w %d0,(-2,%fp)
bpl.s 1f
clr.w (%a5)
1:
subq.l #2,%sp
move.w %d0,-(%sp)
jsr (USE).l
addq.w #4,%sp
pea (-2,%fp)
jsr (TAKE).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl AFTER_JOIN
AFTER_JOIN:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
clr.w (-2,%fp)
cmpi.w #3,(%a5)
ble.s 1f
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (GETW).l
addq.w #4,%sp
move.w %d0,(-2,%fp)
1:
tst.w (-2,%fp)
beq.s 2f
move.w #2,(%a5)
2:
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (USE).l
addq.w #4,%sp
pea (-2,%fp)
jsr (TAKE).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl UNIT_DATA
UNIT_DATA:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),-(%sp)
jsr (GETW).l
addq.w #4,%sp
move.w %d0,(2,%a5)
tst.w (2,%a5)
bne.s 1f
move.w #3,(%a5)
1:
subq.l #2,%sp
move.w %d0,-(%sp)
jsr (USE).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.STFLAGS","aw",@nobits
.balign 4
.Ldata:
.space 4
