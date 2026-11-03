.set __dpc_fragment_origin,0xe5b156
.text
.space 2
.globl XPD_$RESET_PTRACE_OPTS
XPD_$RESET_PTRACE_OPTS:
link.w %fp,#0
movea.l (8,%fp),%a0
clr.b (0xc,%a0)
clr.l (%a0)
clr.b (0xd,%a0)
clr.l (4,%a0)
clr.l (8,%a0)
unlk %fp
rts
