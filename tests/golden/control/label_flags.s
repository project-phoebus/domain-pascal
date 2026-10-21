.text
.globl REL
REL:
link.w %fp,#0
movea.l (U).l,%a0
subq.w #1,(0x30,%a0)
bpl.s 1f
clr.w (0x30,%a0)
bra.s 2f
1:
bne.s 3f
jsr (Z).l
3:
2:
unlk %fp
rts
.globl REL_ABS
REL_ABS:
link.w %fp,#0
subq.w #1,(N).l
bpl.s 1f
clr.w (N).l
bra.s 2f
1:
bne.s 3f
jsr (Z).l
3:
2:
unlk %fp
rts
.globl SETDOWN
SETDOWN:
link.w %fp,#0
subq.w #1,(N).l
bpl.s 1f
clr.w (N).l
1:
bne.s 2f
jsr (Z).l
2:
unlk %fp
rts
.globl TST_ELSE
TST_ELSE:
link.w %fp,#0
tst.w (N).l
ble.s 1f
jsr (Z).l
bra.s 2f
1:
bne.s 3f
jsr (Y).l
3:
2:
unlk %fp
rts
.globl TST_ELSE_ORDERED
TST_ELSE_ORDERED:
link.w %fp,#0
tst.w (N).l
bne.s 1f
jsr (Z).l
bra.s 2f
1:
ble.s 3f
jsr (Y).l
3:
2:
unlk %fp
rts
.globl EQ_AFTER_SUB
EQ_AFTER_SUB:
link.w %fp,#0
subq.w #1,(N).l
tst.w (N).l
bne.s 1f
jsr (Z).l
1:
unlk %fp
rts
.globl GT_AFTER_SUB
GT_AFTER_SUB:
link.w %fp,#0
subq.w #1,(N).l
tst.w (N).l
ble.s 1f
jsr (Z).l
bra.s 2f
1:
bne.s 3f
jsr (Y).l
3:
2:
unlk %fp
rts
.globl CMPI_ELSE
CMPI_ELSE:
link.w %fp,#0
cmpi.w #0x20,(N).l
bne.s 1f
jsr (Z).l
bra.s 2f
1:
tst.w (N).l
bne.s 3f
jsr (Y).l
3:
2:
unlk %fp
rts
.globl UNTIL_AFTER_JOIN
UNTIL_AFTER_JOIN:
link.w %fp,#0
.balignw 4,0x2048
1:
subq.w #1,(N).l
bpl.s 2f
clr.w (N).l
2:
tst.w (N).l
bne.s 1b
unlk %fp
rts
.balignw 4,0x2048
