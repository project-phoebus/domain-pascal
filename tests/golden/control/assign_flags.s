.text
.globl LOAD_EQ
LOAD_EQ:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0xc,%a5),%a0
move.l (%a0),%d0
bne.s 1f
move.w #1,(%a5)
1:
move.l %d0,(4,%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl LOAD_SIGN
LOAD_SIGN:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0x10,%a5),%a0
move.w (%a0),%d0
bpl.s 1f
move.w #2,(%a5)
1:
move.w %d0,(2,%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl LOAD_POSITIVE
LOAD_POSITIVE:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0x10,%a5),%a0
move.w (%a0),%d0
ble.s 1f
move.w #3,(%a5)
1:
move.w %d0,(2,%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl COPY_OR
COPY_OR:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.b (8,%fp),%d0
move.b (0xa,%fp),%d1
move.b %d0,%d2
or.b %d1,%d2
move.b %d2,%d0
bpl.s 1f
move.w #4,(%a5)
1:
move.b %d0,(8,%a5)
move.b %d1,%d0
move.b %d0,(8,%a5)
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl CALL_RESULT
CALL_RESULT:
link.w %fp,#-4
move.l %d2,-(%sp)
subq.l #2,%sp
move.w (8,%fp),-(%sp)
jsr (GETW).l
addq.w #4,%sp
move.w %d0,%d2
beq.s 1f
subq.l #2,%sp
move.w (8,%fp),-(%sp)
jsr (USE).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (USE).l
1:
move.l (-8,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ASFLAGS","aw",@nobits
.balign 4
.Ldata:
.space 20
