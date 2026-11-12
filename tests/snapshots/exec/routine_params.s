.text
.globl ADD
ADD:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a5),%d1
move.w (%a0),%d0
add.w %d0,%d1
move.w %d1,(%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl TWICE
TWICE:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.w (%a0),%d0
move.w %d0,%d1
add.w %d0,%d1
move.w %d1,%d0
unlk %fp
rts
.globl SQUARE
SQUARE:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.w (%a0),%d0
move.w %d0,%d1
muls.w %d0,%d1
move.w %d1,%d0
unlk %fp
rts
.globl APPLY
APPLY:
link.w %fp,#0
pea (0xc,%fp)
movea.l (8,%fp),%a0
jsr (%a0)
addq.w #4,%sp
pea (0xc,%fp)
movea.l (8,%fp),%a0
jsr (%a0)
unlk %fp
rts
.globl MAP
MAP:
link.w %fp,#-4
pea (0xc,%fp)
movea.l (8,%fp),%a0
jsr (%a0)
unlk %fp
rts
.globl PASS_ON
PASS_ON:
link.w %fp,#-4
subq.l #2,%sp
move.w (0xc,%fp),%d0
addq.w #1,%d0
move.w %d0,-(%sp)
move.l (8,%fp),-(%sp)
bsr.s MAP
unlk %fp
rts
.globl FOUR
FOUR:
link.w %fp,#-4
moveq #4,%d0
unlk %fp
rts
.globl CUBE
CUBE:
link.w %fp,#-4
move.l %d2,-(%sp)
movea.l (8,%fp),%a0
jsr (%a0)
move.l %d0,%d2
movea.l (8,%fp),%a0
jsr (%a0)
move.l %d0,%d1
movea.l (8,%fp),%a0
muls.w %d1,%d2
jsr (%a0)
move.l %d0,%d1
muls.w %d1,%d2
move.w %d2,%d0
move.l (-8,%fp),%d2
unlk %fp
rts
.globl PROC_ONCE
PROC_ONCE:
link.w %fp,#0
movea.l (8,%fp),%a0
jsr (%a0)
unlk %fp
rts
.globl BUMP
BUMP:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
addi.w #0x64,(%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ROUTINE_PARAMS_$RUN
ROUTINE_PARAMS_$RUN:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
clr.w (%a5)
subq.l #2,%sp
move.w %d2,-(%sp)
move.l #ADD,-(%sp)
bsr.w APPLY
addq.w #8,%sp
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d2,%d0
bpl.s 1f
addq.w #1,%d0
1:
asr.w #1,%d0
move.w %d0,-(%sp)
move.l #ADD,-(%sp)
bsr.w APPLY
addq.w #8,%sp
subq.l #2,%sp
move.w (%a5),%d0
addq.w #1,%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
subq.l #2,%sp
move.w #3,-(%sp)
move.l #TWICE,-(%sp)
bsr.w MAP
addq.w #8,%sp
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
subq.l #2,%sp
move.w %d2,-(%sp)
move.l #SQUARE,-(%sp)
bsr.w MAP
addq.w #8,%sp
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
subq.l #2,%sp
move.w #3,-(%sp)
move.l #SQUARE,-(%sp)
bsr.w PASS_ON
addq.w #8,%sp
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.l #FOUR,-(%sp)
bsr.w CUBE
addq.w #4,%sp
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.l #BUMP,-(%sp)
bsr.w PROC_ONCE
addq.w #4,%sp
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ROUTINE_PARAMS","aw",@nobits
.balign 4
.Ldata:
.space 4
