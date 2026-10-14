"""Tests for tools/srcsurvey.py (no image, dpc or ../source needed)."""

import os
import re
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..'))
import srcsurvey  # noqa: E402

MODULE = """\
{ a module file }
module m;

%include 'base.ins.pas';

define
  m_$glob;
  m_$other;

var
  x: extern integer32;

{ body: m/late.pas }
procedure late(var t: integer); forward; internal;

%include 'm/frag.pas';                { two routines }
%include 'm/missing.pas';             { routine 0xE00000,
                                        nested 0xE00010 }
%include 'm/late.pas';
"""

FRAG = """\
{ m_$glob - a global.

  Original address: 0x00E1C084, 98 bytes, m.pas, section WIRED_PROC.

  A heading in a comment does not count:
  procedure fake;
}

[section(wired_proc, wired_data)]
procedure m_$glob(in line: integer; out status: status_$t);
var
  p: ^procedure(i: integer); options(val_param);
  r: record
    case integer of
      1: (a: integer);
      2: (b: integer32);
  end;

  { nested }
  procedure helper;
  begin
    if line = 1 then begin
      case line of
        1: x := 2;
      end;
    end;
  end;

begin
  helper;
  late(line);
end;

{ second.

  Original address: 0x00E1C0E6, 30 bytes, m.pas.
}
[section(wired_proc, wired_data)]
function m_$local(c: char): integer;
  internal;
begin
  m_$local := 0;
end;
"""


class PrepareTest(unittest.TestCase):
    def test_missing_fragments_and_forward(self):
        have = {'base.ins.pas', 'm/frag.pas'}
        text, dropped = srcsurvey.prepare_text(MODULE, lambda p: p in have)
        self.assertEqual(dropped, ['m/missing.pas', 'm/late.pas'])
        self.assertNotIn('missing.pas', text)
        self.assertNotIn('nested 0xE00010', text)          # the comment's tail
        self.assertNotIn('{ body:', text)
        self.assertIn('procedure late(var t: integer); extern; internal;', text)
        self.assertIn("%include 'm/frag.pas';", text)

    def test_complete_module_unchanged(self):
        text, dropped = srcsurvey.prepare_text(MODULE, lambda p: True)
        self.assertEqual((text, dropped), (MODULE, []))

    def test_module_includes(self):
        self.assertEqual(srcsurvey.module_includes(MODULE),
                         ['base.ins.pas', 'm/frag.pas', 'm/missing.pas', 'm/late.pas'])


class RoutineTest(unittest.TestCase):
    def test_parse(self):
        rs = srcsurvey.parse_routines(FRAG)
        self.assertEqual([r.name for r in rs], ['m_$glob', 'm_$local'])
        g, l = rs
        self.assertEqual([n.name for n in g.nested], ['helper'])
        self.assertEqual((g.addr, g.size), (0xE1C084, 98))
        self.assertEqual((l.addr, l.size), (0xE1C0E6, 30))
        self.assertFalse(g.internal)
        self.assertTrue(l.internal)
        self.assertTrue(FRAG[g.start:].startswith('[section'))
        self.assertTrue(FRAG[:g.end].endswith('late(line);\nend;'))
        self.assertTrue(FRAG[:l.end].endswith('m_$local := 0;\nend;'))

    def test_spans(self):
        spans = srcsurvey.routine_spans(FRAG)
        self.assertEqual([n for _a, _b, n in spans], ['m_$glob', 'm_$local'])
        a, b, _n = spans[0]
        self.assertEqual(FRAG.splitlines()[a - 1], '[section(wired_proc, wired_data)]')
        self.assertEqual(FRAG.splitlines()[b - 1], 'end;')

    def test_extern_heading_has_no_body(self):
        rs = srcsurvey.parse_routines('procedure a(x: integer); internal; extern;\n'
                                      'procedure b; begin end;\n')
        self.assertEqual([(r.name, r.has_body) for r in rs], [('a', False), ('b', True)])

    def test_stub(self):
        text, deleted = srcsurvey.stub_text(FRAG, {'m_$glob', 'm_$local'})
        self.assertEqual(deleted, ['m_$glob'])
        self.assertNotIn('helper', text)
        self.assertIn('function m_$local(c: char): integer;\n  internal; extern;', text)
        self.assertEqual(text.count('[section'), 0)
        self.assertIn('Original address: 0x00E1C0E6', text)    # header kept
        rs = srcsurvey.parse_routines(text)
        self.assertEqual([(r.name, r.has_body, r.addr) for r in rs],
                         [('m_$local', False, 0xE1C0E6)])

    def test_stub_with_forward_heading(self):
        # the module file has `forward; internal;', the fragment repeats
        # the heading with the body: stubbing must not redeclare it
        mod = "{ body: m/f.pas }\nprocedure f(x: integer); forward; internal;\n"
        frag = "procedure f(x: integer);\n  internal;\nbegin\nend;\n"
        text, deleted = srcsurvey.stub_text(frag, {'f'}, {'f'})
        self.assertEqual((text.strip(), deleted), ('', []))
        m2 = srcsurvey.extern_forwards(mod, {'f'})
        self.assertIn('procedure f(x: integer); extern; internal;', m2)
        self.assertEqual(srcsurvey.extern_forwards(mod, {'g'}), mod)

    def test_drop_defines(self):
        t = srcsurvey.drop_defines(MODULE, ['M_$GLOB'])
        self.assertIn('define\n  m_$other;\n\nvar', t)
        t = srcsurvey.drop_defines(t, ['m_$other'])
        self.assertNotIn('define', t)
        self.assertIn('\nvar\n  x: extern integer32;', t)


