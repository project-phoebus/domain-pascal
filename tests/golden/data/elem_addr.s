.text
.globl FIND
FIND:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d1
clr.w %d0
move.w %d1,%d2
movea.l #SLOTS,%a0
lsl.w #3,%d2
lea (0,%a0,%d2.w),%a0
move.w (-8,%a0),%d1
cmp.w (CUR).l,%d1
beq.s 1f
moveq #1,%d0
bra.s 2f
1:
tst.w (-6,%a0)
bne.s 3f
moveq #2,%d0
3:
2:
move.l (-8,%fp),%d2
unlk %fp
rts
.globl AVAIL
AVAIL:
link.w %fp,#0
movem.l %d2/%a2,-(%sp)
move.l (8,%fp),%d2
movea.l #MMAP,%a0
move.l %d2,%d0
lsl.l #4,%d0
lea (0,%a0,%d0.l),%a2
cmpi.b #0x45,(-0x1ffc,%a2)
bls.s 1f
pea (.Lc0,%pc)
jsr (CRASH).l
addq.w #4,%sp
1:
st -(%sp)
clr.w %d0
move.b (-0x1ffc,%a2),%d0
move.w %d0,-(%sp)
move.l %d2,-(%sp)
pea (-0x2000,%a2)
jsr (ADD_TO_WSL).l
movem.l (-8,%fp),%d2/%a2
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0x0006,0x0004
.balignw 4,0x2048
