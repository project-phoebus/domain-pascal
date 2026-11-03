.text
.globl SET_RANGES_$CONS
SET_RANGES_$CONS:
link.w %fp,#0
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w (0xc,%fp),%d2
move.w (0xe,%fp),%d3
move.w %d0,%d4
lea (%a5),%a0
add.w %d1,%d4
moveq #3,%d1
subq.w #1,%d4
.balignw 4,0x2048
1:
clr.l (%a0)+
dbf %d1,1b
move.w %d4,%d1
cmpi.w #0x7f,%d4
bls.s 2f
moveq #0x7f,%d4
move.w %d4,%d1
2:
lea (%a5),%a0
sub.w %d0,%d1
bcs.s 3f
.balignw 4,0x2048
4:
move.w %d4,%d0
lsr.w #3,%d0
neg.w %d0
bset %d4,(0xf,%a0,%d0.w)
subq.w #1,%d4
dbf %d1,4b
3:
move.w %d2,%d0
add.w %d3,%d0
subq.w #1,%d0
move.w %d0,%d1
cmpi.w #0x7f,%d0
bls.s 5f
moveq #0x7f,%d0
move.w %d0,%d1
5:
lea (%a5),%a0
sub.w %d2,%d1
bcs.s 6f
.balignw 4,0x2048
7:
move.w %d0,%d2
lsr.w #3,%d2
neg.w %d2
bset %d0,(0xf,%a0,%d2.w)
subq.w #1,%d0
dbf %d1,7b
6:
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movem.l (-0x10,%fp),%d2-%d4/%a5
unlk %fp
rts
.Lc0:
.short 0x0010
.globl SET_RANGES_$ADD
SET_RANGES_$ADD:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
clr.l (%a5)
clr.l (4,%a5)
clr.l (8,%a5)
move.l #1,(0xc,%a5)
move.w %d1,%d2
move.w %d2,%d1
cmpi.w #0x7f,%d2
bls.s 1f
moveq #0x7f,%d2
move.w %d2,%d1
1:
lea (%a5),%a0
sub.w %d0,%d1
bcs.s 2f
.balignw 4,0x2048
3:
move.w %d2,%d0
lsr.w #3,%d0
neg.w %d0
bset %d2,(0xf,%a0,%d0.w)
subq.w #1,%d2
dbf %d1,3b
2:
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl SET_RANGES_$SUB
SET_RANGES_$SUB:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.l #-1,(%a5)
move.l #-1,(4,%a5)
move.l #-1,(8,%a5)
move.l #-1,(0xc,%a5)
clr.l (0x10,%a5)
clr.l (0x14,%a5)
clr.l (0x18,%a5)
clr.l (0x1c,%a5)
lea (%a5),%a0
lea (0x20,%a5),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.w %d1,%d2
move.w %d2,%d1
cmpi.w #0xff,%d2
bls.s 1f
move.w #0xff,%d2
move.w %d2,%d1
1:
lea (0x10,%a5),%a0
sub.w %d0,%d1
bcs.s 2f
.balignw 4,0x2048
3:
move.w %d2,%d0
lsr.w #3,%d0
neg.w %d0
bclr %d2,(0x1f,%a0,%d0.w)
subq.w #1,%d2
dbf %d1,3b
2:
pea (.Lc1,%pc)
pea (0x10,%a5)
jsr (EXEC_$LOG_MEM).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.Lc1:
.short 0x0020
.globl SET_RANGES_$MIXED
SET_RANGES_$MIXED:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w (0xc,%fp),%d2
clr.l (%a5)
clr.l (4,%a5)
clr.l (8,%a5)
move.l #2,(0xc,%a5)
moveq #0x7f,%d3
lea (%a5),%a0
sub.w %d0,%d3
lsr.w #3,%d3
bset %d0,(0,%a0,%d3.w)
move.w %d2,%d0
move.w %d0,%d2
cmpi.w #0x7f,%d0
bls.s 1f
moveq #0x7f,%d0
move.w %d0,%d2
1:
lea (%a5),%a0
sub.w %d1,%d2
bcs.s 2f
.balignw 4,0x2048
3:
move.w %d0,%d1
lsr.w #3,%d1
neg.w %d1
bset %d0,(0xf,%a0,%d1.w)
subq.w #1,%d0
dbf %d2,3b
2:
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl SET_RANGES_$WIDEN
SET_RANGES_$WIDEN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l #-0x80000000,(%a5)
clr.l (4,%a5)
clr.l (8,%a5)
move.l #0x20,(0xc,%a5)
move.l #-1,(0x10,%a5)
move.l #-1,(0x14,%a5)
move.l #-1,(0x18,%a5)
move.l #-1,(0x1c,%a5)
move.l #-1,(0x20,%a5)
move.l #-1,(0x24,%a5)
move.l #-1,(0x28,%a5)
move.l #-1,(0x2c,%a5)
clr.l (0x10,%a5)
clr.l (0x14,%a5)
clr.l (0x18,%a5)
clr.l (0x1c,%a5)
lea (%a5),%a0
lea (0x20,%a5),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
pea (.Lc1,%pc)
pea (0x10,%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl SET_RANGES_$BR
SET_RANGES_$BR:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
lea (0x10,%a5),%a0
move.w %d1,%d2
moveq #7,%d1
.balignw 4,0x2048
1:
clr.l (%a0)+
dbf %d1,1b
move.w %d2,%d1
cmpi.w #0xff,%d2
bls.s 2f
move.w #0xff,%d2
move.w %d2,%d1
2:
lea (0x10,%a5),%a0
sub.w %d0,%d1
bcs.s 3f
.balignw 4,0x2048
4:
move.w %d2,%d0
lsr.w #3,%d0
neg.w %d0
bset %d2,(0x1f,%a0,%d0.w)
subq.w #1,%d2
dbf %d1,4b
3:
pea (.Lc1,%pc)
pea (0x10,%a5)
jsr (EXEC_$LOG_MEM).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SET_RANGES","aw",@nobits
.balign 4
.Ldata:
.space 48
