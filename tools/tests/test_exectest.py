"""Tests for tools/exectest.py (the execution-test driver) and its runner.

The header parser needs nothing.  The runner checks (an odd-address word
access, a clobbered callee-saved register, an illegal instruction, a
wild access, the argument slots) need Musashi and the m68k-elf tools, and
are skipped without them.  The test programs themselves run under `make
exectest' (part of `make test').
"""

import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..'))
import exectest  # noqa: E402


class SpecTest(unittest.TestCase):
    def parse(self, header, body='module m;\n'):
        return exectest.parse_spec('t.pas', '{ ' + header + ' }\n' + body)

    def test_full(self):
        s = self.parse('''Prose.
  call: M_$A
  call: m_$b w:3 @l:-5! b:200 -> w
  link: data.mod.pas
  dpc: --loop-align=off
  expect:
    1
    => 2
''')
        self.assertEqual(s.calls, [('M_$A', [], None),
                                   ('M_$B', ['w:3', '@l:-5!', 'b:200'], 'w')])
        self.assertEqual(s.dpc_opts, ['--loop-align=off'])
        self.assertEqual([os.path.basename(f) for f in s.links], ['data.mod.pas'])
        self.assertEqual(s.expect, ['1', '=> 2'])
        self.assertIsNone(s.xfail)

    def test_xfail(self):
        s = self.parse('xfail: known\ncall: X\nexpect:\n  1\n')
        self.assertEqual(s.xfail, 'known')

    def test_warns(self):
        s = self.parse('call: X\nwarns: best effort (not byte-identical): a b\n'
                       'warns: another one\nexpect:\n  1\n')
        self.assertEqual(s.warns, ['best effort (not byte-identical): a b', 'another one'])
        self.assertEqual(self.parse('call: X\nexpect:\n  1\n').warns, [])
        with self.assertRaises(exectest.SpecError):
            self.parse('call: X\nwarns:\nexpect:\n  1\n')

    def test_missing_warnings(self):
        s = self.parse('call: X\nwarns: odd shape\nwarns: other\nexpect:\n  1\n')
        err = 'm.pas:3:1: warning: best effort (not byte-identical): an odd shape is\n'
        self.assertEqual(exectest.missing_warnings(s, err), ['other'])
        self.assertEqual(exectest.missing_warnings(s, err + 'x: other\n'), [])

    def test_outcome(self):
        plain = self.parse('call: X\nexpect:\n  1\n')
        xf = self.parse('xfail: known\ncall: X\nwarns: w\nexpect:\n  1\n')
        R = exectest.Result
        self.assertEqual(exectest.outcome(plain, R(True, [], ran=True)), 'pass')
        self.assertEqual(exectest.outcome(plain, R(False, [], ran=True)), 'fail')
        self.assertEqual(exectest.outcome(xf, R(True, [], ran=True)), 'xpass')
        self.assertEqual(exectest.outcome(xf, R(False, [], ran=True)), 'xfail')
        # a missing `warns:' text stops the test before it runs: for an
        # xfail, a failure, not the expected one
        self.assertEqual(exectest.outcome(xf, R(False, [], 'dpc did not warn: w')), 'notrun')

    def test_refused(self):
        for h in ('expect:\n 1\n',                      # no call
                  'call: X\n',                          # no expect
                  'call: X w:3!\nexpect:\n',            # out on a value
                  'call: X q:3\nexpect:\n',             # bad size
                  'call: X -> w w:1\nexpect:\n',        # -> not last
                  'call: X -> z\nexpect:\n',            # bad kind
                  'call: X\nlink: a.pas\nexpect:\n',    # not .mod.pas
                  'call: X\nexpect: 1\n'):              # value on expect:
            with self.assertRaises(exectest.SpecError, msg=h):
                self.parse(h)

    def test_tests_parse(self):
        import glob
        for p in glob.glob(os.path.join(exectest.TESTDIR, '*.pas')):
            if p.endswith('.ins.pas') or p.endswith('.mod.pas'):
                continue
            exectest.parse_spec(p)


