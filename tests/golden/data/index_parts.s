.text
.globl IP_$SH
IP_$SH:
link.w %fp,#-4
move.l (8,%fp),%d0
movea.l #IP_$TBL,%a0
lsr.l #8,%d0
lsr.l #2,%d0
lsl.w #2,%d0
move.l (0,%a0,%d0.w),%d0
unlk %fp
rts
.globl IP_$SH1
IP_$SH1:
link.w %fp,#-4
move.l (8,%fp),%d0
movea.l #IP_$TBL,%a0
lsr.l #8,%d0
lsr.l #2,%d0
lsl.w #2,%d0
move.l (4,%a0,%d0.w),%d0
unlk %fp
rts
.globl IP_$MM
IP_$MM:
link.w %fp,#-4
move.l (8,%fp),%d0
subi.l #0x200,%d0
lsl.l #4,%d0
bpl.s 1f
addi.l #0x3ff,%d0
1:
asr.l #8,%d0
movea.l #IP_$T56,%a0
asr.l #2,%d0
lsl.w #2,%d0
move.l (0,%a0,%d0.w),%d0
unlk %fp
rts
.globl IP_$LV
IP_$LV:
link.w %fp,#-4
move.l (8,%fp),%d0
movea.l #IP_$TBL,%a0
move.l %d0,%d1
lsl.l #2,%d1
move.l (0,%a0,%d1.l),%d0
unlk %fp
rts
.globl IP_$WP1
IP_$WP1:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #IP_$W8,%a0
move.w %d0,%d1
add.w %d1,%d1
move.w (0,%a0,%d1.w),%d0
unlk %fp
rts
.globl IP_$WK
IP_$WK:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #IP_$W8,%a0
move.w %d0,%d1
add.w %d1,%d1
move.w (4,%a0,%d1.w),%d0
unlk %fp
rts
.globl IP_$WM2
IP_$WM2:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #IP_$W8,%a0
move.w %d0,%d1
add.w %d1,%d1
move.w (-6,%a0,%d1.w),%d0
unlk %fp
rts
.globl IP_$Z48
IP_$Z48:
link.w %fp,#-4
move.w (8,%fp),%d0
cmpi.w #-1,%d0
bne.s 1f
clr.l %d0
bra.s 2f
1:
clr.l %d1
movea.l #WI_$R48,%a0
move.w %d0,%d1
lsl.l #4,%d1
move.l %d1,%d0
add.l %d0,%d0
add.l %d0,%d1
move.l (0,%a0,%d1.l),%d0
2:
unlk %fp
rts
.globl IP_$M12
IP_$M12:
link.w %fp,#-4
moveq #7,%d0
movea.l #IP_$R12,%a0
and.l (8,%fp),%d0
lsl.l #2,%d0
move.l %d0,%d1
add.l %d1,%d1
add.l %d1,%d0
move.l (4,%a0,%d0.l),%d0
unlk %fp
rts
.globl IP_$LV1
IP_$LV1:
link.w %fp,#-4
move.l (8,%fp),%d0
movea.l #IP_$TBL,%a0
move.l %d0,%d1
lsl.l #2,%d1
move.l (4,%a0,%d1.l),%d0
unlk %fp
rts
.balignw 4,0x2048
