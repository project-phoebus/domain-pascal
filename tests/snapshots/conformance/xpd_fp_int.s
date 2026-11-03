.set __dpc_fragment_origin,0xe5c55a
.text
.space 2
.globl XPD_$GET_FP_INT
XPD_$GET_FP_INT:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
movea.l (0xc,%fp),%a0
clr.l (%a0)
tst.b (PEB_$INFO+0x20).l
bpl.s 1f
subq.l #2,%sp
move.w (%a2),-(%sp)
jsr (FP_$GET_FP).l
bra.s 2f
1:
pea (%a2)
jsr (PEB_$GET_FP).l
2:
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl XPD_$PUT_FP_INT
XPD_$PUT_FP_INT:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
movea.l (0xc,%fp),%a0
clr.l (%a0)
tst.b (PEB_$INFO+0x20).l
bpl.s 1f
subq.l #2,%sp
move.w (%a2),-(%sp)
jsr (FP_$PUT_FP).l
bra.s 2f
1:
pea (%a2)
jsr (PEB_$PUT_FP).l
2:
movea.l (-4,%fp),%a2
unlk %fp
rts
.balignw 4,0x2048
