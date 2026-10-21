.text
.globl PTRK_$ACQUIRE
PTRK_$ACQUIRE:
link.w %fp,#-0x48
moveq #0xf,%d0
lea (%fp),%a0
.balignw 4,0x2048
2:
movea.l #-1,%a1
move.l %a1,(-0x42,%a0)
addq.l #4,%a0
dbf %d0,2b
move.w #7,(-0x48,%fp)
pea (-0x48,%fp)
jsr (PTRK_$SETUP).l
unlk %fp
rts
.globl PTRK_$SET
PTRK_$SET:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l #5,%a1
movea.l (%a0),%a0
move.l %a1,(0x12,%a0)
unlk %fp
rts
.globl PTRK_$SET_STACK
PTRK_$SET_STACK:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (%a0),%a0
move.l #0x7000,(2,%a0)
unlk %fp
rts
.globl PTRK_$UNUSED
PTRK_$UNUSED:
link.w %fp,#-4
movea.l (8,%fp),%a0
movea.l (%a0),%a0
cmpi.l #-1,(0xe,%a0)
seq %d0
unlk %fp
rts
.globl PTRK_$SUM
PTRK_$SUM:
link.w %fp,#-0xc
clr.l %d0
movea.l (8,%fp),%a0
movea.l (%a0),%a0
bra.s 2f
.balignw 4,0x2048
1:
add.l (4,%a0),%d0
movea.l (%a0),%a0
cmpi.l #0x64,%d0
ble.s 4f
movea.l #0x800000,%a0
bra.s 5f
4:
tst.l %d0
bpl.s 6f
suba.l %a0,%a0
6:
5:
2:
cmpa.l #0x800000,%a0
bne.s 1b
unlk %fp
rts
.balignw 4,0x2048
