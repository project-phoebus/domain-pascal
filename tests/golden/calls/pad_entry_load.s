.text
.globl IO_$DEVICE_INTERRUPTING
IO_$DEVICE_INTERRUPTING:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),%d2
move.l (0xa,%fp),-(%sp)
move.w %d2,-(%sp)
jsr (IO_$CHECK_UNIT).l
addq.w #8,%sp
tst.b %d0
bpl.s 1f
move.w %d2,%d0
add.w %d0,%d0
tst.w (0,%a5,%d0.w)
sne %d0
1:
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl PMAP_$PURGE_WS
PMAP_$PURGE_WS:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
subq.l #2,%sp
move.b (0xa,%fp),%d3
move.w #0x14,-(%sp)
jsr (ML_$LOCK).l
addq.w #4,%sp
tst.b %d3
bpl.s 1f
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (MMAP_$PURGE).l
addq.w #4,%sp
bra.s 2f
1:
move.w %d2,%d0
add.w %d0,%d0
clr.w (0,%a5,%d0.w)
2:
subq.l #2,%sp
move.w #0x14,-(%sp)
jsr (ML_$UNLOCK).l
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl TTY_$K_FLUSH_INPUT
TTY_$K_FLUSH_INPUT:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a2
subq.l #2,%sp
pea (%a2)
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (TTY_$I_FLUSH_INPUT).l
addq.w #8,%sp
tst.l (%a2)
bne.s 1f
clr.w (0x10,%a5)
1:
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl OLD_DIR_LOOKUP
OLD_DIR_LOOKUP:
link.w %fp,#-8
pea (%a2)
movea.l (0x10,%fp),%a2
subq.l #2,%sp
pea (-8,%fp)
pea (-4,%fp)
move.w (0xc,%fp),-(%sp)
move.l (8,%fp),-(%sp)
jsr (DIR_$FIND).l
lea (0x10,%sp),%sp
tst.b %d0
bpl.s 1f
clr.l (%a2)
bra.s 2f
1:
move.l (-4,%fp),%d0
add.l (-8,%fp),%d0
move.l %d0,(%a2)
2:
movea.l (-0xc,%fp),%a2
unlk %fp
rts
FLUSH_BATCH:
link.w %fp,#-4
pea (%a2)
subq.l #2,%sp
movea.l (%fp),%a2
move.w #0x14,-(%sp)
jsr (ML_$LOCK).l
addq.w #4,%sp
addq.w #1,(-2,%a2)
move.w (-2,%a2),(0x10,%a5)
subq.l #2,%sp
move.w #0x14,-(%sp)
jsr (ML_$UNLOCK).l
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl PMAP_$FLUSH
PMAP_$FLUSH:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(-2,%fp)
bsr.s FLUSH_BATCH
bsr.s FLUSH_BATCH
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PAD_ENTRY_LOAD","aw",@nobits
.balign 4
.Ldata:
.space 20
