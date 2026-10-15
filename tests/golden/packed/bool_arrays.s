.text
.globl COUNT
COUNT:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (PROC1_$CURRENT).l,%d0
movea.l (8,%fp),%a0
cmpi.w #1,(%a0)
bne.s 1f
move.b %d0,%d1
clr.w %d2
subq.b #1,%d1
move.b %d1,%d2
lsr.w #3,%d2
lea (0,%a5,%d2.w),%a0
moveq #7,%d2
moveq #7,%d3
and.b %d1,%d2
sub.b %d2,%d3
btst %d3,(0x18,%a0)
beq.s 2f
move.w %d0,%d1
lsl.w #2,%d1
clr.l (0x1c,%a5,%d1.w)
2:
1:
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FORK
FORK:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.b (PROC1_$CURRENT+1).l,%d0
clr.w %d1
subq.b #1,%d0
move.b %d0,%d1
lsr.w #3,%d1
lea (0,%a5,%d1.w),%a1
moveq #7,%d1
moveq #7,%d2
and.b %d0,%d1
sub.b %d1,%d2
btst %d2,(0x18,%a1)
beq.s 1f
move.b (1,%a0),%d0
clr.w %d1
subq.b #1,%d0
move.b %d0,%d1
lsr.w #3,%d1
lea (0,%a5,%d1.w),%a1
moveq #7,%d1
moveq #7,%d2
and.b %d0,%d1
sub.b %d1,%d2
btst %d2,(0x18,%a1)
bne.s 2f
addq.w #1,(0x122,%a5)
2:
move.w (%a0),%d0
moveq #7,%d1
subq.w #1,%d0
move.l #0x80,%d2
and.w %d0,%d1
lsr.l %d1,%d2
move.w %d0,%d1
lsr.w #3,%d1
lea (0,%a5,%d1.w),%a0
or.b %d2,(0x18,%a0)
1:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl UNPROFILE
UNPROFILE:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
moveq #7,%d1
move.w (%a0),%d0
move.l #-0x81,%d2
subq.w #1,%d0
and.w %d0,%d1
lsr.l %d1,%d2
move.w %d0,%d1
lsr.w #3,%d1
lea (0,%a5,%d1.w),%a0
and.b %d2,(0x18,%a0)
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl USED
USED:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.b (PROC1_$CURRENT+1).l,%d0
clr.w %d1
subq.b #1,%d0
move.b %d0,%d1
lsr.w #3,%d1
lea (0,%a5,%d1.w),%a0
moveq #7,%d1
moveq #7,%d2
and.b %d0,%d1
sub.b %d1,%d2
btst %d2,(0x18,%a0)
sne %d0
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl INIT_ALL
INIT_ALL:
link.w %fp,#-4
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #0x3f,%d0
moveq #1,%d1
.balignw 4,0x2048
2:
move.w %d1,%d2
moveq #7,%d3
subq.w #1,%d2
move.l #0x80,%d4
and.w %d2,%d3
lsr.l %d3,%d4
move.w %d2,%d3
lsr.w #3,%d3
lea (0,%a5,%d3.w),%a0
or.b %d4,(0x18,%a0)
addq.w #1,%d1
dbf %d0,2b
movem.l (-0x14,%fp),%d2-%d4/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.PCHIST","aw",@progbits
.balign 4
.Ldata:
.space 292
.byte 0x08,0x08,0x0c,0x00,0x10,0x00,0x00,0x00,0x3a,0x14,0x20,0x5c
.space 4
