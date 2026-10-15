.text
.globl SET_SCALES
SET_SCALES:
link.w %fp,#0
movem.l %a2-%a3,-(%sp)
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
move.w (%a1),(%a0)
movea.l (0x10,%fp),%a2
move.w (%a2),(2,%a0)
movea.l (0x14,%fp),%a3
move.w (%a3),(4,%a0)
movem.l (-8,%fp),%a2-%a3
unlk %fp
rts
.globl SET_MODE
SET_MODE:
link.w %fp,#0
movea.l (8,%fp),%a0
clr.w (%a0)
movea.l (0xc,%fp),%a1
move.w (%a1),(2,%a0)
unlk %fp
rts
.balignw 4,0x2048
