.text
.globl MIN_MAX_$MIN_SW
MIN_MAX_$MIN_SW:
link.w %fp,#-4
move.w (8,%fp),%d0
cmp.w (0xa,%fp),%d0
ble.s 1f
move.w (0xa,%fp),%d0
1:
unlk %fp
rts
.globl MIN_MAX_$MAX_SW
MIN_MAX_$MAX_SW:
link.w %fp,#-4
move.w (8,%fp),%d0
cmp.w (0xa,%fp),%d0
bge.s 1f
move.w (0xa,%fp),%d0
1:
unlk %fp
rts
.globl MIN_MAX_$MIN_UW
MIN_MAX_$MIN_UW:
link.w %fp,#-4
move.w (8,%fp),%d0
cmp.w (0xa,%fp),%d0
bls.s 1f
move.w (0xa,%fp),%d0
1:
unlk %fp
rts
.globl MIN_MAX_$MIN_UL
MIN_MAX_$MIN_UL:
link.w %fp,#-4
clr.l %d0
move.w (8,%fp),%d0
cmp.l (0xa,%fp),%d0
bls.s 1f
move.l (0xa,%fp),%d0
1:
unlk %fp
rts
.globl MIN_MAX_$MIN_DIFF
MIN_MAX_$MIN_DIFF:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
ext.l %d0
ext.l %d1
move.l #0xb0,%d2
sub.l %d1,%d2
cmp.l %d2,%d0
ble.s 1f
move.l %d2,%d0
1:
move.l (-8,%fp),%d2
unlk %fp
rts
.globl MIN_MAX_$MAX0
MIN_MAX_$MAX0:
link.w %fp,#-4
move.l (8,%fp),%d0
sub.l (0xc,%fp),%d0
bpl.s 1f
clr.l %d0
1:
unlk %fp
rts
.globl MIN_MAX_$MIN_CONST
MIN_MAX_$MIN_CONST:
link.w %fp,#-4
move.w (8,%fp),%d0
cmpi.w #0x400,%d0
ble.s 1f
move.w #0x400,%d0
1:
unlk %fp
rts
.globl MIN_MAX_$MIN_TWO
MIN_MAX_$MIN_TWO:
link.w %fp,#-4
move.w (8,%fp),%d0
cmpi.w #2,%d0
ble.s 1f
moveq #2,%d0
1:
unlk %fp
rts
.globl MIN_MAX_$MAX_MIXED
MIN_MAX_$MAX_MIXED:
link.w %fp,#-4
move.w (8,%fp),%d0
clr.l %d1
ext.l %d0
move.w (0xa,%fp),%d1
cmp.l %d1,%d0
bge.s 1f
move.l %d1,%d0
1:
unlk %fp
rts
.globl MIN_MAX_$MAX_NEG
MIN_MAX_$MAX_NEG:
link.w %fp,#-4
moveq #-1,%d0
clr.l %d1
move.w (8,%fp),%d1
cmp.l %d1,%d0
bge.s 1f
move.l %d1,%d0
1:
unlk %fp
rts
.globl MIN_MAX_$IN_PLACE
MIN_MAX_$IN_PLACE:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,%d2
cmp.w %d1,%d2
bge.s 1f
move.w %d1,%d2
1:
move.w %d2,(4,%a5)
subq.l #2,%sp
move.w (4,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl MIN_MAX_$RANGE0
MIN_MAX_$RANGE0:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(%a5)
cmpi.w #3,(%a5)
bhi.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$MARK).l
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl MIN_MAX_$RANGE1
MIN_MAX_$RANGE1:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(2,%a5)
clr.l %d0
move.w (2,%a5),%d0
subq.l #1,%d0
cmpi.l #2,%d0
bhi.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$MARK).l
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.MIN_MAX","aw",@nobits
.balign 4
.Ldata:
.space 8
