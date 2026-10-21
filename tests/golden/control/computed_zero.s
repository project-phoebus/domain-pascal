.text
.globl FLAGS
FLAGS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (%a5),%d0
or.w (2,%a5),%d0
bne.s 1f
clr.w (2,%a5)
1:
move.w (%a5),%d0
sub.w (2,%a5),%d0
bpl.s 2f
move.w #1,(2,%a5)
2:
move.w (%a5),%d0
sub.w (2,%a5),%d0
tst.w %d0
ble.s 3f
move.w #2,(2,%a5)
3:
move.l #0x180,%d0
and.l (4,%a5),%d0
beq.s 4f
move.w #3,(2,%a5)
4:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl TESTED
TESTED:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
subq.l #2,%sp
movea.l (8,%fp),%a2
move.w (%a5),-(%sp)
jsr (GETW).l
addq.w #4,%sp
tst.w %d0
bne.s 1f
clr.w (2,%a5)
1:
move.w (%a5),%d0
ext.l %d0
divs.w #0x3c,%d0
tst.w %d0
bne.s 2f
move.w #1,(2,%a5)
2:
move.w (%a5),%d0
ext.l %d0
divs.w #0x3c,%d0
swap %d0
tst.w %d0
beq.s 3f
move.w #2,(2,%a5)
3:
clr.w %d0
move.b (2,%a2),%d0
tst.w %d0
bne.s 4f
move.w #3,(2,%a5)
4:
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl IS_ZERO
IS_ZERO:
link.w %fp,#-4
subq.l #2,%sp
move.w (8,%fp),-(%sp)
jsr (GETW).l
tst.w %d0
seq %d0
unlk %fp
rts
.globl BIT2
BIT2:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d1
btst #2,%d1
sne %d0
move.l %d1,(4,%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl ASSIGNED
ASSIGNED:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w #1,-(%sp)
jsr (GETW).l
addq.w #4,%sp
move.w %d0,%d1
sub.w (%a5),%d1
bne.s 1f
clr.w (2,%a5)
1:
sub.w (%a5),%d1
bpl.s 2f
addi.w #0x100,%d1
2:
add.w (%a5),%d1
tst.w %d1
bgt.s 3f
move.w #2,(2,%a5)
3:
move.w %d1,(2,%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.CZERO","aw",@nobits
.balign 4
.Ldata:
.space 8
