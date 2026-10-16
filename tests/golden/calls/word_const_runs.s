.text
.globl WRUNS_$SOCK
WRUNS_$SOCK:
link.w %fp,#0
subq.l #2,%sp
move.l #0x80400,-(%sp)
move.l #0x100001,-(%sp)
move.w #1,-(%sp)
jsr (SOCK_$OPEN).l
lea (0xc,%sp),%sp
move.b %d0,(OK).l
move.l #0x10400,-(%sp)
move.l #0x10001,-(%sp)
move.l #NUM,-(%sp)
jsr (SOCK_$ALLOCATE_USER).l
move.b %d0,(OK).l
unlk %fp
rts
.globl WRUNS_$LOG
WRUNS_$LOG:
link.w %fp,#0
subq.l #2,%sp
clr.l -(%sp)
clr.w -(%sp)
move.w (WORD_V).l,-(%sp)
move.w (ASID_V).l,-(%sp)
move.w (WORD_V).l,-(%sp)
move.l (8,%fp),-(%sp)
move.w #5,-(%sp)
jsr (NETLOG_$LOG_IT).l
unlk %fp
rts
.globl WRUNS_$UNLOCK
WRUNS_$UNLOCK:
link.w %fp,#-0x10
movea.l (8,%fp),%a0
move.l (%a0),(-4,%fp)
pea (-0x10,%fp)
pea (-0xc,%fp)
clr.l -(%sp)
clr.l -(%sp)
clr.l -(%sp)
move.l #0x40000,-(%sp)
move.l (-4,%fp),-(%sp)
move.l #ANY_UID,-(%sp)
jsr (FILE_$PRIV_UNLOCK).l
move.b %d0,(OK).l
unlk %fp
rts
.globl WRUNS_$LOCK
WRUNS_$LOCK:
link.w %fp,#-0x10
movea.l (8,%fp),%a0
move.w (%a0),(-2,%fp)
subq.l #2,%sp
pea (-0x10,%fp)
pea (-0xa,%fp)
pea (-8,%fp)
clr.w -(%sp)
pea (.Lc0,%pc)
clr.l -(%sp)
clr.l -(%sp)
clr.l -(%sp)
move.l #0x880000,-(%sp)
clr.w -(%sp)
move.w (-2,%fp),-(%sp)
move.w #1,-(%sp)
move.w (ASID_V).l,-(%sp)
move.l #ANY_UID,-(%sp)
jsr (FILE_$PRIV_LOCK).l
lea (0x30,%sp),%sp
subq.l #2,%sp
pea (-0x10,%fp)
pea (-0xa,%fp)
pea (-8,%fp)
clr.w -(%sp)
pea (.Lc0,%pc)
clr.l -(%sp)
clr.l -(%sp)
clr.l -(%sp)
clr.l -(%sp)
st -(%sp)
pea (4).w
move.w (ASID_V).l,-(%sp)
move.l #ANY_UID,-(%sp)
jsr (FILE_$PRIV_LOCK).l
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0x0000,0x0000
.balignw 4,0x2048
