#!/usr/bin/env python3
"""objdis.py OBJ... - objdump -dr of dpc objects in alldis format, absolute
operands and immediates named by their relocation's symbol."""
import re, subprocess, sys
for obj in sys.argv[1:]:
    out = subprocess.run(['m68k-elf-objdump', '-dr', obj], capture_output=True, text=True).stdout
    lines = out.splitlines()
    pending = None
    def flush():
        global pending
        if pending:
            print(pending[0])
            pending = None
    for line in lines:
        m = re.match(r'([0-9a-f]+) <([^>]+)>:', line)
        if m:
            flush()
            print('== %s %s' % (m.group(2), m.group(1)))
            continue
        r = re.match(r'\s+[0-9a-f]+: (R_68K_\S+)\s+(\S+)', line)
        if r and pending:
            sym = r.group(2)
            txt = pending[0]
            head, ops = txt.split('\t', 2)[:2], txt.split('\t', 2)[2] if txt.count('\t') >= 2 else ''
            # replace the next bare 0 operand (absolute) or #0 immediate
            new, n = re.subn(r'(?<![\w@(+-])(#?)0(?: <[^>]*>)?(?=,|$|\))', lambda mm: mm.group(1) + sym, ops, count=1)
            if n:
                pending = ('\t'.join(head + [new]),)
            continue
        m = re.match(r'(\s+[0-9a-f]+:\t.*)', line)
        if m:
            flush()
            t = re.sub(r' <[^>]*>', '', m.group(1))
            pending = (t,)
    flush()
