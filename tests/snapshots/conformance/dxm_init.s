.section ".text.DXM_WIRED_.OS_INIT_PROC","ax",@progbits
.globl DXM_$INIT
DXM_$INIT:
link.w %fp,#0
movea.l #.Ldata.WIRED_DATA,%a0
pea (0x62c,%a0)
jsr (EC_$INIT).l
addq.w #4,%sp
movea.l #.Ldata.WIRED_DATA,%a0
pea (0x610,%a0)
jsr (EC_$INIT).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.DXM_WIRED_.WIRED_DATA","aw",@nobits
.balign 4
.Ldata.WIRED_DATA:
.space 1596
