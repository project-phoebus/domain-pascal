.text
.globl PACKED_BOOLS_ALL_$ALL
PACKED_BOOLS_ALL_$ALL:
link.w %fp,#-8
movem.l %d2-%d6/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.w (0x50,%a5)
move.w #0x80,(0x48,%a5)
move.w #1,(0x60,%a5)
moveq #0x3f,%d0
moveq #1,%d1
.balignw 4,0x2048
2:
move.w %d1,(0x68,%a5)
moveq #0x3f,%d2
moveq #1,%d3
.balignw 4,0x2048
5:
move.w %d3,%d4
moveq #7,%d5
subq.w #1,%d4
move.l #-0x81,%d6
and.w %d4,%d5
lsr.l %d5,%d6
move.w %d4,%d5
lsr.w #3,%d5
lea (0,%a5,%d5.w),%a0
and.b %d6,(%a0)
addq.w #1,%d3
dbf %d2,5b
move.w %d1,%d2
moveq #7,%d3
subq.w #1,%d2
move.l #0x80,%d4
and.w %d2,%d3
lsr.l %d3,%d4
move.w %d2,%d3
lsr.w #3,%d3
lea (0,%a5,%d3.w),%a0
or.b %d4,(%a0)
moveq #7,%d2
moveq #1,%d3
.balignw 4,0x2048
8:
cmp.w (0x60,%a5),%d3
bne.s 10f
move.w (0x48,%a5),(0x58,%a5)
bra.s 11f
10:
clr.w (0x58,%a5)
11:
clr.w %d4
move.b (-1,%a5,%d3.w),%d4
cmp.w (0x58,%a5),%d4
beq.s 12f
addq.w #1,(0x50,%a5)
12:
addq.w #1,%d3
dbf %d2,8b
moveq #0x3f,%d2
moveq #1,%d3
.balignw 4,0x2048
14:
move.b %d3,%d4
clr.w %d5
subq.b #1,%d4
move.b %d4,%d5
lsr.w #3,%d5
lea (0,%a5,%d5.w),%a0
moveq #7,%d5
moveq #7,%d6
and.b %d4,%d5
sub.b %d5,%d6
btst %d6,(%a0)
beq.s 16f
cmp.w (0x68,%a5),%d3
beq.s 18f
addq.w #1,(0x50,%a5)
bra.s 18f
16:
cmp.w (0x68,%a5),%d3
bne.s 19f
addq.w #1,(0x50,%a5)
19:
18:
addq.w #1,%d3
dbf %d2,14b
move.w (0x48,%a5),%d2
bpl.s 20f
addq.w #1,%d2
20:
asr.w #1,%d2
move.w %d2,(0x48,%a5)
tst.w (0x48,%a5)
bne.s 21f
move.w #0x80,(0x48,%a5)
addq.w #1,(0x60,%a5)
21:
addq.w #1,%d1
dbf %d0,2b
pea (0x50,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
clr.w (0x50,%a5)
move.w #0x80,(0x48,%a5)
move.w #1,(0x60,%a5)
moveq #0x3f,%d0
moveq #1,%d1
.balignw 4,0x2048
23:
move.w %d1,(0x68,%a5)
moveq #0x3f,%d2
moveq #1,%d3
.balignw 4,0x2048
26:
move.w %d3,%d4
moveq #7,%d5
subq.w #1,%d4
move.l #0x80,%d6
and.w %d4,%d5
lsr.l %d5,%d6
move.w %d4,%d5
lsr.w #3,%d5
lea (0,%a5,%d5.w),%a0
or.b %d6,(%a0)
addq.w #1,%d3
dbf %d2,26b
move.w %d1,%d2
moveq #7,%d3
subq.w #1,%d2
move.l #-0x81,%d4
and.w %d2,%d3
lsr.l %d3,%d4
move.w %d2,%d3
lsr.w #3,%d3
lea (0,%a5,%d3.w),%a0
and.b %d4,(%a0)
moveq #7,%d2
moveq #1,%d3
.balignw 4,0x2048
29:
cmp.w (0x60,%a5),%d3
bne.s 31f
move.w #0xff,%d4
sub.w (0x48,%a5),%d4
move.w %d4,(0x58,%a5)
bra.s 32f
31:
move.w #0xff,(0x58,%a5)
32:
clr.w %d4
move.b (-1,%a5,%d3.w),%d4
cmp.w (0x58,%a5),%d4
beq.s 33f
addq.w #1,(0x50,%a5)
33:
addq.w #1,%d3
dbf %d2,29b
moveq #0x3f,%d2
moveq #1,%d3
.balignw 4,0x2048
35:
move.b %d3,%d4
clr.w %d5
subq.b #1,%d4
move.b %d4,%d5
lsr.w #3,%d5
lea (0,%a5,%d5.w),%a0
moveq #7,%d5
moveq #7,%d6
and.b %d4,%d5
sub.b %d5,%d6
btst %d6,(%a0)
bne.s 37f
cmp.w (0x68,%a5),%d3
beq.s 39f
addq.w #1,(0x50,%a5)
bra.s 39f
37:
cmp.w (0x68,%a5),%d3
bne.s 40f
addq.w #1,(0x50,%a5)
40:
39:
addq.w #1,%d3
dbf %d2,35b
move.w (0x48,%a5),%d2
bpl.s 41f
addq.w #1,%d2
41:
asr.w #1,%d2
move.w %d2,(0x48,%a5)
tst.w (0x48,%a5)
bne.s 42f
move.w #0x80,(0x48,%a5)
addq.w #1,(0x60,%a5)
42:
addq.w #1,%d1
dbf %d0,23b
pea (0x50,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
clr.w (0x50,%a5)
moveq #0x3f,%d0
moveq #1,%d1
.balignw 4,0x2048
44:
move.w %d1,%d2
moveq #7,%d3
subq.w #1,%d2
move.l #-0x81,%d4
and.w %d2,%d3
lsr.l %d3,%d4
move.w %d2,%d3
lsr.w #3,%d3
lea (0,%a5,%d3.w),%a0
and.b %d4,(%a0)
addq.w #1,%d1
dbf %d0,44b
moveq #0x3f,%d0
moveq #1,%d1
.balignw 4,0x2048
47:
move.w %d1,(0x68,%a5)
move.w %d1,%d2
moveq #7,%d3
subq.w #1,%d2
move.l #0x80,%d4
and.w %d2,%d3
lsr.l %d3,%d4
move.w %d2,%d3
lsr.w #3,%d3
lea (0,%a5,%d3.w),%a0
or.b %d4,(%a0)
moveq #0x3f,%d2
moveq #1,%d3
.balignw 4,0x2048
50:
move.b %d3,%d4
clr.w %d5
subq.b #1,%d4
move.b %d4,%d5
lsr.w #3,%d5
lea (0,%a5,%d5.w),%a0
moveq #7,%d5
moveq #7,%d6
and.b %d4,%d5
sub.b %d5,%d6
btst %d6,(%a0)
beq.s 52f
cmp.w (0x68,%a5),%d3
ble.s 54f
addq.w #1,(0x50,%a5)
bra.s 54f
52:
cmp.w (0x68,%a5),%d3
bgt.s 55f
addq.w #1,(0x50,%a5)
55:
54:
addq.w #1,%d3
dbf %d2,50b
addq.w #1,%d1
dbf %d0,47b
pea (0x50,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
clr.w (0x50,%a5)
moveq #0x3f,%d0
moveq #1,%d1
.balignw 4,0x2048
57:
move.w %d1,(0x68,%a5)
move.w %d1,%d2
moveq #7,%d3
subq.w #1,%d2
move.l #-0x81,%d4
and.w %d2,%d3
lsr.l %d3,%d4
move.w %d2,%d3
lsr.w #3,%d3
lea (0,%a5,%d3.w),%a0
and.b %d4,(%a0)
moveq #0x3f,%d2
moveq #1,%d3
.balignw 4,0x2048
60:
move.b %d3,%d4
clr.w %d5
subq.b #1,%d4
move.b %d4,%d5
lsr.w #3,%d5
lea (0,%a5,%d5.w),%a0
moveq #7,%d5
moveq #7,%d6
and.b %d4,%d5
sub.b %d5,%d6
btst %d6,(%a0)
beq.s 62f
cmp.w (0x68,%a5),%d3
bgt.s 64f
addq.w #1,(0x50,%a5)
bra.s 64f
62:
cmp.w (0x68,%a5),%d3
ble.s 65f
addq.w #1,(0x50,%a5)
65:
64:
addq.w #1,%d3
dbf %d2,60b
addq.w #1,%d1
dbf %d0,57b
pea (0x50,%a5)
jsr (EXEC_$LOG_W).l
movem.l (-0x20,%fp),%d2-%d6/%a5
unlk %fp
rts
.globl PACKED_BOOLS_ALL_$BIGT
PACKED_BOOLS_ALL_$BIGT:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.b (1,%a0),%d0
clr.w %d1
subq.b #1,%d0
move.b %d0,%d1
lsr.w #3,%d1
lea (0,%a5,%d1.w),%a0
moveq #7,%d1
moveq #7,%d2
and.b %d0,%d1
sub.b %d1,%d2
btst %d2,(8,%a0)
sne %d0
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl PACKED_BOOLS_ALL_$BIG
PACKED_BOOLS_ALL_$BIG:
link.w %fp,#-4
movem.l %d2-%d5/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w #0xff,%d1
moveq #1,%d2
.balignw 4,0x2048
2:
move.w %d2,%d3
moveq #7,%d4
subq.w #1,%d3
move.l #-0x81,%d5
and.w %d3,%d4
lsr.l %d4,%d5
move.w %d3,%d4
lsr.w #3,%d4
lea (0,%a5,%d4.w),%a0
and.b %d5,(8,%a0)
addq.w #1,%d2
dbf %d1,2b
move.w %d0,%d1
moveq #7,%d0
subq.w #1,%d1
move.l #0x80,%d2
and.w %d1,%d0
lsr.l %d0,%d2
move.w %d1,%d0
lsr.w #3,%d0
lea (0,%a5,%d0.w),%a0
or.b %d2,(8,%a0)
pea (.Lc0,%pc)
pea (8,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (8,%fp)
bsr.w PACKED_BOOLS_ALL_$BIGT
addq.w #4,%sp
move.b %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_BOOL).l
movem.l (-0x18,%fp),%d2-%d5/%a5
unlk %fp
rts
.Lc0:
.short 0x0020
.globl PACKED_BOOLS_ALL_$LO8T
PACKED_BOOLS_ALL_$LO8T:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.b (1,%a0),%d0
clr.w %d1
subq.b #8,%d0
move.b %d0,%d1
lsr.w #3,%d1
lea (0,%a5,%d1.w),%a0
moveq #7,%d1
moveq #7,%d2
and.b %d0,%d1
sub.b %d1,%d2
btst %d2,(0x28,%a0)
sne %d0
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl PACKED_BOOLS_ALL_$LO8
PACKED_BOOLS_ALL_$LO8:
link.w %fp,#-4
movem.l %d2-%d5/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w #0xff,%d1
moveq #8,%d2
.balignw 4,0x2048
2:
move.w %d2,%d3
moveq #7,%d4
subq.w #8,%d3
move.l #-0x81,%d5
and.w %d3,%d4
lsr.l %d4,%d5
move.w %d3,%d4
lsr.w #3,%d4
lea (0,%a5,%d4.w),%a0
and.b %d5,(0x28,%a0)
addq.w #1,%d2
dbf %d1,2b
move.w %d0,%d1
moveq #7,%d0
subq.w #8,%d1
move.l #0x80,%d2
and.w %d1,%d0
lsr.l %d0,%d2
move.w %d1,%d0
lsr.w #3,%d0
lea (0,%a5,%d0.w),%a0
or.b %d2,(0x28,%a0)
pea (.Lc0,%pc)
pea (0x28,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (8,%fp)
bsr.w PACKED_BOOLS_ALL_$LO8T
addq.w #4,%sp
move.b %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_BOOL).l
movem.l (-0x18,%fp),%d2-%d5/%a5
unlk %fp
rts
.globl PACKED_BOOLS_ALL_$INITS
PACKED_BOOLS_ALL_$INITS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (.Lc1,%pc)
pea (0x6a,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (.Lc1,%pc)
pea (0x72,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (.Lc2,%pc)
pea (0x7a,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (.Lc1,%pc)
pea (0x7c,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (.Lc3,%pc)
pea (0x80,%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.Lc1:
.short 0x0004
.Lc3:
.short 0x0006
.Lc2:
.short 0x0002
.globl PACKED_BOOLS_ALL_$BYTES
PACKED_BOOLS_ALL_$BYTES:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.b (0x80,%a5),%d0
ext.w %d0
move.w %d0,(0x86,%a5)
pea (0x86,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
clr.w %d0
move.b (0x84,%a5),%d0
addi.w #0x48,%d0
move.w %d0,(0x86,%a5)
pea (0x86,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.b (0x83,%a5),%d0
ext.w %d0
move.w %d0,(0x86,%a5)
pea (0x86,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.b #0x80,(0x81,%a5)
move.b (0x81,%a5),%d0
ext.w %d0
move.w %d0,(0x86,%a5)
pea (0x86,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
clr.w %d0
move.b (0x85,%a5),%d0
move.w %d0,(0x86,%a5)
pea (0x86,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PACKED_BOOLS_ALL_$SETUP
PACKED_BOOLS_ALL_$SETUP:
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
move.l #-0x81,%d4
and.w %d2,%d3
lsr.l %d3,%d4
move.w %d2,%d3
lsr.w #3,%d3
lea (0,%a5,%d3.w),%a0
and.b %d4,(0x88,%a0)
addq.w #1,%d1
dbf %d0,2b
moveq #4,%d0
moveq #3,%d1
.balignw 4,0x2048
5:
move.w %d1,%d2
moveq #7,%d3
subq.w #1,%d2
move.l #0x80,%d4
and.w %d2,%d3
lsr.l %d3,%d4
move.w %d2,%d3
lsr.w #3,%d3
lea (0,%a5,%d3.w),%a0
or.b %d4,(0x88,%a0)
addq.w #1,%d1
dbf %d0,5b
movem.l (-0x14,%fp),%d2-%d4/%a5
unlk %fp
rts
.globl PACKED_BOOLS_ALL_$W
PACKED_BOOLS_ALL_$W:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
bra.s 2f
.balignw 4,0x2048
1:
addq.w #1,%d1
2:
move.b %d1,%d0
clr.w %d2
subq.b #1,%d0
move.b %d0,%d2
lsr.w #3,%d2
lea (0,%a5,%d2.w),%a0
moveq #7,%d2
moveq #7,%d3
and.b %d0,%d2
sub.b %d2,%d3
btst %d3,(0x88,%a0)
bne.s 1b
move.w %d1,(0x86,%a5)
pea (0x86,%a5)
jsr (EXEC_$LOG_W).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl PACKED_BOOLS_ALL_$RP
PACKED_BOOLS_ALL_$RP:
link.w %fp,#-4
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w %d2,%d0
.balignw 4,0x2048
1:
addq.w #1,%d0
move.b %d0,%d1
clr.w %d3
subq.b #1,%d1
move.b %d1,%d3
lsr.w #3,%d3
lea (0,%a5,%d3.w),%a0
moveq #7,%d3
moveq #7,%d4
and.b %d1,%d3
sub.b %d3,%d4
btst %d4,(0x88,%a0)
beq.s 1b
move.w %d0,(0x86,%a5)
pea (0x86,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.w %d2,%d0
addq.w #3,%d0
.balignw 4,0x2048
2:
addq.w #1,%d0
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
btst %d3,(0x88,%a0)
bne.s 2b
move.w %d0,(0x86,%a5)
pea (0x86,%a5)
jsr (EXEC_$LOG_W).l
movem.l (-0x14,%fp),%d2-%d4/%a5
unlk %fp
rts
.globl PACKED_BOOLS_ALL_$AT
PACKED_BOOLS_ALL_$AT:
link.w %fp,#0
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
clr.w (0x90,%a5)
move.b %d2,%d0
clr.w %d1
subq.b #1,%d0
move.b %d0,%d1
lsr.w #3,%d1
lea (0,%a5,%d1.w),%a0
moveq #7,%d1
moveq #7,%d4
and.b %d0,%d1
sub.b %d1,%d4
btst %d4,(0x88,%a0)
beq.s 1f
move.b %d3,%d0
clr.w %d1
subq.b #1,%d0
move.b %d0,%d1
lsr.w #3,%d1
lea (0,%a5,%d1.w),%a0
moveq #7,%d1
moveq #7,%d4
and.b %d0,%d1
sub.b %d1,%d4
btst %d4,(0x88,%a0)
beq.s 1f
move.w #1,(0x90,%a5)
1:
pea (0x90,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.b %d2,%d0
clr.w %d1
subq.b #1,%d0
move.b %d0,%d1
lsr.w #3,%d1
lea (0,%a5,%d1.w),%a0
moveq #7,%d1
moveq #7,%d2
and.b %d0,%d1
sub.b %d1,%d2
btst %d2,(0x88,%a0)
bne.s 3f
move.b %d3,%d0
clr.w %d1
subq.b #1,%d0
move.b %d0,%d1
lsr.w #3,%d1
lea (0,%a5,%d1.w),%a0
moveq #7,%d1
moveq #7,%d2
and.b %d0,%d1
sub.b %d1,%d2
btst %d2,(0x88,%a0)
bne.s 2f
3:
addq.w #2,(0x90,%a5)
2:
pea (0x90,%a5)
jsr (EXEC_$LOG_W).l
movem.l (-0x10,%fp),%d2-%d4/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.PACKED_BOOLS_ALL","aw",@progbits
.balign 4
.Ldata:
.space 106
.byte 0xff,0x80,0x55,0xaa,0x55,0x00,0xaa,0x00,0x12,0x34,0x56,0x00,0x00,0x00,0x00,0x00
.byte 0xe4,0x00,0x07,0x41,0x40,0x00,0xfb,0x00,0xc8,0x7f,0x80
.space 15
