.text
.globl ONE
ONE:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0),%d2
cmpi.l #0x5f,%d2
bhi.s 1f
moveq #0x5f,%d0
lea (%a5),%a0
sub.w %d2,%d0
lsr.w #3,%d0
btst %d2,(0,%a0,%d0.w)
beq.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 7f
1:
subq.l #2,%sp
clr.w -(%sp)
7:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
2:
moveq #0x20,%d0
add.l %d2,%d0
cmpi.l #0x5f,%d0
bhi.s 3f
moveq #0x20,%d0
moveq #0x5f,%d1
add.l %d2,%d0
lea (%a5),%a0
sub.w %d0,%d1
lsr.w #3,%d1
btst %d0,(0,%a0,%d1.w)
beq.s 3f
st %d0
bra.s 4f
3:
clr.b %d0
4:
move.b %d0,(0xc,%a5)
tst.b (0xc,%a5)
bpl.s 5f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 8f
5:
subq.l #2,%sp
clr.w -(%sp)
8:
jsr (EXEC_$LOGV_W).l
6:
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl SET96_$RUN
SET96_$RUN:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
clr.l (%a5)
clr.l (4,%a5)
clr.l (8,%a5)
moveq #2,%d0
move.w %d0,%d1
moveq #0x5f,%d0
addq.w #3,%d1
lea (%a5),%a0
sub.w %d1,%d0
lsr.w #3,%d0
bset %d1,(0,%a0,%d0.w)
moveq #0x5f,%d0
moveq #0x5f,%d1
lea (%a5),%a0
sub.w %d0,%d1
lsr.w #3,%d1
bset %d0,(0,%a0,%d1.w)
bset #4,(%a5)
pea (8,%fp)
bsr.w ONE
addq.w #4,%sp
pea (0xc,%fp)
bsr.w ONE
addq.w #4,%sp
pea (0x10,%fp)
bsr.w ONE
addq.w #4,%sp
pea (0x14,%fp)
bsr.w ONE
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SET96","aw",@nobits
.balign 4
.Ldata:
.space 16
