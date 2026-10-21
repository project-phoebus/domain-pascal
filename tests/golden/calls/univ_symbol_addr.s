.text
.globl USYM_$LOG
USYM_$LOG:
link.w %fp,#0
move.w #6,-(%sp)
pea (.Lc0,%pc)
move.w #6,-(%sp)
jsr (LOG_$ADD).l
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.long PBU_CTL
.globl USYM_$WIRE
USYM_$WIRE:
link.w %fp,#0
pea (.Lc1,%pc)
pea (.Lc2,%pc)
jsr (MST_$WIRE_AREA).l
addq.w #8,%sp
pea (.Lc3,%pc)
move.l (%sp),-(%sp)
jsr (MST_$WIRE_AREA).l
unlk %fp
rts
.balignw 4,0x2048
.Lc1:
.long PEB_$WIRED_PROC_END
.Lc3:
.long PBU_CTL+2
.Lc2:
.long PEB_$WIRED_PROC_START
.globl USYM_$ROUTINE
USYM_$ROUTINE:
link.w %fp,#0
pea (.Lc0,%pc)
pea (.Lc4,%pc)
jsr (EXT_$CALLBACK).l
unlk %fp
rts
.balignw 4,0x2048
.Lc4:
.long USYM_$CB
.balignw 4,0x2048
