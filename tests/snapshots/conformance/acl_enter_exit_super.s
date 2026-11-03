.set __dpc_fragment_origin,0xe46f90
.text
.globl ACL_$ENTER_SUPER
ACL_$ENTER_SUPER:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (PROC1_$CURRENT).l,%d0
add.w %d0,%d0
lea (0,%a5,%d0.w),%a0
addq.w #1,(0xb76,%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ACL_$EXIT_SUPER
ACL_$EXIT_SUPER:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (PROC1_$CURRENT).l,%d0
add.w %d0,%d0
lea (0,%a5,%d0.w),%a0
tst.w (0xb76,%a0)
bne.s 1f
pea (.Lc0,%pc)
jsr (CRASH_SYSTEM).l
1:
move.w (PROC1_$CURRENT).l,%d0
add.w %d0,%d0
lea (0,%a5,%d0.w),%a0
subq.w #1,(0xb76,%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0x0023,0x0003
.section ".bss.moddata.ACL_","aw",@nobits
.balign 4
.Ldata:
.space 3064
