.text
.globl STASH
STASH:
link.w %fp,#0
move.w (AS_ID).l,%d0
movea.l #ECS,%a0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
lea (-0xc,%a0,%d0.w),%a1
move.l %a1,(SAVED).l
unlk %fp
rts
.globl NOTE
NOTE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
movea.l #PAGES,%a0
move.l %d0,%d1
add.l %d1,%d1
move.l %d1,%d0
lsl.l #8,%d0
lsl.l #3,%d0
add.l %d0,%d1
lea (0,%a0,%d1.l),%a1
move.w (%a5),(-0xffe,%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl SUBSYS
SUBSYS:
link.w %fp,#-4
move.w (AS_ID).l,%d0
movea.l #ELON_$BIG+60000,%a0
add.w %d0,%d0
lea (0,%a0,%d0.w),%a1
tst.w (-0x6e62,%a1)
sgt %d0
unlk %fp
rts
.globl AFTER
AFTER:
link.w %fp,#0
subq.l #2,%sp
move.w (AS_ID).l,-(%sp)
jsr (F).l
move.w %d0,(AS_ID).l
movea.l #ECS,%a0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
lea (-0xc,%a0,%d0.w),%a1
move.l %a1,(SAVED).l
unlk %fp
rts
.globl BOTH
BOTH:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (AS_ID).l,%d0
movea.l #ECS,%a0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
lea (-0xc,%a0,%d0.w),%a1
move.l %a1,(SAVED).l
move.w (%a5),%d0
movea.l #QUITS,%a0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
lea (-0xc,%a0,%d0.w),%a0
move.l %a0,(OTHER).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ELON","aw",@nobits
.balign 4
.Ldata:
.space 4
.comm ELON_$BIG,32896,4
