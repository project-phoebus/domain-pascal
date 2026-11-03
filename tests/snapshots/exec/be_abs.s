.text
.globl BE_ABS_$WIDE
BE_ABS_$WIDE:
link.w %fp,#-8
movea.l (8,%fp),%a0
move.w (%a0),%d0
bpl.s 1f
neg.w %d0
1:
clr.l %d1
move.w %d0,%d1
move.l %d1,%d0
unlk %fp
rts
.globl BE_ABS_$ELEM
BE_ABS_$ELEM:
link.w %fp,#-4
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
lea (%a5),%a0
move.l %a0,(8,%a5)
movea.l (0xc,%fp),%a1
move.w (%a1),%d0
bpl.s 1f
neg.w %d0
1:
movea.l (8,%fp),%a0
move.w (%a0),%d1
movea.l (8,%a5),%a0
add.w %d1,%d1
move.w %d0,(-2,%a0,%d1.w)
moveq #2,%d2
lea (2,%a5),%a2
.balignw 4,0x2048
3:
pea (-2,%a2)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
addq.l #2,%a2
dbf %d2,3b
movem.l (-0x10,%fp),%d2/%a2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_ABS","aw",@nobits
.balign 4
.Ldata:
.space 12
