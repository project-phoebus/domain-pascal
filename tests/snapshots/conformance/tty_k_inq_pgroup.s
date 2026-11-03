.set __dpc_fragment_origin,0xe67942
.text
.space 2
.globl TTY_$K_INQ_PGROUP
TTY_$K_INQ_PGROUP:
link.w %fp,#-8
movem.l %a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0x10,%fp),%a2
subq.l #2,%sp
pea (%a2)
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (TTY_$I_GET_DESC).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
movea.l %d0,%a0
movea.l (0xc,%fp),%a3
lea (0x4c,%a0),%a1
move.l (%a1)+,(%a3)
move.l (%a1)+,(4,%a3)
1:
movem.l (-0x14,%fp),%a2-%a3/%a5
unlk %fp
rts
.section ".bss.moddata.TTY","aw",@nobits
.balign 4
.Ldata:
.space 4
