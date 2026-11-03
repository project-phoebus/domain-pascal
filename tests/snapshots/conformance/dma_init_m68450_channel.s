.set __dpc_fragment_origin,0xe0a328
.text
DMA_$INIT_M68450_CHANNEL:
link.w %fp,#-4
move.w (0xc,%fp),%d0
movea.l (8,%fp),%a0
move.b #0x10,(7,%a0)
movea.l (8,%fp),%a0
move.b #0xff,(%a0)
movea.l (8,%fp),%a0
move.b #0x28,(4,%a0)
movea.l (8,%fp),%a0
move.b #4,(6,%a0)
movea.l (8,%fp),%a0
move.b %d0,(0x2d,%a0)
unlk %fp
rts
.globl DMA_$INIT
DMA_$INIT:
link.w %fp,#0
subq.l #2,%sp
move.w #3,-(%sp)
pea (DMA+0xc0).l
bsr.s DMA_$INIT_M68450_CHANNEL
addq.w #8,%sp
subq.l #2,%sp
move.w #2,-(%sp)
pea (DMA+0x80).l
bsr.s DMA_$INIT_M68450_CHANNEL
addq.w #8,%sp
subq.l #2,%sp
clr.w -(%sp)
move.l #DMA,-(%sp)
bsr.s DMA_$INIT_M68450_CHANNEL
addq.w #8,%sp
subq.l #2,%sp
move.w #1,-(%sp)
pea (DMA+0x40).l
bsr.s DMA_$INIT_M68450_CHANNEL
unlk %fp
rts
