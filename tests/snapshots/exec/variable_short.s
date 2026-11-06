.text
.globl FIRST
FIRST:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.l (%a0),%d0
unlk %fp
rts
.globl SECOND
SECOND:
link.w %fp,#-4
move.w (8,%fp),%d0
ext.l %d0
unlk %fp
rts
.globl VARS_$BY_ADDRESS
VARS_$BY_ADDRESS:
link.w %fp,#-8
move.l %d2,-(%sp)
pea (.Lc0,%pc)
bsr.s FIRST
addq.w #4,%sp
move.l %d0,%d2
pea (.Lc1,%pc)
pea (.Lc2,%pc)
bsr.s FIRST
move.l %d2,%d1
add.l %d0,%d1
move.l %d1,%d0
move.l (-0xc,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0x0000,0x0003
.Lc2:
.short 0x0000,0x0004
.Lc1:
.short 0x0000,0x0005
.globl VARS_$BY_VALUE
VARS_$BY_VALUE:
link.w %fp,#-8
move.l %d2,-(%sp)
subq.l #2,%sp
move.w #4,-(%sp)
bsr.s SECOND
addq.w #4,%sp
move.l %d0,%d2
move.l #0x50006,-(%sp)
bsr.s SECOND
move.l %d2,%d1
add.l %d0,%d1
move.l %d1,%d0
move.l (-0xc,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
