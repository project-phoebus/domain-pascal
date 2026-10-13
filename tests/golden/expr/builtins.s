.text
.globl CONSTS
CONSTS:
link.w %fp,#0
move.w #8,(W).l
move.w #0x10,(W).l
move.w #0xa,(W).l
move.w #8,(W).l
move.l #4,(L).l
move.w #2,(C).l
move.w #1,(C).l
move.w #1,(C).l
move.w #-7,(W).l
move.w #7,(W).l
move.w #2,(W).l
move.w #9,(W).l
move.w #0x7fff,(W).l
move.l #0x7fffffff,(L).l
move.b #0xff,(CH).l
clr.b (B).l
st (B).l
move.w #9,(W).l
move.w #-3,(W).l
st (B).l
unlk %fp
rts
.globl COMPUTED
COMPUTED:
link.w %fp,#0
addq.w #1,(W).l
subq.l #1,(L).l
unlk %fp
rts
.globl EN
EN:
link.w %fp,#0
addq.w #1,(C).l
unlk %fp
rts
.balignw 4,0x2048
