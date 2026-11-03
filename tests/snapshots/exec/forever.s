.text
.globl FOREVER_$MID
FOREVER_$MID:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(2,%a5)
clr.w (%a5)
.balignw 4,0x2048
1:
addq.w #1,(%a5)
move.w (%a5),%d0
cmp.w (2,%a5),%d0
bgt.s 2f
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
bra.s 1b
2:
subq.l #2,%sp
move.w #0x64,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FOREVER_$LAST_IF
FOREVER_$LAST_IF:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(2,%a5)
clr.w (%a5)
.balignw 4,0x2048
1:
addq.w #1,(%a5)
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w (%a5),%d0
cmp.w (2,%a5),%d0
bne.s 1b
subq.l #2,%sp
move.w #0x64,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FOREVER_$LAST_EXIT
FOREVER_$LAST_EXIT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(%a5)
.balignw 4,0x2048
1:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.w #1,(%a5)
tst.w (%a5)
bgt.s 1b
subq.l #2,%sp
move.w #0x64,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FOREVER_$INNER
FOREVER_$INNER:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(2,%a5)
clr.w (%a5)
.balignw 4,0x2048
1:
addq.w #1,(%a5)
move.w (%a5),%d0
lsr.b #1,%d0
bcc.s 1b
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w (%a5),%d0
cmp.w (2,%a5),%d0
blt.s 1b
subq.l #2,%sp
move.w (%a5),%d0
add.w %d0,%d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w #0x64,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FOREVER_$ARMS
FOREVER_$ARMS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(2,%a5)
clr.w (%a5)
.balignw 4,0x2048
1:
addq.w #1,(%a5)
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w (%a5),%d0
sub.w (2,%a5),%d0
move.w %d0,(4,%a5)
move.w (4,%a5),%d0
beq.s 3f
cmpi.w #1,%d0
beq.s 5f
bra.s 1b
3:
subq.l #2,%sp
move.w (%a5),%d0
neg.w %d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
5:
subq.l #2,%sp
move.w #0x64,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FOREVER_$RET
FOREVER_$RET:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(2,%a5)
clr.w (%a5)
.balignw 4,0x2048
1:
addq.w #1,(%a5)
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w (%a5),%d0
cmp.w (2,%a5),%d0
bne.s 1b
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FOREVER_$GOTO
FOREVER_$GOTO:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(2,%a5)
clr.w (%a5)
.balignw 4,0x2048
1:
addq.w #1,(%a5)
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w (%a5),%d0
cmp.w (2,%a5),%d0
bne.s 1b
subq.l #2,%sp
move.w #0x64,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FOREVER_$NESTED
FOREVER_$NESTED:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(2,%a5)
clr.w (%a5)
.balignw 4,0x2048
1:
addq.w #1,(%a5)
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
clr.w (4,%a5)
.balignw 4,0x2048
2:
addq.w #1,(4,%a5)
move.w (4,%a5),%d0
cmp.w (%a5),%d0
blt.s 2b
subq.l #2,%sp
move.w (4,%a5),%d0
add.w %d0,%d0
move.w %d0,%d1
lsl.w #2,%d1
add.w %d1,%d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w (%a5),%d0
cmp.w (2,%a5),%d0
bne.s 1b
subq.l #2,%sp
move.w #0x64,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FOREVER_$FRET
FOREVER_$FRET:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(2,%a5)
clr.w %d2
.balignw 4,0x2048
1:
addq.w #1,%d2
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
cmp.w (2,%a5),%d2
bne.s 1b
move.w %d2,%d0
add.w %d0,%d0
move.w %d0,%d1
lsl.w #2,%d1
add.w %d1,%d0
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl FOREVER_$FRET_LAST
FOREVER_$FRET_LAST:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(2,%a5)
clr.w %d2
.balignw 4,0x2048
1:
addq.w #1,%d2
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w %d2,%d0
cmp.w (2,%a5),%d2
bne.s 1b
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl FOREVER_$ARM_RET
FOREVER_$ARM_RET:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(2,%a5)
clr.w (%a5)
.balignw 4,0x2048
1:
addq.w #1,(%a5)
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w (%a5),%d0
sub.w (2,%a5),%d0
move.w %d0,(4,%a5)
move.w (4,%a5),%d0
beq.s 3f
cmpi.w #-1,%d0
beq.s 1b
bra.s 2f
3:
subq.l #2,%sp
move.w (%a5),%d0
neg.w %d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
bra.s 5f
2:
subq.l #2,%sp
move.w #0x3e8,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
bra.s 1b
5:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FOREVER_$GOTO_END
FOREVER_$GOTO_END:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(2,%a5)
clr.w (%a5)
.balignw 4,0x2048
1:
addq.w #1,(%a5)
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w (%a5),%d0
cmp.w (2,%a5),%d0
bne.s 1b
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FOREVER_$NEST_RET
FOREVER_$NEST_RET:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(2,%a5)
clr.w (%a5)
.balignw 4,0x2048
1:
addq.w #1,(%a5)
clr.w (4,%a5)
.balignw 4,0x2048
2:
addq.w #1,(4,%a5)
subq.l #2,%sp
move.w (4,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w (4,%a5),%d0
cmp.w (%a5),%d0
blt.s 2b
move.w (%a5),%d0
cmp.w (2,%a5),%d0
bne.s 1b
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.FOREVER","aw",@nobits
.balign 4
.Ldata:
.space 8
