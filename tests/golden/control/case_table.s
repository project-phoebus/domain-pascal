.text
.globl OSINFO
OSINFO:
link.w %fp,#0
movea.l (8,%fp),%a0
move.w (0x26,%a0),%d0
cmpi.w #5,%d0
bcc.s 1f
add.w %d0,%d0
move.w (7f,%pc,%d0.w),%d0
jmp (7f,%pc,%d0.w)
7:
.short 2f-7b
.short 3f-7b
.short 4f-7b
.short 6f-7b
.short 5f-7b
2:
move.w #7,(X3).l
bra.s 1f
3:
move.l (0x28,%a0),(Y1).l
bra.s 1f
4:
move.w #1,(X2).l
bra.s 1f
5:
clr.w (X1).l
bra.s 1f
6:
move.w #2,(X2).l
1:
unlk %fp
rts
.globl CREATE_P
CREATE_P:
link.w %fp,#0
move.w (K).l,%d0
subq.w #3,%d0
cmpi.w #8,%d0
bcc.s 4f
add.w %d0,%d0
move.w (5f,%pc,%d0.w),%d0
jmp (5f,%pc,%d0.w)
5:
.short 2f-5b
.short 2f-5b
.short 2f-5b
.short 4f-5b
.short 4f-5b
.short 3f-5b
.short 1f-5b
.short 2f-5b
2:
move.w #5,(X1).l
bra.s 1f
3:
move.w #6,(X1).l
bra.s 1f
4:
clr.w (X1).l
1:
unlk %fp
rts
.globl SERVER
SERVER:
link.w %fp,#0
clr.w %d0
move.b (C).l,%d0
subi.w #0x64,%d0
cmpi.w #9,%d0
bcc.s 5f
add.w %d0,%d0
move.w (6f,%pc,%d0.w),%d0
jmp (6f,%pc,%d0.w)
6:
.short 2f-6b
.short 5f-6b
.short 3f-6b
.short 5f-6b
.short 3f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 4f-6b
2:
move.w #1,(X1).l
bra.s 1f
3:
move.w #2,(X1).l
bra.s 1f
4:
move.w #3,(X1).l
bra.s 1f
5:
clr.w (X2).l
1:
unlk %fp
rts
.globl TOUCH_AHEAD
TOUCH_AHEAD:
link.w %fp,#0
move.w (K).l,%d0
addq.w #4,%d0
cmpi.w #4,%d0
bcc.s 1f
add.w %d0,%d0
move.w (6f,%pc,%d0.w),%d0
jmp (6f,%pc,%d0.w)
6:
.short 5f-6b
.short 4f-6b
.short 3f-6b
.short 2f-6b
2:
move.w #1,(X1).l
bra.s 1f
3:
move.w #2,(X1).l
bra.s 1f
4:
move.w #3,(X1).l
bra.s 1f
5:
move.w #4,(X1).l
1:
unlk %fp
rts
.balignw 4,0x2048
