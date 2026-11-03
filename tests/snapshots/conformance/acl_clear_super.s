.set __dpc_fragment_origin,0xe46ff8
.text
.globl ACL_$CLEAR_SUPER
ACL_$CLEAR_SUPER:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (PROC1_$CURRENT).l,%d0
add.w %d0,%d0
lea (0,%a5,%d0.w),%a0
clr.w (0xb76,%a0)
tst.b (0xbf8,%a5)
bpl.s 1f
move.w (PROC1_$CURRENT).l,%d0
cmp.w (0xb72,%a5),%d0
bne.s 1f
move.l #ACL_$EXCLUSION_LOCK,-(%sp)
jsr (ML_$EXCLUSION_STOP).l
clr.b (0xbf8,%a5)
1:
movea.l (-8,%fp),%a5
unlk %fp
rts
.section ".bss.moddata.ACL_","aw",@nobits
.balign 4
.Ldata:
.space 3068
