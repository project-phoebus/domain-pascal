.set __dpc_fragment_origin,0xe47098
.text
.globl ACL_$IN_SUBSYS
ACL_$IN_SUBSYS:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (PROC1_$CURRENT).l,%d0
movea.l #ACL_$DATA+60000,%a0
add.w %d0,%d0
lea (0,%a0,%d0.w),%a1
tst.w (-0x3d5a,%a1)
sgt %d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.section ".bss.moddata.ACL_","aw",@nobits
.balign 4
.Ldata:
.space 3068
.comm ACL_$DATA,44440,4
