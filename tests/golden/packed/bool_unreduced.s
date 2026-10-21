.text
.globl ENTRY_CLEAR
ENTRY_CLEAR:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d0
neg.w %d1
lsl.w #3,%d0
add.w %d0,%d1
bclr #15,(0xa,%a5,%d1.w)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ENTRY_SET
ENTRY_SET:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d0
neg.w %d1
lsl.w #3,%d0
add.w %d0,%d1
bset #15,(0xa,%a5,%d1.w)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ENTRY_LOW
ENTRY_LOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d0
neg.w %d1
lsl.w #3,%d0
add.w %d0,%d1
bclr #1,(0xb,%a5,%d1.w)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PTR_CLEAR
PTR_CLEAR:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (%a0),%a1
bclr #7,(2,%a1)
movea.l (%a0),%a1
bset #7,(%a1)
movea.l (%a0),%a0
bset #1,(3,%a0)
unlk %fp
rts
.globl ABS_BITS
ABS_BITS:
link.w %fp,#0
bset #15,(X_$HDR+2).l
bclr #15,(X_$HDR).l
bset #14,(X_$HDR).l
bclr #1,(X_$HDR+3).l
bset #0,(X_$HDR+3).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BOOL_UNREDUCED","aw",@nobits
.balign 4
.Ldata:
.space 112
