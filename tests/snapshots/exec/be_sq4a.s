.text
.globl BS_$WIDEN
BS_$WIDEN:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.b (8,%fp),%d2
move.b %d2,(6,%a5)
clr.l %d0
move.b %d2,%d0
move.l %d0,(%a5)
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.b %d2,(5,%a5)
clr.l %d0
move.b %d2,%d0
add.l (%a5),%d0
move.l %d0,(%a5)
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l %d0
move.b %d2,%d0
lsr.l #2,%d0
andi.l #0x3f,%d0
move.l %d0,(%a5)
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.w #0xf0,%d0
and.b (6,%a5),%d0
andi.l #0xffff,%d0
move.l %d0,(%a5)
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl BS_$MODW
BS_$MODW:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
move.w (0xc,%fp),%d3
subq.l #2,%sp
move.w %d3,-(%sp)
move.l %d2,-(%sp)
jsr (M$OIS$WLW).l
addq.w #8,%sp
move.w %d0,%d1
ext.l %d1
move.l %d1,(%a5)
move.l %d1,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d3,-(%sp)
move.l %d2,-(%sp)
jsr (M$OIS$WLW).l
addq.w #8,%sp
move.w %d0,%d1
ext.l %d1
addi.l #0x186a0,%d1
move.l %d1,(%a5)
move.l %d1,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BS_$BOTH
BS_$BOTH:
link.w %fp,#-4
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
move.l (0xc,%fp),%d1
move.l (0x10,%fp),%d3
move.l %d1,-(%sp)
move.l %d2,-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d3,%d1
addq.l #1,%d1
move.l %d1,-(%sp)
move.l %d0,-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d0,(%a5)
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l %d2,%d4
move.l %d3,%d0
addq.l #1,%d4
addq.l #2,%d0
move.l %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
move.l %d0,-(%sp)
move.l %d4,-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d0,(%a5)
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
pea (8,%fp)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
move.l %d3,%d1
subq.l #1,%d1
move.l %d1,-(%sp)
move.l %d0,-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d0,(%a5)
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x14,%fp),%d2-%d4/%a5
unlk %fp
rts
.globl BS_$CASE
BS_$CASE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.l (%a5)
move.w %d0,%d1
cmpi.w #1,%d1
beq.s 2f
cmpi.w #2,%d1
beq.s 3f
bra.s 1f
2:
moveq #0xa,%d0
bra.s 4f
3:
moveq #0x14,%d0
4:
move.l %d0,(%a5)
1:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BS_$GOTO_WITH
BS_$GOTO_WITH:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
beq.s 1f
move.w %d2,%d0
addq.w #1,%d0
move.w %d0,(0xa,%a5)
subq.l #2,%sp
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w %d2,(0xc,%a5)
1:
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w #0x64,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl BS_$GOTO_FOR
BS_$GOTO_FOR:
link.w %fp,#-4
movem.l %d2-%d4,-(%sp)
move.w (8,%fp),%d2
beq.s 1f
moveq #2,%d3
moveq #1,%d4
.balignw 4,0x2048
3:
subq.l #2,%sp
move.w %d4,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
cmpi.w #3,%d4
bne.s 5f
1:
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w #0x63,-(%sp)
bra.s 7f
5:
addq.w #1,%d4
dbf %d3,3b
subq.l #2,%sp
move.w #0x64,-(%sp)
7:
jsr (EXEC_$MARK).l
6:
movem.l (-0x10,%fp),%d2-%d4
unlk %fp
rts
.globl BS_$GOTO_LOOP
BS_$GOTO_LOOP:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w %d2,(8,%a5)
tst.w %d2
beq.s 1f
bra.s 3f
.balignw 4,0x2048
2:
subq.w #1,(8,%a5)
1:
subq.l #2,%sp
move.w (8,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
tst.w (8,%a5)
beq.s 5f
3:
tst.w (8,%a5)
bgt.s 2b
5:
subq.l #2,%sp
move.w #0x63,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
tst.w %d2
beq.s 6f
.balignw 4,0x2048
7:
addq.w #1,(8,%a5)
subq.l #2,%sp
move.w (8,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
6:
tst.w (8,%a5)
bmi.s 7b
subq.l #2,%sp
move.w #0x63,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl BS_$GOTO_CASE
BS_$GOTO_CASE:
link.w %fp,#0
move.l %d2,-(%sp)
move.w (8,%fp),%d2
beq.s 1f
move.w %d2,%d0
cmpi.w #1,%d0
beq.s 3f
cmpi.w #2,%d0
beq.s 4f
bra.s 5f
3:
subq.l #2,%sp
move.w #0xa,-(%sp)
bra.s 6f
4:
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
subq.l #2,%sp
move.w %d2,%d0
lsl.w #2,%d0
move.w %d0,%d1
lsl.w #2,%d1
add.w %d1,%d0
move.w %d0,-(%sp)
bra.s 6f
5:
subq.l #2,%sp
move.w #0x1e,-(%sp)
6:
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
subq.l #2,%sp
move.w #0x63,-(%sp)
jsr (EXEC_$MARK).l
move.l (-4,%fp),%d2
unlk %fp
rts
.globl BS_$ADDR_OPS
BS_$ADDR_OPS:
link.w %fp,#-0x1c
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
lea (0xe,%a5),%a0
move.l %a0,(-0x1c,%fp)
pea (-0x1c,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
move.l %a0,%d0
lea (0xe,%a5),%a0
move.l %d0,%d2
move.l %a0,%d0
andi.l #0x3ff,%d0
move.l %d0,(%a5)
move.l %d2,%d1
andi.l #0x3ff,%d1
cmp.l %d1,%d0
seq %d1
move.b %d1,(-0x1a,%fp)
pea (-0x1a,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
lea (0xe,%a5),%a0
move.l %a0,%d0
ori.l #3,%d0
move.l %d0,(%a5)
moveq #3,%d1
move.l %d2,%d3
and.l %d2,%d1
sub.l %d1,%d3
addq.l #3,%d3
cmp.l %d3,%d0
seq %d3
move.b %d3,(-0x1a,%fp)
pea (-0x1a,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
lea (0xe,%a5),%a0
move.l %a0,%d0
eori.l #0x55,%d0
move.l %d0,(%a5)
moveq #0x55,%d1
moveq #0x55,%d3
add.l %d2,%d1
and.l %d2,%d3
add.l %d3,%d3
sub.l %d3,%d1
cmp.l %d1,%d0
seq %d3
move.b %d3,(-0x1a,%fp)
pea (-0x1a,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
lea (-0x18,%fp),%a0
move.l %a0,(-0x1c,%fp)
pea (-0x1c,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
move.l %a0,%d0
lea (-0x14,%fp),%a0
move.l %d0,%d2
move.l %a0,%d0
andi.l #0xffff,%d0
move.l %d0,(%a5)
move.l %d2,%d1
addq.l #4,%d1
andi.l #0xffff,%d1
cmp.l %d1,%d0
seq %d1
move.b %d1,(-0x1a,%fp)
pea (-0x1a,%fp)
jsr (EXEC_$LOG_BOOL).l
movem.l (-0x28,%fp),%d2-%d3/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_SQ4A","aw",@nobits
.balign 4
.Ldata:
.space 48
