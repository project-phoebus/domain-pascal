.text
.globl FRAME_VAR
FRAME_VAR:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
pea (-4,%fp)
jsr (Q).l
addq.w #4,%sp
move.l (-4,%fp),%d0
cmpi.l #0xf0003,%d0
beq.s 2f
cmpi.l #0xe000d,%d0
bne.s 1f
2:
move.w #1,(4,%a5)
1:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl UNIT_VAR
UNIT_VAR:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (%a5),%d0
cmpi.l #1,%d0
beq.s 1f
cmpi.l #5,%d0
beq.s 1f
cmpi.l #7,%d0
beq.s 1f
move.w #2,(4,%a5)
1:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ZERO_FIRST
ZERO_FIRST:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
pea (-2,%fp)
jsr (GETC).l
addq.w #4,%sp
move.b (-2,%fp),%d0
beq.s 2f
cmpi.b #0xf,%d0
bne.s 1f
2:
move.w #3,(4,%a5)
1:
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ORCHAIN","aw",@nobits
.balign 4
.Ldata:
.space 8
