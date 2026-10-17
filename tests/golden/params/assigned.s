.text
.globl PA_$ROUND
PA_$ROUND:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
addi.l #0x7fff,%d0
andi.l #-0x8000,%d0
move.l %d0,(%a5)
move.w #1,(4,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PA_$PAGES
PA_$PAGES:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
addi.w #0x3ff,%d0
andi.w #-0x400,%d0
move.w %d0,(4,%a5)
.balignw 4,0x2048
1:
clr.l (%a5)
subi.w #0x400,%d0
bne.s 1b
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PA_$PROT
PA_$PROT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
ori.w #2,%d0
move.w %d1,(4,%a5)
move.w %d0,(4,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PA_$CLAMP
PA_$CLAMP:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
tst.w %d0
bpl.s 1f
clr.w %d0
1:
cmpi.w #0x11,%d1
bls.s 2f
moveq #0x11,%d1
2:
move.w %d0,(4,%a5)
move.w %d1,(4,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PA_$STRIP
PA_$STRIP:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.b (8,%fp),%d0
move.b (0xa,%fp),%d1
bpl.s 1f
andi.b #0x7f,%d0
1:
move.b %d0,(6,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PA_$COMPRESS
PA_$COMPRESS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
bra.s 2f
.balignw 4,0x2048
1:
lsr.l #3,%d0
2:
cmpi.l #0x2000,%d0
bge.s 1b
addq.l #1,%d0
move.l %d0,(%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PA_$TAKE
PA_$TAKE:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w (0xc,%fp),%d2
sub.w %d1,%d0
cmp.w %d2,%d0
ble.s 1f
move.w %d2,%d0
1:
move.w %d0,(4,%a5)
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
SHOW:
link.w %fp,#-4
movea.l (%fp),%a0
move.l (8,%a0),(%a5)
unlk %fp
rts
.globl PA_$RESIZE
PA_$RESIZE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
addi.l #0x7fff,%d0
andi.l #-0x8000,%d0
move.l %d0,(8,%fp)
bsr.s SHOW
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PA_$PUSHED
PA_$PUSHED:
link.w %fp,#0
move.b (0xc,%fp),%d0
bpl.s 1f
clr.l (8,%fp)
1:
move.l (8,%fp),-(%sp)
jsr (PA_$LOG).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PA","aw",@nobits
.balign 4
.Ldata:
.space 8
