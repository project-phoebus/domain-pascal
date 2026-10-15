.text
.globl CLEANUP
CLEANUP:
link.w %fp,#-0x1c
pea (%a5)
lea (.Ldata).l,%a5
pea (-0x18,%fp)
jsr (FIM_$CLEANUP).l
addq.w #4,%sp
move.l %d0,(-0x1c,%fp)
cmpi.l #0x120035,%d0
bne.s 1f
addq.w #1,(4,%a5)
pea (-0x18,%fp)
jsr (FIM_$RLS_CLEANUP).l
addq.w #4,%sp
1:
pea (-0x1c,%fp)
jsr (REPORT).l
movea.l (-0x20,%fp),%a5
unlk %fp
rts
.globl FIELD
FIELD:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w #3,-(%sp)
jsr (PROBE).l
addq.w #4,%sp
move.l %d0,(-4,%fp)
cmpi.w #5,(-2,%fp)
bne.s 1f
clr.w (4,%a5)
1:
pea (-4,%fp)
jsr (REPORT).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl INTO_STATIC
INTO_STATIC:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (4,%a5),-(%sp)
jsr (PROBE).l
move.l %d0,(%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl DROPPED
DROPPED:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w #1,-(%sp)
jsr (PROBE).l
addq.w #4,%sp
subq.l #2,%sp
move.w #2,-(%sp)
jsr (PROBE).l
move.l %d0,(%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl GET_STATUS
GET_STATUS:
link.w %fp,#-8
subq.l #2,%sp
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (PROBE).l
addq.w #4,%sp
move.l %d0,(-4,%fp)
beq.s 1f
move.w #1,(-2,%fp)
1:
move.l (-4,%fp),(-8,%fp)
move.l (-8,%fp),%d0
unlk %fp
rts
.globl PASS_ON
PASS_ON:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.l (%a0),(-4,%fp)
move.l (-4,%fp),%d0
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.RRES","aw",@nobits
.balign 4
.Ldata:
.space 8
