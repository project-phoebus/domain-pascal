.text
WORD_ELEM:
link.w %fp,#-8
move.w (I).l,%d0
add.w %d0,%d0
move.w (-2,%a5,%d0.w),%d0
unlk %fp
rts
LONG_ELEM:
link.w %fp,#-8
move.w (I).l,%d0
lsl.w #2,%d0
lea (0,%a5,%d0.w),%a0
move.l (0x80,%a0),%d0
unlk %fp
rts
REC_ELEM:
link.w %fp,#-8
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #5,%d1
move.w %d1,%d0
add.w %d0,%d0
add.w %d0,%d1
move.w (0x64,%a5,%d1.w),%d0
unlk %fp
rts
CONST_ELEM:
link.w %fp,#-8
move.w (4,%a5),%d0
unlk %fp
rts
LONG_INDEX:
link.w %fp,#-8
move.l (J).l,%d0
lsl.l #2,%d0
lea (0,%a5,%d0.l),%a0
move.l (0x80,%a0),%d0
unlk %fp
rts
FRAME_ELEM:
link.w %fp,#-0x10
move.w (I).l,%d0
add.w %d0,%d0
move.w (-0xe,%fp,%d0.w),%d0
unlk %fp
rts
STORES:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,(W).l
move.w %d0,%d1
lsl.w #2,%d1
lea (0,%a5,%d1.w),%a0
clr.l (0x80,%a0)
unlk %fp
rts
BY_ADDRESS:
link.w %fp,#-4
move.w (I).l,%d0
add.w %d0,%d0
pea (-2,%a5,%d0.w)
jsr (P).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.INDEX","aw",@nobits
.balign 4
.Ldata:
.space 576
