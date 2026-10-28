.arch 68020
.text
.globl FIM_$GET_FIM_ADDR
FIM_$GET_FIM_ADDR:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (PROC1_$AS_ID).l,%d0
move.l (0x3c,%a5,%d0.w*4),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl PROC1_$GET_TYPE
PROC1_$GET_TYPE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
movea.l (0xa,%fp),%a0
move.w (0xc42,%a5,%d0.w*2),(%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl NAME_$GET_NDIR_UID
NAME_$GET_NDIR_UID:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (PROC1_$AS_ID).l,%d0
movea.l (8,%fp),%a1
lea (0x3e0,%a5,%d0.w*8),%a0
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BAT_$IS_BUSY
BAT_$IS_BUSY:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.b (0xdca,%a5,%d0.w),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SCALED_INDEX","aw",@nobits
.balign 4
.Ldata:
.space 3564
