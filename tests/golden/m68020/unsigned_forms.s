.arch 68020
.text
.globl Q
Q:
link.w %fp,#0
moveq #7,%d0
and.w (P).l,%d0
move.w %d0,(X).l
move.l (L).l,%d0
lsr.l #8,%d0
lsr.l #2,%d0
move.l %d0,(Y).l
move.l (L).l,%d0
lsl.l #8,%d0
lsl.l #2,%d0
move.l %d0,(Z).l
st (C).l
unlk %fp
rts
.globl R
R:
link.w %fp,#-4
movea.l (8,%fp),%a0
st (%a0)
unlk %fp
rts
.balignw 4,0x2048
