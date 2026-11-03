.section ".text.SCSI_.OS_INIT_PROC","ax",@progbits
.globl SCSI_$CINIT
SCSI_$CINIT:
link.w %fp,#-4
move.l #0x100002,%d0
unlk %fp
rts
.balignw 4,0x2048
.section ".text.SCSI_.SCSI_WIRED_PROC","ax",@progbits
.globl SCSI_$FREE_ASID
SCSI_$FREE_ASID:
rts
.balignw 4,0x2048
.section ".bss.moddata.SCSI_.SCSI_WIRED_DATA","aw",@nobits
.balign 4
.Ldata.SCSI_WIRED_DATA:
.globl SCSI_$PRESENT
SCSI_$PRESENT:
.space 4