NEG = r'''
	.text
	.globl	CLOB
CLOB:	moveq	#1,%d2
	rts
	.globl	ODD
ODD:	lea	(0x10001).l,%a0
	move.w	(%a0),%d0
	rts
	.globl	ILL
ILL:	.short	0x49c0		| extb.l %d0: 68020 only
	rts
	.globl	WILD
WILD:	move.l	(0x200000).l,%d0
	rts
	.globl	POP
POP:	addq.l	#2,%sp
	rts
	.globl	ARGS
ARGS:	moveq	#0,%d0
	add.w	(4,%sp),%d0	| w:3 (pushed last)
	moveq	#0,%d1
	move.b	(6,%sp),%d1	| b:2: the high byte of its slot
	add.w	%d1,%d0
	movea.l	(8,%sp),%a0	| @l:7!
	add.l	(%a0),%d0
	move.l	%d0,(%a0)
	rts
	.globl	LOG
LOG:	pea	(cell,%pc)
	jsr	(EXEC_$LOG_L).l
	addq.w	#4,%sp
	rts
cell:	.long	-9
	.globl	NIL
NIL:	suba.l	%a0,%a0
	move.l	(8,%a0),%d0	| a nil pointer: reads the vector table
	rts
	.globl	CODEW
CODEW:	lea	(cell,%pc),%a0
	move.l	%d0,(%a0)	| a store into the code section
	rts
	.globl	STUBWILD
STUBWILD:	pea	(0x20).w	| a "pointer" into low memory
	jsr	(EXEC_$LOG_L).l
	addq.w	#4,%sp
	rts
	.globl	STUBODD
STUBODD:	pea	(cell+1,%pc)
	jsr	(EXEC_$LOG_W).l
	addq.w	#4,%sp
	rts
	.globl	DIV0
DIV0:	moveq	#0,%d1
	divs.w	%d1,%d0
	rts
	.globl	CHK
CHK:	moveq	#-1,%d0
	chk.w	#5,%d0
	rts
	.globl	TRAPV
TRAPV:	move.w	#2,%ccr
	trapv
	rts
	.globl	TRAP
TRAP:	trap	#3
	rts
	.globl	LINEA
LINEA:	.short	0xa000
	rts
	.globl	LINEF
LINEF:	.short	0xf000
	rts
	.globl	CCR010
CCR010:	move.w	%ccr,%d0	| 68010 and later: illegal on a 68000
	moveq	#7,%d0
	rts
	.globl	SPIN
SPIN:	bra.s	SPIN
	.globl	RECUR
RECUR:	bsr.s	RECUR
	.globl	IPL
IPL:	ori.w	#0x700,%sr
	rts
	.globl	PADW
PADW:	move.w	(4,%sp),%d0	| w:5, then the pad (filled with 0xA5)
	swap	%d0
	move.w	(6,%sp),%d0
	rts
	.globl	BYTELO
BYTELO:	move.w	(4,%sp),%d0	| b:0x12: low byte is not 0 and not 0x12
	rts
	.globl	CLOBREGS
CLOBREGS:	move.l	%d2,-(%sp)
	moveq	#0,%d0		| the stub's junk is never 0 in either half
	moveq	#0,%d1
	movea.l	%d0,%a0
	movea.l	%d0,%a1
	move.w	#0,%ccr		| the stub's junk CCR always has C set
	jsr	(EXEC_$CLOBBER).l
	move.w	%sr,%d2
	movem.l	%d0-%d1/%a0-%a1,cell2
	moveq	#0,%d0		| bit n: register n (D0 D1 A0 A1 CCR) kept
	tst.l	cell2
	bne.s	1f
	ori.w	#1,%d0
1:	tst.l	cell2+4
	bne.s	2f
	ori.w	#2,%d0
2:	tst.l	cell2+8
	bne.s	3f
	ori.w	#4,%d0
3:	tst.l	cell2+12
	bne.s	4f
	ori.w	#8,%d0
4:	andi.w	#0x1f,%d2
	bne.s	5f
	ori.w	#16,%d0
5:	move.l	(%sp)+,%d2
	rts
	.globl	IDW
IDW:	pea	(cell+2,%pc)
	jsr	(EXEC_$ID_W).l	| the low word of -9: 0xFFF7
	addq.w	#4,%sp
	rts
	.data
	.globl	cell2
cell2:	.space	20
'''


