.set __dpc_fragment_origin,0xe3bb58
.text
.globl BAT_$GET_BAT_STEP
BAT_$GET_BAT_STEP:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
mulu.w #0x234,%d1
lea (0,%a5,%d1.w),%a0
move.w (-0x220,%a0),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BAT_","aw",@nobits
.balign 4
.Ldata:
.space 3384
