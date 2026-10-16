.text
.globl LOVL_$A
LOVL_$A:
link.w %fp,#0
clr.w (LOVL_$DATA+0x8e04).l
clr.w (LOVL_$LOT+0xc400).l
pea (LOVL_$DATA+0x8e00).l
jsr (LOVL_$USE).l
clr.l (LOVL_$DATA+0x8500).l
unlk %fp
rts
.globl LOVL_$B
LOVL_$B:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.l (%a0),%d0
lsl.l #2,%d0
move.l %d0,%d1
neg.l %d0
lsl.l #3,%d1
movea.l #LOVL_$LOT,%a0
add.l %d1,%d0
move.b (-2,%a0,%d0.l),%d0
unlk %fp
rts
.globl LOVL_$C
LOVL_$C:
link.w %fp,#-4
movea.l (8,%fp),%a0
movea.l #LOVL_$DATA+60000,%a1
moveq #0x3f,%d0
addq.l #2,%a1
.balignw 4,0x2048
2:
move.w (%a0),(-0x5c62,%a1)
addq.l #2,%a1
dbf %d0,2b
unlk %fp
rts
.globl LOVL_$D
LOVL_$D:
link.w %fp,#0
movea.l (8,%fp),%a0
move.l (%a0),%d0
movea.l #LOVL_$DATA,%a0
lsl.l #8,%d0
lsl.l #2,%d0
clr.w (0,%a0,%d0.l)
unlk %fp
rts
.balignw 4,0x2048
.comm LOVL_$DATA,36480,4
.comm LOVL_$LOT,50180,4
