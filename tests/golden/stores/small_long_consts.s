.text
.globl FRAME_LOCAL
FRAME_LOCAL:
link.w %fp,#-4
moveq #0x12,%d0
move.l %d0,(-4,%fp)
pea (-4,%fp)
jsr (USE_L).l
unlk %fp
rts
.globl OTHER_UNIT
OTHER_UNIT:
link.w %fp,#0
moveq #0x64,%d0
move.l %d0,(MMAP_X).l
unlk %fp
rts
.globl SAME_CONSTANT
SAME_CONSTANT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
moveq #1,%d0
move.l %d0,(%a5)
move.l %d0,(4,%a5)
move.l %d0,(8,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl RUN_APART
RUN_APART:
link.w %fp,#0
move.l %d2,-(%sp)
movea.l (8,%fp),%a0
clr.w (8,%a0)
moveq #0x29,%d0
move.l %d0,(0x14,%a0)
moveq #2,%d1
move.l %d1,(0xc,%a0)
moveq #0x23,%d2
move.l %d2,(0x1c,%a0)
clr.l (0x18,%a0)
clr.l (0x10,%a0)
move.l (-4,%fp),%d2
unlk %fp
rts
.globl BELOW_A_WORD
BELOW_A_WORD:
link.w %fp,#0
movea.l (8,%fp),%a0
move.l #1,(%a0)
clr.w (4,%a0)
unlk %fp
rts
.globl BELOW_A_LONG
BELOW_A_LONG:
link.w %fp,#0
movea.l (8,%fp),%a0
move.l #7,(%a0)
clr.l (4,%a0)
unlk %fp
rts
.globl BELOW_A_WORD_FIRST
BELOW_A_WORD_FIRST:
link.w %fp,#0
movea.l (8,%fp),%a0
move.w #1,(4,%a0)
moveq #1,%d0
move.l %d0,(%a0)
unlk %fp
rts
.globl DEVICE_LONG
DEVICE_LONG:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l #5,(0x10,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SMALLK","aw",@nobits
.balign 4
.Ldata:
.space 20
