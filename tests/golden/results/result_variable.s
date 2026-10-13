.text
.globl NEXT_ID
NEXT_ID:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
pea (%a5)
jsr (ML_$SPIN_LOCK).l
addq.w #4,%sp
move.w %d0,(-2,%fp)
move.w (4,%a5),%d2
addq.w #1,(4,%a5)
cmpi.w #-0x600,(4,%a5)
bls.s 1f
move.w #1,(4,%a5)
1:
subq.l #2,%sp
move.w %d0,-(%sp)
pea (%a5)
jsr (ML_$SPIN_UNLOCK).l
move.w %d2,%d0
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl RTNVA
RTNVA:
link.w %fp,#-8
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
pea (%a5)
jsr (ML_$SPIN_LOCK).l
addq.w #4,%sp
move.w %d0,(-2,%fp)
move.l (8,%fp),-(%sp)
jsr (WORKER).l
addq.w #4,%sp
move.l %d0,%d2
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
pea (%a5)
jsr (ML_$SPIN_UNLOCK).l
move.l %d2,%d0
movem.l (-0x10,%fp),%d2/%a5
unlk %fp
rts
.globl CHK_IO
CHK_IO:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d1
move.l (0xc,%fp),%d0
tst.b (0xc,%a5)
bmi.s 1f
clr.w %d0
bra.s 2f
1:
cmp.l (8,%a5),%d1
bne.s 3f
moveq #1,%d0
bra.s 4f
3:
cmp.l (8,%a5),%d0
bne.s 5f
moveq #2,%d0
bra.s 6f
5:
clr.w %d0
6:
4:
2:
clr.b (0xc,%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl EARLY
EARLY:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d1
st %d0
cmp.w (0xe,%a5),%d1
beq.s 1f
move.w %d1,(0x10,%a5)
clr.b %d0
1:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl IN_IF
IN_IF:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
cmpi.w #3,%d0
ble.s 1f
move.w %d0,%d1
add.w %d1,%d1
move.w (0xc,%a5,%d1.w),%d2
1:
jsr (TICK).l
move.w %d2,%d0
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl SECOND_CELL
SECOND_CELL:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (0x28,%a5),(-4,%fp)
movea.l (-4,%fp),%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.RESULTS","aw",@nobits
.balign 4
.Ldata:
.space 52
