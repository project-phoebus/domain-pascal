.text
.globl GET_BOOT_FLAGS
GET_BOOT_FLAGS:
link.w %fp,#0
movea.l (8,%fp),%a0
move.w (BOOT_FLAGS).l,(%a0)
unlk %fp
rts
.balignw 4,0x2048
