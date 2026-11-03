.set __dpc_fragment_origin,0xe73fe4
.text
.globl FILE_$TRUNCATE_D
FILE_$TRUNCATE_D:
link.w %fp,#-4
movem.l %a2-%a3,-(%sp)
movea.l (8,%fp),%a2
movea.l (0x14,%fp),%a3
subq.l #2,%sp
pea (%a3)
pea (-2,%fp)
clr.l -(%sp)
movea.l (0x10,%fp),%a0
move.l (%a0),-(%sp)
move.w #2,-(%sp)
pea (%a2)
jsr (FILE_$CHECK_PROT).l
lea (0x18,%sp),%sp
tst.l (%a3)
bne.s 1f
subq.l #2,%sp
pea (%a3)
pea (-4,%fp)
clr.w -(%sp)
movea.l (0xc,%fp),%a0
move.l (%a0),-(%sp)
pea (%a2)
jsr (AST_$TRUNCATE).l
bra.s 2f
1:
pea (%a3)
jsr (OS_PROC_SHUTWIRED).l
2:
movem.l (-0xc,%fp),%a2-%a3
unlk %fp
rts
