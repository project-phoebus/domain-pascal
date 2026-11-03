.arch 68020
.text
.globl MAP_STATUS
MAP_STATUS:
link.w %fp,#0
movea.l (8,%fp),%a0
move.l (%a0),%d0
subi.l #0x2e0001,%d0
cmpi.l #5,%d0
bcc.s 7f
move.w (8f,%pc,%d0.w*2),%d0
jmp (8f,%pc,%d0.w)
8:
.short 2f-8b
.short 3f-8b
.short 4f-8b
.short 5f-8b
.short 6f-8b
2:
moveq #1,%d0
bra.s 9f
3:
moveq #2,%d0
bra.s 9f
4:
moveq #3,%d0
bra.s 9f
5:
moveq #4,%d0
bra.s 9f
6:
moveq #5,%d0
9:
move.l %d0,(S).l
bra.s 1f
7:
clr.l (S).l
1:
unlk %fp
rts
.globl PICK
PICK:
link.w %fp,#0
move.w (W).l,%d0
cmpi.w #4,%d0
bcc.s 1f
move.w (6f,%pc,%d0.w*2),%d0
jmp (6f,%pc,%d0.w)
6:
.short 2f-6b
.short 3f-6b
.short 4f-6b
.short 5f-6b
2:
moveq #0xa,%d0
bra.s 7f
3:
moveq #0xb,%d0
bra.s 7f
4:
moveq #0xc,%d0
bra.s 7f
5:
moveq #0xd,%d0
7:
move.l %d0,(S).l
1:
unlk %fp
rts
.balignw 4,0x2048
