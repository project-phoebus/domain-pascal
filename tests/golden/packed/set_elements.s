.text
.globl OWNED
OWNED:
link.w %fp,#-4
move.l (8,%fp),%d0
movea.l #SCSI_$PPN_OWNERS,%a0
lea (0,%a0,%d0.l),%a0
clr.w %d0
adda.l #-0x10000,%a0
move.b (%a0),%d0
tst.w %d0
sne %d0
unlk %fp
rts
.globl CLEAR_BUSY
CLEAR_BUSY:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.b (0,%a5,%d0.w)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl COUNT_BUSY
COUNT_BUSY:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #0xf,%d0
clr.w %d1
.balignw 4,0x2048
2:
clr.w %d2
move.b (0,%a5,%d1.w),%d2
tst.w %d2
beq.s 4f
addq.w #1,(0x10,%a5)
4:
addq.w #1,%d1
dbf %d0,2b
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SET_ELEMENTS","aw",@nobits
.balign 4
.Ldata:
.space 20
