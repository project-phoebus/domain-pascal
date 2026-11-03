.text
.globl BE_L_OPS_$SHIFT
BE_L_OPS_$SHIFT:
link.w %fp,#-4
move.w (8,%fp),%d0
lsr.w #8,%d0
andi.w #7,%d0
unlk %fp
rts
.globl BE_L_OPS_$MINMAX
BE_L_OPS_$MINMAX:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.b (8,%fp),%d2
move.b (0xa,%fp),%d3
move.b %d2,%d0
move.b %d3,%d1
ext.w %d0
ext.w %d1
cmp.w %d1,%d0
ble.s 1f
move.w %d1,%d0
1:
move.w %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.b %d2,%d0
move.b %d3,%d1
ext.w %d0
ext.w %d1
cmp.w %d1,%d0
bge.s 2f
move.w %d1,%d0
2:
move.w %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl BE_L_OPS_$UMINMAX
BE_L_OPS_$UMINMAX:
link.w %fp,#-4
move.l %d2,-(%sp)
move.b (8,%fp),%d0
move.b (0xa,%fp),%d1
clr.w %d2
move.b %d0,%d2
clr.w %d0
move.b %d1,%d0
cmp.w %d0,%d2
bls.s 1f
move.w %d0,%d2
1:
move.w %d2,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
move.l (-8,%fp),%d2
unlk %fp
rts
.globl BE_L_OPS_$INDEX
BE_L_OPS_$INDEX:
link.w %fp,#-4
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
move.b (8,%fp),%d0
moveq #6,%d1
moveq #-3,%d2
lea (-6,%a5),%a0
.balignw 4,0x2048
2:
move.w %d2,%d3
add.w %d3,%d3
move.w %d3,%d4
lsl.w #2,%d4
add.w %d4,%d3
move.w %d3,(6,%a0)
addq.w #1,%d2
addq.l #2,%a0
dbf %d1,2b
move.b %d0,%d1
ext.w %d1
add.w %d1,%d1
move.w (6,%a5,%d1.w),%d0
movem.l (-0x14,%fp),%d2-%d4/%a5
unlk %fp
rts
.globl TWO
TWO:
link.w %fp,#0
move.l %d2,-(%sp)
move.l (0xc,%fp),%d2
pea (8,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
tst.l %d2
bne.s 1f
pea (.Lc0,%pc)
bra.s 3f
1:
pea (.Lc1,%pc)
3:
jsr (EXEC_$LOG_L).l
2:
move.l (-4,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0x0000,0x0001
.Lc1:
.short 0x0000,0x0000
.globl BE_L_OPS_$NIL
BE_L_OPS_$NIL:
link.w %fp,#0
clr.l -(%sp)
pea (5).w
bsr.s TWO
unlk %fp
rts
.globl BE_L_OPS_$PACKED
BE_L_OPS_$PACKED:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
andi.b #0xf,(0xe,%a5)
ori.b #0x10,(0xe,%a5)
move.b #9,(0xf,%a5)
andi.b #0xf0,(0xe,%a5)
move.b %d0,%d1
andi.b #0xf,%d1
or.b %d1,(0xe,%a5)
moveq #0xf,%d0
and.b (0xe,%a5),%d0
move.w %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
clr.w %d0
move.b (0xf,%a5),%d0
move.w %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.w #0xf0,%d0
and.b (0xe,%a5),%d0
lsr.w #4,%d0
move.w %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_L_OPS","aw",@nobits
.balign 4
.Ldata:
.space 16