@unittest.skipIf(exectest.missing_prereqs(), 'exectest prerequisites missing')
class RunnerTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.runner = exectest.build_runner()
        cls.tmp = tempfile.TemporaryDirectory()
        d = cls.tmp.name
        src = os.path.join(d, 'neg.s')
        with open(src, 'w') as f:
            f.write(NEG)
        obj = os.path.join(d, 'neg.o')
        elf = os.path.join(d, 'neg.elf')
        cls.bin = os.path.join(d, 'neg.bin')
        for cmd in ([exectest.AS, '-m68010', src, '-o', obj],
                    [exectest.LD, '-T', os.path.join(exectest.TOOLDIR, 'link.ld'),
                     '-o', elf, obj, os.path.join(exectest.BUILD, 'stubs.o')],
                    [exectest.OBJCOPY, '-O', 'binary', elf, cls.bin]):
            subprocess.run(cmd, check=True, capture_output=True)
        cls.syms = exectest.symbols(elf)

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def call(self, spec):
        name, *rest = spec.split()
        cmd = [self.runner, '--bin', self.bin, '--base', hex(exectest.BASE),
               '--end', hex(self.syms['_end']),
               '--text-end', hex(self.syms['_etext'])]
        for n, a in self.syms.items():
            if n.startswith('EXEC_$'):
                cmd += ['--stub', '%s=%s' % (n, hex(a))]
        cmd += ['--call', ' '.join([hex(self.syms[name])] + rest)]
        p = subprocess.run(cmd, capture_output=True, text=True)
        return p.returncode, p.stdout

    def test_clobbered_d2(self):
        rc, out = self.call('CLOB')
        self.assertEqual(rc, 1)
        self.assertIn('D2 not preserved', out)

    def test_odd_word_access(self):
        rc, out = self.call('ODD')
        self.assertEqual(rc, 1)
        self.assertIn('address error', out)
        self.assertIn('0x010001', out)

    def test_68020_instruction(self):
        rc, out = self.call('ILL')
        self.assertEqual(rc, 1)
        self.assertIn('illegal instruction', out)

    def test_wild(self):
        rc, out = self.call('WILD')
        self.assertEqual(rc, 1)
        self.assertIn('wild read.l at 0x200000', out)

    def test_bad_return(self):
        rc, out = self.call('POP')
        self.assertEqual(rc, 1)

    def test_args(self):
        rc, out = self.call('ARGS w:3 b:2 @l:7! -> l')
        self.assertEqual((rc, out), (0, 'out 12\n=> 12\n'))

    def test_stub(self):
        rc, out = self.call('LOG')
        self.assertEqual((rc, out), (0, '-9\n'))


    def fails(self, spec, msg):
        rc, out = self.call(spec)
        self.assertEqual(rc, 1, out)
        self.assertIn(msg, out)
        return out

    def test_nil_pointer(self):
        self.fails('NIL', 'wild read.l at 0x000008')

    def test_code_write(self):
        self.fails('CODEW', 'wild write.l')

    def test_stub_wild_pointer(self):
        self.fails('STUBWILD', 'EXEC_$LOG_L: argument address 0x00000020 is wild')

    def test_stub_odd_pointer(self):
        self.fails('STUBODD', 'is odd')

    def test_exceptions(self):
        for name, msg in (('DIV0', 'zero divide (vector 5)'),
                          ('CHK', 'CHK (vector 6)'),
                          ('TRAPV', 'TRAPV (vector 7)'),
                          ('TRAP', 'TRAP (vector 35)'),
                          ('LINEA', 'A-line'),
                          ('LINEF', 'F-line')):
            with self.subTest(name):
                out = self.fails(name, msg)
                self.assertNotIn('wild', out)   # the vector fetch is not it

    def test_cpu_is_68010(self):
        # move from CCR is a 68010 instruction (illegal on a 68000);
        # extb.l is 68020-only (test_68020_instruction).
        self.assertEqual(self.call('CCR010 -> l'), (0, '=> 7\n'))

    def test_instruction_cap(self):
        self.fails('SPIN', 'no return after 2000000 instructions')

    def test_stack_overflow(self):
        self.fails('RECUR', 'stack overflow')

    def test_interrupt_mask(self):
        self.fails('IPL', 'interrupt mask 7')

    def test_pad_after_word(self):
        # a 2-byte slot is padded to 4: the pad (0xA5 fill) above it
        self.assertEqual(self.call('PADW w:5 -> l'), (0, '=> %d\n' % 0x0005a5a5))

    def test_byte_in_high_half(self):
        self.assertEqual(self.call('BYTELO b:0x12 -> uw'), (0, '=> %d\n' % 0x12a5))

    def test_stub_clobbers(self):
        # D0, D1, A0, A1 and the CCR all changed by EXEC_$CLOBBER
        self.assertEqual(self.call('CLOBREGS -> l'), (0, '=> 0\n'))

    def test_id_w_junk_above(self):
        rc, out = self.call('IDW -> l')
        self.assertEqual(rc, 0, out)
        v = int(out.split()[1])
        self.assertEqual(v & 0xffff, 0xfff7)
        self.assertNotEqual(v & 0xffff0000, 0xffff0000)   # not sign-extended
        self.assertNotEqual(v & 0xffff0000, 0)            # not zero-extended
        self.assertEqual(self.call('IDW -> w'), (0, '=> -9\n'))


if __name__ == '__main__':
    unittest.main()
