.text
.globl LAST_ONLY
LAST_ONLY:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
pea (-4,%fp)
jsr (SCAN).l
clr.b (0x20,%a5)
movea.l #0,%a0
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl FIRST_NIL
FIRST_NIL:
link.w %fp,#-8
move.w (8,%fp),%d0
suba.l %a0,%a0
moveq #7,%d1
movea.l #DEVICES,%a1
.balignw 4,0x2048
2:
cmp.w (2,%a1),%d0
bne.s 4f
movea.l (4,%a1),%a0
bra.s 5f
4:
addq.l #8,%a1
dbf %d1,2b
5:
unlk %fp
rts
.globl LOCKED
LOCKED:
link.w %fp,#-4
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),%d2
move.w #6,-(%sp)
jsr (ML_$LOCK).l
addq.w #4,%sp
move.w %d2,%d0
lsl.w #2,%d0
tst.l (0,%a5,%d0.w)
bne.s 1f
move.w #1,(0x22,%a5)
suba.l %a2,%a2
bra.s 2f
1:
movea.l (0,%a5,%d0.w),%a2
2:
subq.l #2,%sp
move.w #6,-(%sp)
jsr (ML_$UNLOCK).l
movea.l %a2,%a0
movem.l (-0x10,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl RETURNED
RETURNED:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
beq.s 1f
move.w %d0,(0x22,%a5)
move.w %d0,%d1
lsl.w #2,%d1
movea.l (0,%a5,%d1.w),%a0
bra.s 2f
1:
clr.w (0x22,%a5)
suba.l %a0,%a0
2:
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.NILRES","aw",@nobits
.balign 4
.Ldata:
.space 36
