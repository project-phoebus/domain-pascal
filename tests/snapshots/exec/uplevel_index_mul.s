.text
INIT_UNIT:
link.w %fp,#-8
movea.l (%fp),%a0
moveq #0x2e,%d0
movea.l #UI_$UNITS,%a1
mulu.w (-2,%a0),%d0
lea (0,%a1,%d0.w),%a1
move.w (-2,%a0),%d0
addi.w #0x44c,%d0
move.w %d0,(%a1)
move.w (-2,%a0),%d0
addi.w #0x44c,%d0
addq.w #1,%d0
move.w %d0,(0x2c,%a1)
unlk %fp
rts
.globl SET_OTHER
SET_OTHER:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.w (%a0),(-2,%fp)
bsr.s INIT_UNIT
unlk %fp
rts
INIT_OWN:
link.w %fp,#-8
movea.l (%fp),%a0
moveq #0x2e,%d0
mulu.w (-2,%a0),%d0
lea (0,%a5,%d0.w),%a1
move.w (-2,%a0),%d0
addi.w #0x898,%d0
move.w %d0,(%a1)
move.w (-2,%a0),%d0
addi.w #0x898,%d0
addq.w #1,%d0
move.w %d0,(0x2c,%a1)
unlk %fp
rts
.globl SET_OWN
SET_OWN:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),(-2,%fp)
bsr.s INIT_OWN
movea.l (-8,%fp),%a5
unlk %fp
rts
INIT_BIG:
link.w %fp,#-4
movea.l (%fp),%a0
moveq #0x2e,%d0
movea.l #UI_$BIG,%a1
muls.w (-2,%a0),%d0
movea.l (0xc,%a0),%a0
lea (0,%a1,%d0.l),%a1
adda.l #0x8fec,%a1
move.w (%a0),(%a1)
unlk %fp
rts
.globl SET_BIG
SET_BIG:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.w (%a0),(-2,%fp)
bsr.s INIT_BIG
unlk %fp
rts
GET:
link.w %fp,#-8
movea.l (%fp),%a0
moveq #0x2e,%d0
movea.l #UI_$BIG,%a1
muls.w (-2,%a0),%d0
lea (0,%a1,%d0.l),%a0
adda.l #0x8fec,%a0
move.w (%a0),%d0
unlk %fp
rts
.globl GET_BIG
GET_BIG:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.w (%a0),(-2,%fp)
bsr.s GET
unlk %fp
rts
.globl UI_$RUN
UI_$RUN:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
moveq #3,%d0
movea.l #UI_$UNITS,%a0
lea (%a5),%a1
.balignw 4,0x2048
2:
clr.w (0,%a0)
clr.w (0x2c,%a0)
clr.w (0,%a1)
clr.w (0x2c,%a1)
lea (0x2e,%a1),%a1
lea (0x2e,%a0),%a0
dbf %d0,2b
pea (.Lc0,%pc)
bsr.w SET_OTHER
addq.w #4,%sp
pea (.Lc0,%pc)
bsr.w SET_OWN
addq.w #4,%sp
pea (UI_$UNITS+0x2c).l
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (UI_$UNITS+0x30).l
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (UI_$UNITS+0x2e).l
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (UI_$UNITS+0x5a).l
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (UI_$UNITS+0x5c).l
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (0x2c,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (0x2e,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (0x5a,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (0x5c,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
clr.w (UI_$BIG+0x2c).l
clr.w (UI_$BIG+0x5a).l
move.w #-1,(UI_$BIG+0x5c).l
move.w #-1,(UI_$BIG+0x11f52).l
clr.w (UI_$BIG+0x11fac).l
pea (.Lc1,%pc)
pea (.Lc2,%pc)
bsr.w SET_BIG
addq.w #8,%sp
pea (.Lc3,%pc)
pea (.Lc4,%pc)
bsr.w SET_BIG
addq.w #8,%sp
pea (UI_$BIG+0x2c).l
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (UI_$BIG+0x86).l
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (UI_$BIG+0x5c).l
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (.Lc2,%pc)
bsr.w GET_BIG
addq.w #4,%sp
move.w %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (UI_$BIG+0x11f7c).l
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (UI_$BIG+0x11fac).l
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (.Lc4,%pc)
bsr.w GET_BIG
addq.w #4,%sp
move.w %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (UI_$BIG+0x11f52).l
jsr (EXEC_$LOG_W).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.Lc0:
.short 0x0001
.Lc4:
.short 0x031f
.Lc3:
.short 0x1133
.Lc2:
.short 0xfce1
.Lc1:
.short 0x0ce6
.balignw 4,0x2048
.section ".bss.moddata.UPLEVEL_INDEX_MUL","aw",@nobits
.balign 4
.Ldata:
.space 184
