.text
.globl WITH_UNIT
WITH_UNIT:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #0x2c,%d0
movea.l (8,%fp),%a0
movea.l (0x10,%fp),%a1
mulu.w (%a0),%d0
lea (0,%a5,%d0.w),%a0
move.w (-2,%a0),(%a1)
movea.l (0xc,%fp),%a2
move.w (%a2),(-2,%a0)
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl BY_UNIT_VAR
BY_UNIT_VAR:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
moveq #0x2c,%d0
mulu.w (0xce0,%a5),%d0
move.w (-0x2c,%a5,%d0.w),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl BY_VALUE
BY_VALUE:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
moveq #0x64,%d1
mulu.w %d0,%d1
lea (0,%a5,%d1.w),%a0
move.l (0x160,%a0),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl OTHER_UNIT
OTHER_UNIT:
link.w %fp,#-4
moveq #0x5c,%d0
movea.l (8,%fp),%a0
movea.l #RT,%a1
mulu.w (%a0),%d0
move.l (0,%a1,%d0.w),%d0
unlk %fp
rts
.globl WORD_PRODUCT
WORD_PRODUCT:
link.w %fp,#-4
move.w (8,%fp),%d0
moveq #0x2c,%d1
muls.w %d0,%d1
addi.w #0x34,%d1
move.w %d1,%d0
unlk %fp
rts
.globl LONG_PRODUCT
LONG_PRODUCT:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
moveq #0x2c,%d0
muls.w (0xce0,%a5),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl BY_268
BY_268:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
mulu.w #0x10c,%d1
lea (0,%a5,%d1.w),%a0
move.w (0x480,%a0),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.IM","aw",@nobits
.balign 4
.Ldata:
.space 3300
