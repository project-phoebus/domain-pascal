.text
.globl SIO_$I_XMIT_DONE
SIO_$I_XMIT_DONE:
link.w %fp,#-4
pea (%a2)
movea.l (8,%fp),%a2
bclr #0,(0x75,%a2)
pea (%a2)
jsr (SIO_$I_TSTART).l
btst #0,(0x75,%a2)
sne %d0
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl SIO_$START_CUR
SIO_$START_CUR:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (%a5),-(%sp)
jsr (SIO_$I_TSTART).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl SIO_$START_NEXT
SIO_$START_NEXT:
link.w %fp,#-4
jsr (SIO_$I_NEXT).l
move.l %a0,(-4,%fp)
move.l (-4,%fp),-(%sp)
jsr (SIO_$I_TSTART).l
addq.w #4,%sp
move.l (-4,%fp),-(%sp)
jsr (SIO_$I_TSTART).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SIO","aw",@nobits
.balign 4
.Ldata:
.space 4
