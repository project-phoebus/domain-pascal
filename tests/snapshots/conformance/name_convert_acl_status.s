.set __dpc_fragment_origin,0xe5861c
.text
.globl NAME_CONVERT_ACL_STATUS
NAME_CONVERT_ACL_STATUS:
link.w %fp,#0
movea.l (8,%fp),%a0
cmpi.l #0x230001,(%a0)
bne.s 1f
move.l #0xe0013,(%a0)
bra.s 2f
1:
cmpi.l #0x230002,(%a0)
bne.s 3f
move.l #0xe0014,(%a0)
bra.s 4f
3:
cmpi.l #0x230004,(%a0)
bne.s 5f
move.l #0xe000e,(%a0)
5:
4:
2:
unlk %fp
rts
