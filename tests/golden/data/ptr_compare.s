.text
.globl P_$SET
P_$SET:
link.w %fp,#-4
movem.l %a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
pea (%a2)
jsr (FIND_PCB).l
addq.w #4,%sp
movea.l %a0,%a3
cmpa.l (%a5),%a3
bne.s 1f
pea (%a2)
jsr (STOP_TIMER).l
1:
clr.l (%a3)
movem.l (-0x10,%fp),%a2-%a3/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PTR_COMPARE","aw",@nobits
.balign 4
.Ldata:
.space 4
