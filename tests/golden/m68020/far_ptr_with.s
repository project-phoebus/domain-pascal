.arch 68020
.text
.globl MARK_WITH
MARK_WITH:
link.w %fp,#-4
move.l (8,%fp),%d0
movea.l (0xc,%fp),%a0
move.l %d0,%d1
lsl.l #4,%d1
lea (0,%a0,%d1.l),%a0
movea.l %a0,%a1
adda.l #-0x100000,%a1
bset #6,(%a1)
unlk %fp
rts
.balignw 4,0x2048
