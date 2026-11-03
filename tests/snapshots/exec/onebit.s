.text
.globl ONEBIT_$SET
ONEBIT_$SET:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.l (0xa,%fp),%d1
move.w %d0,(%a5)
move.l %d1,(4,%a5)
move.w %d0,(8,%a5)
move.l %d1,(0xa,%a5)
lea (8,%a5),%a0
move.l %a0,(0x10,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ONEBIT_$TEST
ONEBIT_$TEST:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (%a5),%d0
lsr.b #1,%d0
bcc.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
move.w (%a5),%d0
btst #6,%d0
beq.s 2f
subq.l #2,%sp
move.w #6,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
move.w (%a5),%d0
btst #8,%d0
beq.s 3f
subq.l #2,%sp
move.w #8,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
3:
move.w (%a5),%d0
btst #14,%d0
beq.s 4f
subq.l #2,%sp
move.w #0xe,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
4:
move.w (%a5),%d0
btst #1,%d0
bne.s 5f
subq.l #2,%sp
move.w #0x14,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
5:
move.l (4,%a5),%d0
lsr.b #1,%d0
bcc.s 6f
subq.l #2,%sp
move.w #0x1f,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
6:
move.l (4,%a5),%d0
btst #6,%d0
beq.s 7f
subq.l #2,%sp
move.w #0x24,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
7:
move.l (4,%a5),%d0
btst #8,%d0
beq.s 8f
subq.l #2,%sp
move.w #0x26,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
8:
move.l (4,%a5),%d0
btst #16,%d0
beq.s 9f
subq.l #2,%sp
move.w #0x2e,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
9:
move.l (4,%a5),%d0
btst #30,%d0
beq.s 10f
subq.l #2,%sp
move.w #0x3c,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
10:
move.l (4,%a5),%d0
lsr.b #1,%d0
bcs.s 11f
subq.l #2,%sp
move.w #0x46,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
11:
subq.l #2,%sp
move.w #0x63,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ONEBIT_$REC
ONEBIT_$REC:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0x10,%a5),%a0
move.w (%a0),%d0
lsr.b #1,%d0
bcc.s 1f
subq.l #2,%sp
move.w #0x65,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
movea.l (0x10,%a5),%a0
move.l (2,%a0),%d0
btst #5,%d0
beq.s 2f
subq.l #2,%sp
move.w #0x69,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
movea.l (0x10,%a5),%a0
move.l (2,%a0),%d0
lsr.l #4,%d0
btst #16,%d0
beq.s 3f
subq.l #2,%sp
move.w #0x74,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
3:
subq.l #2,%sp
move.w #0xc7,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ONEBIT","aw",@nobits
.balign 4
.Ldata:
.space 20
