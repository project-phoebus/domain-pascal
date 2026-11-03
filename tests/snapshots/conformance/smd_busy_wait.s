.set __dpc_fragment_origin,0xe1d89c
.section ".text.SMD.WIRED_PROC","ax",@progbits
.globl SMD_$PUTC
SMD_$PUTC:
rts
.globl SMD_$BUSY_WAIT
SMD_$BUSY_WAIT:
link.w %fp,#0
pea (%a5)
lea (.Ldata.WIRED_DATA).l,%a5
movea.l (0xc,%fp),%a0
clr.l (%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl SMD_$LITES
SMD_$LITES:
link.w %fp,#0
pea (%a5)
lea (.Ldata.WIRED_DATA).l,%a5
move.w (0xa,%fp),-(%sp)
move.w (8,%fp),-(%sp)
jsr (DISP_LITES).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SMD.WIRED_DATA","aw",@nobits
.balign 4
.Ldata.WIRED_DATA:
.space 4
