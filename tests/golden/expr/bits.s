.text
MASK:
link.w %fp,#-4
moveq #-2,%d0
and.w (8,%fp),%d0
unlk %fp
rts
HELD:
link.w %fp,#0
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d1,(W).l
moveq #-2,%d1
and.w %d0,%d1
move.w %d1,(R).l
unlk %fp
rts
MASK_OR:
link.w %fp,#-4
move.w #0xff,%d0
and.w (8,%fp),%d0
ori.w #0x100,%d0
unlk %fp
rts
MASK_XOR:
link.w %fp,#-4
move.w (8,%fp),%d0
lsl.w #2,%d0
eori.w #0x55,%d0
unlk %fp
rts
SHL10:
link.w %fp,#-4
move.l (8,%fp),%d0
lsl.l #8,%d0
lsl.l #2,%d0
unlk %fp
rts
SHR3:
link.w %fp,#-4
move.l (8,%fp),%d0
lsr.l #3,%d0
unlk %fp
rts
SAR2:
link.w %fp,#-4
move.w (8,%fp),%d0
asr.w #2,%d0
unlk %fp
rts
SHL_VAR:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w (N).l,%d1
lsl.w %d1,%d0
unlk %fp
rts
.balignw 4,0x2048
