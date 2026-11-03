.text
.globl PURIFY
PURIFY:
link.w %fp,#0
moveq #0xb,%d0
add.l (COUNT).l,%d0
move.l #0x186a0,-(%sp)
move.l %d0,-(%sp)
jsr (M$DIU$LLL).l
move.l %d0,(Q).l
unlk %fp
rts
.globl TEN_PERCENT
TEN_PERCENT:
link.w %fp,#0
move.l (PAGES).l,%d0
add.l %d0,%d0
move.l %d0,%d1
move.l #0x186a0,-(%sp)
lsl.l #2,%d1
add.l %d1,%d0
move.l %d0,-(%sp)
jsr (M$DIU$LLL).l
move.l %d0,(Q).l
unlk %fp
rts
.globl WORDS
WORDS:
link.w %fp,#0
clr.l %d0
move.w (SIZE).l,%d0
addq.l #3,%d0
lsr.l #2,%d0
move.w %d0,(N).l
unlk %fp
rts
.globl PREVIOUS
PREVIOUS:
link.w %fp,#0
clr.l %d0
move.w (HEAD).l,%d0
move.l #0x186a0,-(%sp)
addi.l #0xff,%d0
move.l %d0,-(%sp)
jsr (M$OIS$LLL).l
move.l %d0,(Q).l
unlk %fp
rts
.balignw 4,0x2048
