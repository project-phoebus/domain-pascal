.text
ADD_Q:
link.w %fp,#-8
move.w (8,%fp),%d0
addq.w #2,%d0
unlk %fp
rts
ADD_MOVEQ:
link.w %fp,#-8
moveq #0x1e,%d0
add.w (8,%fp),%d0
unlk %fp
rts
ADD_I:
link.w %fp,#-8
move.l (8,%fp),%d0
addi.l #0x1000,%d0
unlk %fp
rts
SUB_Q:
link.w %fp,#-8
move.w (8,%fp),%d0
subq.w #1,%d0
unlk %fp
rts
SUB_I:
link.w %fp,#-8
move.w (8,%fp),%d0
subi.w #0x400,%d0
unlk %fp
rts
RSUB:
link.w %fp,#-8
moveq #0x1f,%d0
sub.w (8,%fp),%d0
unlk %fp
rts
TIMES6:
link.w %fp,#-8
move.w (8,%fp),%d0
add.w %d0,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
addq.w #2,%d0
unlk %fp
rts
TIMES12:
link.w %fp,#-8
move.w (8,%fp),%d0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
unlk %fp
rts
TIMES28:
link.w %fp,#-8
move.w (8,%fp),%d0
lsl.w #2,%d0
move.w %d0,%d1
neg.w %d0
lsl.w #3,%d1
add.w %d1,%d0
unlk %fp
rts
TIMES564:
link.w %fp,#-8
move.w (8,%fp),%d0
mulu.w #0x234,%d0
unlk %fp
rts
QUOT:
link.w %fp,#-8
move.w (8,%fp),%d0
ext.l %d0
divs.w #0x1f,%d0
unlk %fp
rts
REMAINDER:
link.w %fp,#-8
move.w (8,%fp),%d0
ext.l %d0
divs.w #0x1f,%d0
swap %d0
unlk %fp
rts
NEGATE:
link.w %fp,#-8
move.w (8,%fp),%d0
neg.w %d0
unlk %fp
rts
COMPLEMENT:
link.w %fp,#-8
move.l (8,%fp),%d0
not.l %d0
unlk %fp
rts
WIDEN:
link.w %fp,#-8
move.w (8,%fp),%d0
ext.l %d0
add.l (L).l,%d0
unlk %fp
rts
WIDEN_UNSIGNED:
link.w %fp,#-8
clr.l %d0
move.w (U).l,%d0
addi.l #0x400,%d0
unlk %fp
rts
UPDATE:
link.w %fp,#-4
addq.w #1,(W).l
subi.l #0x100,(L).l
andi.w #0xff,(W).l
ori.w #-0x8000,(U).l
unlk %fp
rts
.balignw 4,0x2048
