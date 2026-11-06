.text
.globl PM_$ADD
PM_$ADD:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
movea.l (8,%fp),%a1
move.l (%a0),%d0
movea.l (0xc,%fp),%a0
add.l (4,%a1),%d0
add.l (%a0),%d0
move.l %d0,(0x28,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PM_$COUNT
PM_$COUNT:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (%a5),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl PM_$TARGET_B
PM_$TARGET_B:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (0x10,%a5),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl PM_$LOG
PM_$LOG:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (0x24,%a5),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl PM_$NOTED
PM_$NOTED:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (0x28,%a5),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.PROGRAM_MAIN_DATA","aw",@progbits
.balign 4
.Ldata:
.globl COUNT
COUNT:
.byte 0x00,0x00,0x00,0x05
.globl SOURCE
SOURCE:
.byte 0x00,0x00,0x00,0x0b,0x00,0x00,0x00,0x16
.globl TARGET
TARGET:
.space 8
.globl BLOCK
BLOCK:
.byte 0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x02,0x00,0x00,0x00,0x03,0x00,0x00,0x00,0x04
.globl LOG
LOG:
.space 4
.globl NOTED
NOTED:
.space 4
