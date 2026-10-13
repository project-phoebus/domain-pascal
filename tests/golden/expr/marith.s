.text
.globl GET_ADJUST
GET_ADJUST:
link.w %fp,#0
movea.l (8,%fp),%a1
move.l #0x3d090,-(%sp)
move.l (DELTA).l,-(%sp)
jsr (M$DIS$LLL).l
addq.w #8,%sp
move.l %d0,(%a1)
move.l #0x3d090,-(%sp)
move.l (DELTA).l,-(%sp)
jsr (M$OIS$LLL).l
lsl.l #2,%d0
move.l %d0,(4,%a1)
unlk %fp
rts
HELD:
link.w %fp,#-4
move.l (8,%fp),%d0
move.w (0xc,%fp),%d1
subq.l #2,%sp
move.w %d1,-(%sp)
move.l %d0,-(%sp)
jsr (M$DIS$LLW).l
addq.w #8,%sp
subq.l #2,%sp
move.w %d1,-(%sp)
move.l %d0,-(%sp)
jsr (M$MIS$LLW).l
move.l %d0,(L).l
move.w %d1,(W).l
unlk %fp
rts
LOAD_TERM:
link.w %fp,#-8
move.w (0xc,%fp),%d0
muls.w #0x1478,%d0
add.l (8,%fp),%d0
unlk %fp
rts
TICKS:
link.w %fp,#-8
move.w (8,%fp),%d0
mulu.w #0xfa,%d0
unlk %fp
rts
NARROWED:
link.w %fp,#-4
move.l (L).l,%d0
divs.w #0xfa,%d0
move.w %d0,(W).l
subq.l #2,%sp
move.w #0x20,-(%sp)
move.l (L).l,-(%sp)
jsr (M$OIS$WLW).l
move.w %d0,(W).l
unlk %fp
rts
PER_256:
link.w %fp,#-8
move.l (L).l,%d0
bpl.s 1f
addi.l #0xff,%d0
1:
asr.l #8,%d0
unlk %fp
rts
QUARTER:
link.w %fp,#-8
move.w (8,%fp),%d0
bpl.s 1f
addq.w #3,%d0
1:
asr.w #2,%d0
unlk %fp
rts
HASH:
link.w %fp,#-8
clr.l %d0
move.w (U).l,%d0
divu.w #0x6f,%d0
swap %d0
unlk %fp
rts
.balignw 4,0x2048
