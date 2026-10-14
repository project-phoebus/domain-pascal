"""Tests for tools/corpus.py (the front-end corpus test).

The decoding, classification and baseline handling need nothing; the end
to end test runs target/debug/dpc on a tiny made-up corpus and is skipped
when dpc is not built.  The real corpus runs under `make corpus' (part of
`make test').
"""

import io
import os
import sys
import tempfile
import unittest
from contextlib import redirect_stdout

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..'))
import corpus  # noqa: E402

HDR = corpus.HEADER_MAGIC + bytes(corpus.HEADER_SIZE - 4)


def rec(*lines):
    """A `rec' file body: count (including itself), data, even pad."""
    out = b''
    for line in lines:
        n = 4 + len(line)
        out += n.to_bytes(4, 'big') + line + (b'\0' if n & 1 else b'')
    return out


class DecodeTest(unittest.TestCase):
    def test_plain(self):
        self.assertEqual(corpus.decode(b'{ x }\n'), ('plain', b'{ x }\n'))

    def test_uasc(self):
        self.assertEqual(corpus.decode(HDR + b'module m;\n'),
                         ('uasc', b'module m;\n'))

    def test_rec(self):
        body = rec(b'\n', b'MODULE m;\n', b'{ odd }\n')
        self.assertEqual(corpus.decode(HDR + body),
                         ('rec', b'\nMODULE m;\n{ odd }\n'))

    def test_rec_must_parse_exactly(self):
        self.assertIsNone(corpus.decode_rec(b'\0\0\0\x10abc'))
        self.assertIsNone(corpus.decode_rec(b'\0\0\0\x02'))
        self.assertIsNone(corpus.decode_rec(b'\0\0'))
        # Text after a header is not records: uasc.
        self.assertEqual(corpus.decode(HDR + b'{ a }')[0], 'uasc')


class ClassifyTest(unittest.TestCase):
    def test_errors(self):
        err = ('t.pas:1:2: error: undefined type `foo_$t`\n'
               '1 | x\n'
               'dpc: error: cannot read /a/b.pas: nope\n'
               't.pas:3:1: warning: w\n')
        self.assertEqual(corpus.errors(err), [
            'undefined type `foo_$t`', 'cannot read /a/b.pas: nope'])

    def test_kind(self):
        self.assertEqual(corpus.kind('undefined type `foo_$t`'),
                         'undefined type `_`')
        self.assertEqual(
            corpus.kind('functions returning s_t are not supported yet '
                        '(codegen-patterns 3: x)'),
            'functions returning T are not supported yet '
            '(codegen-patterns N: x)')
        self.assertEqual(
            corpus.kind('argument type ^a does not match parameter type b'),
            'argument type T does not match parameter type T')
        self.assertEqual(corpus.kind('constant 5 is out of range for x_t'),
                         'constant N is out of range for T')

    def test_wrapper(self):
        w = corpus.wrapper('ctm.ins.pas')
        self.assertEqual(w.splitlines(), [
            'module corpus_check;',
            "%include '/sys/ins/base.ins.pas';",
            "%include '/sys/ins/gpr.ins.pas';",
            "%include '/sys/ins/ctm.ins.pas';"])
        self.assertEqual(corpus.wrapper('base.ins.pas').count('%include'), 1)


class BaselineTest(unittest.TestCase):
    def test_round_trip_and_compare(self):
        res = {'sys/ins/a.ins.pas': ('ok', []),
               'x/p.pas': ('diag', ['undefined type `q`', 'other'])}
        text = corpus.format_baseline(res)
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, 'b.txt')
            with open(p, 'w') as f:
                f.write(text)
            base = corpus.read_baseline(p)
        self.assertEqual(base, {'sys/ins/a.ins.pas': ('ok', ''),
                                'x/p.pas': ('diag', 'undefined type `_`')})
        self.assertEqual(corpus.compare(base, res), [])
        res2 = dict(res)
        res2['x/p.pas'] = ('ok', [])
        del res2['sys/ins/a.ins.pas']
        res2['y.pas'] = ('internal', ['boom'])
        self.assertEqual(corpus.compare(base, res2), [
            'x/p.pas: was diag undefined type `_`, now ok ',
            'new entry: internal y.pas boom',
            'missing entry: sys/ins/a.ins.pas'])


class MainTest(unittest.TestCase):
    def test_skips_without_corpus(self):
        with tempfile.TemporaryDirectory() as d:
            out = io.StringIO()
            with redirect_stdout(out):
                rc = corpus.main(['--corpus', os.path.join(d, 'none')])
        self.assertEqual(rc, 0)
        self.assertIn('skipped', out.getvalue())

    @unittest.skipUnless(os.path.exists(corpus.DPC), 'dpc not built')
    def test_end_to_end(self):
        with tempfile.TemporaryDirectory() as d:
            c = os.path.join(d, 'install')
            os.makedirs(os.path.join(c, 'sys', 'ins'))
            os.makedirs(os.path.join(c, 'ex'))
            with open(os.path.join(c, 'sys', 'ins', 'base.ins.pas'),
                      'wb') as f:
                f.write(HDR + b'type b_t = integer32;\n')
            with open(os.path.join(c, 'sys', 'ins', 'bad.ins.pas'),
                      'wb') as f:
                f.write(HDR + rec(b'type c_t = nope_t;\n'))
            with open(os.path.join(c, 'ex', 'p.pas'), 'w') as f:
                f.write("module p;\n%include '/sys/ins/base.ins.pas';\n"
                        "var v: b_t;\n")
            base = os.path.join(d, 'baseline.txt')
            old = corpus.WORK
            corpus.WORK = os.path.join(d, 'work')
            try:
                out = io.StringIO()
                with redirect_stdout(out):
                    rc = corpus.main(['--corpus', c, '--baseline', base,
                                      '--update'])
                self.assertEqual(rc, 0, out.getvalue())
                self.assertIn('ok 2, diag 1, internal 0, timeout 0',
                              out.getvalue())
                self.assertEqual(corpus.read_baseline(base), {
                    'sys/ins/base.ins.pas': ('ok', ''),
                    'sys/ins/bad.ins.pas': ('diag', 'undefined type `_`'),
                    'ex/p.pas': ('ok', '')})
                with redirect_stdout(io.StringIO()):
                    self.assertEqual(
                        corpus.main(['--corpus', c, '--baseline', base]), 0)
                with open(os.path.join(c, 'ex', 'p.pas'), 'a') as f:
                    f.write('var w: nope_t;\n')
                out = io.StringIO()
                with redirect_stdout(out):
                    rc = corpus.main(['--corpus', c, '--baseline', base])
                self.assertEqual(rc, 1)
                self.assertIn('ex/p.pas: was ok , now diag', out.getvalue())
            finally:
                corpus.WORK = old


if __name__ == '__main__':
    unittest.main()
