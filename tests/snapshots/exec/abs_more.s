.text
.globl ABS_MORE_$NARROW_U
ABS_MORE_$NARROW_U:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d1
move.w (0xa,%fp),%d0
move.w %d1,%d2
clr.l %d1
ext.l %d2
move.w %d0,%d1
add.l %d1,%d2
bpl.s 1f
neg.l %d2
1:
move.w %d2,(2,%a5)
move.w %d2,%d0
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl ABS_MORE_$NARROW_W
ABS_MORE_$NARROW_W:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d1
move.w (0xa,%fp),%d0
move.w %d1,%d2
clr.l %d1
ext.l %d2
move.w %d0,%d1
add.l %d1,%d2
bpl.s 1f
neg.l %d2
1:
move.w %d2,(%a5)
move.w %d2,%d0
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl ABS_MORE_$MULC
ABS_MORE_$MULC:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
muls.w #0x3e8,%d1
bpl.s 1f
neg.w %d1
1:
move.w %d1,(2,%a5)
move.w %d1,%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl ABS_MORE_$MULC2
ABS_MORE_$MULC2:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
muls.w #0x2af8,%d1
bpl.s 1f
neg.w %d1
1:
subi.w #0x7d0,%d1
move.w %d1,(%a5)
move.w %d1,%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl ABS_MORE_$CASE
ABS_MORE_$CASE:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,%d1
bpl.s 1f
neg.w %d1
1:
cmpi.w #6,%d1
bcc.s 8f
add.w %d1,%d1
move.w (9f,%pc,%d1.w),%d1
jmp (9f,%pc,%d1.w)
9:
.short 3f-9b
.short 4f-9b
.short 5f-9b
.short 6f-9b
.short 8f-9b
.short 7f-9b
3:
clr.w %d0
bra.s 2f
4:
moveq #0xa,%d0
bra.s 2f
5:
moveq #0x14,%d0
bra.s 2f
6:
moveq #0x1e,%d0
bra.s 2f
7:
moveq #0x32,%d0
bra.s 2f
8:
moveq #0x63,%d0
2:
unlk %fp
rts
.globl ABS_MORE_$MAX
ABS_MORE_$MAX:
link.w %fp,#-4
move.w (8,%fp),%d0
bpl.s 1f
neg.w %d0
1:
move.w (0xa,%fp),%d1
bpl.s 2f
neg.w %d1
2:
cmp.w %d1,%d0
bcc.s 3f
move.w %d1,%d0
3:
unlk %fp
rts
.globl ABS_MORE_$POS
ABS_MORE_$POS:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,%d1
bpl.s 2f
neg.w %d1
2:
tst.w %d1
beq.s 1f
moveq #1,%d0
bra.s 3f
1:
clr.w %d0
3:
unlk %fp
rts
.globl ABS_MORE_$INSET
ABS_MORE_$INSET:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,%d1
bpl.s 2f
neg.w %d1
2:
moveq #0x2a,%d0
btst %d1,%d0
beq.s 1f
moveq #1,%d0
bra.s 3f
1:
clr.w %d0
3:
unlk %fp
rts
.globl ABS_MORE_$FUNC
ABS_MORE_$FUNC:
link.w %fp,#-4
pea (8,%fp)
jsr (EXEC_$ID_W).l
move.w %d0,%d1
bpl.s 1f
neg.w %d1
1:
move.w %d1,%d0
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ABS_MORE","aw",@nobits
.balign 4
.Ldata:
.space 4
