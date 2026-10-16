.text
.globl NS_$ADD_TO_WSL
NS_$ADD_TO_WSL:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (6,%a0),(6,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl NS_$ADD_PAGES
NS_$ADD_PAGES:
link.w %fp,#-0x40
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.l (-0x40,%fp)
move.w %d0,%d1
addq.w #1,%d1
lsl.w #2,%d1
move.w (-0x42,%fp,%d1.w),(0x10,%a5)
movea.l (-0x44,%fp),%a5
unlk %fp
rts
.globl NS_$FREE_PAGES
NS_$FREE_PAGES:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
add.w %d1,%d1
move.w %d1,%d0
lsl.w #2,%d0
add.w %d0,%d1
move.w (0x6e,%a5),(-4,%a5,%d1.w)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl NS_$SET_TMASK
NS_$SET_TMASK:
link.w %fp,#0
movea.l (0xc,%fp),%a0
movea.l (8,%fp),%a1
move.b (0x33,%a0),(4,%a1)
unlk %fp
rts
.globl NS_$LOGIT
NS_$LOGIT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.b (0x39,%a0),(0x71,%a5)
clr.w %d0
move.b (0x19,%a0),%d0
add.w %d0,%d0
move.b (0x19,%a0,%d0.w),(0x70,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl NS_$IO
NS_$IO:
link.w %fp,#0
movea.l (8,%fp),%a0
move.b (0xf,%fp),(6,%a0)
unlk %fp
rts
.globl NS_$TABLE
NS_$TABLE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.l (8,%fp),-(%sp)
move.w #1,-(%sp)
jsr (ROUTE_$FIND_PORT).l
move.b %d0,(0x72,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl NS_$SET_SERVICE
NS_$SET_SERVICE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),-(%sp)
jsr (MMAP_$REMOTE_POOL).l
move.w %d0,(0x74,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.NS","aw",@nobits
.balign 4
.Ldata:
.space 120