class DiagTest(unittest.TestCase):
    ERR = """\
m/frag.pas:27:3: error: this operator on sets is not supported yet
27 |   ...
note: in file included from m.pas:3:1
m/frag.pas:40:3: error: the `aligned` attribute (ref 3.15) is not supported yet
m/frag.pas:28:3: error: call of internal routine `M_$X`, which has no body (ref 5.2 p. 5-4)
m/frag.pas:29:3: error: call of internal routine `M_$Y`, which has no body (ref 5.2 p. 5-4)
base.ins.pas:12:1: warning: something 16#10 happened 3 times
dpc: error: not a diagnostic line
"""

    def test_parse_and_kinds(self):
        ds = srcsurvey.parse_diags(self.ERR)
        self.assertEqual(len(ds), 5)
        self.assertEqual((ds[0]['file'], ds[0]['line'], ds[0]['col'], ds[0]['sev']),
                         ('m/frag.pas', 27, 3, 'error'))
        self.assertEqual(srcsurvey.kind_of(ds[1]['msg']),
                         'the `aligned` attribute (ref 3.15) is not supported yet')
        self.assertEqual(srcsurvey.kind_of(ds[2]['msg']),
                         'call of internal routine `X`, which has no body (ref 5.2 p. 5-4)')
        self.assertEqual(srcsurvey.kind_of(ds[4]['msg']), 'something N happened N times')
        self.assertEqual([srcsurvey.classify(d) for d in ds],
                         ['refusal', 'refusal', 'error', 'error', 'warning'])

    def test_summarise_with_locator(self):
        with tempfile.TemporaryDirectory() as tree:
            os.makedirs(os.path.join(tree, 'm'))
            with open(os.path.join(tree, 'm', 'frag.pas'), 'w') as f:
                f.write(FRAG)
            loc = srcsurvey.Locator(tree)
            ds = srcsurvey.parse_diags(self.ERR)
            entries = [(d, loc(d['file'], d['line'])) for d in ds]
        self.assertEqual([w for _d, w in entries],
                         ['m_$glob', 'm_$local', 'm_$glob', 'm_$glob', None])
        refusals, errors, warnings = srcsurvey.summarise(entries)
        self.assertEqual([(k, n) for k, n, _a, _e in refusals],
                         [('the `aligned` attribute (ref 3.15) is not supported yet', 1),
                          ('this operator on sets is not supported yet', 1)])
        self.assertEqual(errors[0][1:3], (2, [('m_$glob', 2)]))
        self.assertEqual(warnings[0][2], [('base.ins.pas', 1)])


class WorkaroundTest(unittest.TestCase):
    # The rewrite mechanism, with the pattern of the former
    # `field-aligned' workaround (dpc now takes alignment attributes, bead
    # domain-pascal-l9p).
    PAT = re.compile(r'(:\s*)\[\s*aligned(?:\s*\(\s*\d+\s*\))?\s*\]\s*', re.I)

    def apply(self, text):
        return srcsurvey.apply_workaround(self.PAT, r'\1', text)

    def test_rewrite(self):
        self.assertEqual(self.apply('timer: [aligned(1)] linteger;')[0], 'timer: linteger;')

    def test_no_attribute_workarounds(self):
        # dpc takes packed records (ref 3.10.7, bead domain-pascal-xlm)
        # and alignment attributes (ref 3.15.6, bead domain-pascal-l9p)
        names = [n for n, *_ in srcsurvey.WORKAROUNDS]
        self.assertNotIn('packed-record', names)
        self.assertNotIn('packed-overlay', names)
        self.assertNotIn('field-aligned', names)

    def test_comments_and_strings_untouched(self):
        # only the real declaration is rewritten and counted, not the
        # same text in a comment or a string
        src = ("{ customer: timer: [aligned(1)] linteger }\n"
               "(* timer: [aligned(1)] linteger *) \"timer: [aligned(1)]\" s = ': [aligned(1)] ';\n"
               "timer: [aligned(1)] linteger;\n")
        text, k = self.apply(src)
        self.assertEqual(k, 1)
        self.assertEqual(text, src.replace('timer: [aligned(1)] linteger;\n',
                                           'timer: linteger;\n'))


class DataSectionsTest(unittest.TestCase):
    """--section-addr pairs for dpc's data blocks, zero-filled
    (`.bss.moddata.') or with initial values (`.moddata.')."""

    class Seg:
        def __init__(self, kind, name, addr, size):
            self.kind, self.name, self.addr, self.size = kind, name, addr, size

        def contains(self, a):
            return self.addr <= a < self.addr + self.size

    def test_both_prefixes(self):
        S = self.Seg
        lm = type('M', (), {})()
        lm.sections = [S('D', 'WIRED_DATA', 0xE1DC00, 0x18000),
                       S('D', '.DATA', 0xE78400, 0xD308)]
        lm.segments = [S('D', 'TTY', 0xE2DDB4, 0x28), S('D', 'TTY', 0xE8242C, 0x2C)]
        asm = ('\t.section\t".moddata.TTY.WIRED_DATA","aw",@progbits\n'
               '\t.section\t".bss.moddata.TTY","aw",@nobits\n')
        self.assertEqual(srcsurvey.data_sections(asm, lm, 'tty'),
                         {'.moddata.TTY.WIRED_DATA': 0xE2DDB4,
                          '.bss.moddata.TTY': 0xE8242C})


if __name__ == '__main__':
    unittest.main()
