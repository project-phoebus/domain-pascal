.text
.globl BE_L_SETS_$LOCAL
BE_L_SETS_$LOCAL:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w #0x20a,%d1
cmpi.w #2,%d0
ble.s 1f
move.w #0x21a,%d1
1:
move.b %d1,(%a5)
btst #3,(%a5)
sne %d0
move.b %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
btst #4,(%a5)
sne %d0
move.b %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_BOOL).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl BE_L_SETS_$PARAM
BE_L_SETS_$PARAM:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.b %d0,(%a5)
btst #3,(%a5)
sne %d0
move.b %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
btst #5,(%a5)
sne %d0
move.b %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_BOOL).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl BE_L_SETS_$WIDE
BE_L_SETS_$WIDE:
link.w %fp,#-4
move.l %d2,-(%sp)
move.l (8,%fp),%d0
clr.w %d2
bset %d0,%d2
btst #9,%d2
sne %d0
move.b %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_BOOL).l
move.l (-8,%fp),%d2
unlk %fp
rts
.globl BE_L_SETS_$DECL
BE_L_SETS_$DECL:
link.w %fp,#-4
clr.l %d0
move.l #0x202,%d1
move.w (8,%fp),%d0
btst %d0,%d1
sne %d1
move.b %d1,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_BOOL).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_L_SETS","aw",@nobits
.balign 4
.Ldata:
.space 4
