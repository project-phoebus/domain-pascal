.text
.globl PBU_$FREE_ASID
PBU_$FREE_ASID:
rts
.globl PBU_$FAULTED_UNITS
PBU_$FAULTED_UNITS:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.l #0x1e000a,(%a0)
clr.w %d0
unlk %fp
rts
.globl PBU_$INIT
PBU_$INIT:
rts
.balignw 4,0x2048
