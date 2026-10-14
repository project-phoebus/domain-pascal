"""Tests for tools/srcsurvey.py (no image, dpc or ../source needed)."""

import io
import os
import re
import sys
import tempfile
import unittest
import unittest.mock

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


class PackedInitTest(unittest.TestCase):
    # KBD's kbd_$state_table: dpc refuses initial values of 4-bit packed
    # fields; the initialiser goes, the declaration stays
    def test_drop_initialiser(self):
        wa = {n: (p, r) for n, _w, _s, p, r in srcsurvey.WORKAROUNDS}
        pat, rep = wa['packed-init']
        src = ("{ kbd_$state_table: array[0..1] of kbd_$state_entry_t := [ }\n"
               "  kbd_$state_table: array[0..1] of kbd_$state_entry_t := [\n"
               "    [key := chr(16#FB), action := 2, next_state := 0],\n"
               "    [key := chr(16#E8), action := 3, next_state := 1]];\n"
               "  other: array[0..1] of integer := [1, 2];\n")
        text, k = srcsurvey.apply_workaround(pat, rep, src)
        self.assertEqual(k, 1)
        self.assertEqual(text.splitlines()[1:],
                         ['  kbd_$state_table: array[0..1] of kbd_$state_entry_t;',
                          '  other: array[0..1] of integer := [1, 2];'])
        self.assertIn('packed-init', srcsurvey.DATA_WORKAROUNDS)

    def test_code_ok_only(self):
        f = srcsurvey.code_ok_only
        head = 'FAIL r (R, 0x10..0x20, 0 relocation(s))'
        listing = ['          ours      | image', '  e1ce94  4e75   rts  | 4e75  rts']
        data = '  section .moddata.KBD.WIRED_DATA differs first at 0xE2DEA4: ours 00, image fb'
        self.assertTrue(f([head, data] + listing))
        self.assertFalse(f([head, '  bytes differ first at 0xE1CEB4 (+0x8): ours 24, image 26',
                            data] + listing))
        self.assertFalse(f([head] + listing))

    def test_within_workaround_ranges(self):
        # CODE-OK needs every differing data byte inside kbd_$state_table
        # (the last 0x14 bytes of KBD's block, 0xE2DDE4 size 0xD4), not
        # just the first one conform.check reports
        f = srcsurvey.within
        self.assertTrue(f([0xE2DEA4, 0xE2DEB7], ['packed-init']))
        self.assertFalse(f([0xE2DEA4, 0xE2DEB8], ['packed-init']))
        self.assertFalse(f([0xE2DEA3], ['packed-init']))

    def test_data_diffs_all_bytes(self):
        import conform
        ours = bytes([1, 2, 3, 4, 5, 6])
        img = bytes([1, 9, 3, 4, 9, 6])

        def run(cmd, cwd=None):
            if cmd[0] == conform.OBJCOPY:
                self.assertEqual(cmd[cmd.index('-j') + 1], '.moddata.K.D')
                with open(cmd[-1], 'wb') as fh:
                    fh.write(ours)
            return ''
        secs = [('.moddata.K.D', 6, {'CONTENTS', 'ALLOC', 'DATA'}),
                ('.bss.moddata.K', 4, {'ALLOC'}),
                ('.text', 2, {'CONTENTS', 'ALLOC', 'CODE'})]
        with tempfile.TemporaryDirectory() as d, \
                unittest.mock.patch.object(conform, 'run', run), \
                unittest.mock.patch.object(conform, 'sections_of', lambda o: secs), \
                unittest.mock.patch.object(conform, 'read_image',
                                           lambda a, n: img[:n]):
            got = srcsurvey.data_diffs(os.path.join(d, 'u.s'),
                                       {'.moddata.K.D': 0x1000, '.bss.moddata.K': 0x2000}, d)
        self.assertEqual(got, [0x1001, 0x1004])


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
        self.assertEqual(srcsurvey.data_sections(asm, lm),
                         {'.moddata.TTY.WIRED_DATA': 0xE2DDB4,
                          '.bss.moddata.TTY': 0xE8242C})

    def test_module_name_not_file_name(self):
        # log/log.pas is `module log_', the map's LOG_ (D 0xE2B280)
        S = self.Seg
        lm = type('M', (), {})()
        lm.sections = [S('D', 'WIRED_DATA', 0xE1DC00, 0x18000)]
        lm.segments = [S('D', 'LOG_', 0xE2B280, 0x1C), S('D', 'LOG', 0xE2B000, 0x1C)]
        asm = '\t.section\t".bss.moddata.LOG_.WIRED_DATA","aw",@nobits\n'
        self.assertEqual(srcsurvey.data_sections(asm, lm),
                         {'.bss.moddata.LOG_.WIRED_DATA': 0xE2B280})



class ModuleRoutineOptsTest(unittest.TestCase):
    """The dpc options for a module's stubbed internal routines: one
    fragment origin for a module of one code section, one per section
    (with each routine's section) for several (bead domain-pascal-xdh.6)."""

    Seg = DataSectionsTest.Seg

    def mod(self, routines):
        m = type('Mod', (), {})()
        m.routines = {}
        for name, addr, internal in routines:
            r = srcsurvey.Routine(name)
            r.addr = addr
            if internal:
                r.options.add('internal')
            m.routines[name] = ('frag.pas', r)
        return m

    def linkmap(self):
        lm = type('M', (), {})()
        lm.sections = [self.Seg('I', 'WIRED_PROC', 0xE00800, 0x1D0F4),
                       self.Seg('I', 'OS_INIT_PROC', 0xE2F000, 0x5F14),
                       self.Seg('I', '.TEXT', 0xE38000, 0x39D7C)]
        return lm

    def test_one_section(self):
        m = self.mod([('a', 0xE1AED0, True), ('b', 0xE1AEE4, True), ('c', 0xE1AEF8, False)])
        f = srcsurvey.module_routine_opts
        self.assertEqual(f(m, [], self.linkmap()), [])
        self.assertEqual(f(m, ['c'], self.linkmap()), [])          # global: deleted
        self.assertEqual(f(m, ['b'], self.linkmap()),
                         ['--fragment-origin', '0xE1AED0', '--module-routine=B=0xE1AEE4'])
        # no map: as one section
        self.assertEqual(f(m, ['b'], None),
                         ['--fragment-origin', '0xE1AED0', '--module-routine=B=0xE1AEE4'])

    def test_several_sections(self):
        m = self.mod([('lock', 0xE1AED0, True), ('put', 0xE1AF0A, False),
                      ('init', 0xE3324C, False),
                      ('dfl', 0xE6726E, True), ('reset', 0xE672DE, False)])
        self.assertEqual(
            srcsurvey.module_routine_opts(m, ['lock', 'dfl', 'init'], self.linkmap()),
            ['--fragment-origin=.TEXT=0xE672DE', '--fragment-origin=WIRED_PROC=0xE1AF0A',
             '--module-routine=WIRED_PROC:LOCK=0xE1AED0',
             '--module-routine=.TEXT:DFL=0xE6726E'])


STATICS = """\
[section(wired_proc, wired_data)]
procedure a(x: integer);
var
  f: integer;                            { frame }
  seps: static set of char := [' ', chr(9)];
  r: static record
    case integer of
      1: (w: integer);
  end;
  g, h: static integer32 := 0;

  procedure inner;
  var
    k: static array[1..2] of integer := [1, 2];
    j: integer;
  begin
  end;

var
  e: extern integer;
  d: define integer;
begin
  inner;
end;

procedure b; internal;
var
  { .DATA +0x00 }
  tbl: static array[0..1] of integer := [
    3,                                   { a; b }
    4];
begin
end;
"""


class StaticsTest(unittest.TestCase):
    """Stubbing keeps a routine's static data (bead domain-pascal-xdh.7)."""

    def test_routine_statics(self):
        a, b = srcsurvey.parse_routines(STATICS)
        sts = srcsurvey.routine_statics(STATICS, a)
        self.assertEqual([(s.routine, s.names, s.section, s.kind) for s in sts],
                         [('a', ['seps'], 'wired_data', 'static'),
                          ('a', ['r'], 'wired_data', 'static'),
                          ('a', ['g', 'h'], 'wired_data', 'static'),
                          ('a_inner', ['k'], 'wired_data', 'static'),
                          ('a', ['d'], 'wired_data', 'define')])
        self.assertEqual(sts[0].rest, ": static set of char := [' ', chr(9)];")
        self.assertTrue(sts[1].rest.endswith('end;'))
        (t,) = srcsurvey.routine_statics(STATICS, b)
        self.assertEqual((t.names, t.section), (['tbl'], None))
        self.assertIn('4];', t.rest)

    def test_hoisted_text(self):
        a, b = srcsurvey.parse_routines(STATICS)
        h = srcsurvey.hoisted_text(srcsurvey.routine_statics(STATICS, a))
        self.assertIn("\nvar (wired_data)\n  a_seps: static set of char := [' ', chr(9)];\n", h)
        self.assertIn('a_g, a_h: static integer32 := 0;', h)
        self.assertIn('a_inner_k: static array', h)   # the enclosing routine's too
        self.assertIn('\n  d: define integer;', h)          # exported: its name
        self.assertNotIn(' f:', h)
        self.assertNotIn(' e:', h)
        # a section no routine compiled has is left out; the module's stays
        self.assertEqual(srcsurvey.hoisted_text(srcsurvey.routine_statics(STATICS, a),
                                                set()), '')
        self.assertTrue(srcsurvey.hoisted_text(srcsurvey.routine_statics(STATICS, b),
                                               set()).startswith('\nvar\n  b_tbl: static'))

    def test_stub_keeps_statics_in_place(self):
        text, deleted = srcsurvey.stub_text(STATICS, {'a', 'b'}, keep_sections={'wired_data'})
        self.assertEqual(deleted, ['a'])
        self.assertEqual(srcsurvey.parse_routines(text)[0].name, 'b')
        self.assertLess(text.index('a_seps'), text.index('a_r'))
        self.assertLess(text.index('inner_k'), text.index('procedure b'))
        self.assertLess(text.index('procedure b'), text.index('b_tbl'))
        self.assertNotIn('begin', text)

    def test_procedure_type_var_parameter(self):
        # a `var' parameter of a procedure type is no `var' part
        text = ('procedure p;\ntype\n  f_t = ^procedure(var x: integer);\n'
                'var\n  s: static integer := 1;\nbegin\nend;\n')
        (p,) = srcsurvey.parse_routines(text)
        self.assertEqual([st.names for st in srcsurvey.routine_statics(text, p)], [['s']])

    def test_same_nested_names(self):
        # two routines' nested routines of one name: distinct prefixes
        text = ''.join('procedure %s;\n  procedure inner;\n  var k: static integer;\n'
                       '  begin\n  end;\nbegin\nend;\n' % n for n in 'pq')
        p, q = srcsurvey.parse_routines(text)
        h = (srcsurvey.hoisted_text(srcsurvey.routine_statics(text, p))
             + srcsurvey.hoisted_text(srcsurvey.routine_statics(text, q)))
        self.assertIn('p_inner_k', h)
        self.assertIn('q_inner_k', h)

    def test_hoisted_owner_and_no_hoist(self):
        text, _d = srcsurvey.stub_text(STATICS, {'a', 'b'}, keep_sections={'wired_data'})
        lines = text.splitlines()
        at = {n: lines.index(next(x for x in lines if n in x)) + 1
              for n in ('a_seps', 'a_inner_k', 'b_tbl', 'procedure b')}
        self.assertEqual(srcsurvey.hoisted_owner(text, at['a_seps']), 'a')
        self.assertEqual(srcsurvey.hoisted_owner(text, at['a_inner_k']), 'a')
        self.assertEqual(srcsurvey.hoisted_owner(text, at['b_tbl']), 'b')
        self.assertIsNone(srcsurvey.hoisted_owner(text, at['procedure b']))
        text, _d = srcsurvey.stub_text(STATICS, {'a', 'b'}, keep_sections={'wired_data'},
                                       no_hoist={'a'})
        self.assertNotIn('a_seps', text)
        self.assertIn('b_tbl', text)

    def test_survey_stage_drops_statics_that_fail_hoisted(self):
        # a static whose type is the routine's own: refused at unit level,
        # so the routine's statics are dropped (not the module's survey)
        fake = ('#!%s\nimport sys\nsrc = sys.argv[-1]\ntext = open("m/f.pas").read()\n'
                'if "p_a" in text:\n'
                '    n = text.splitlines().index("  p_a: static t := [n, 1];") + 1\n'
                '    sys.stderr.write("m/f.pas:%%d:15: error: undefined type `t`\\n" %% n)\n'
                '    sys.exit(1)\n' % sys.executable)
        with tempfile.TemporaryDirectory() as d:
            os.makedirs(os.path.join(d, 'm'))
            with open(os.path.join(d, 'm.pas'), 'w') as fh:
                fh.write("module m;\n%include 'm/f.pas';\n")
            with open(os.path.join(d, 'm/f.pas'), 'w') as fh:
                fh.write('procedure p;\ntype t = array[1..2] of integer;\nconst n = 5;\n'
                         'var\n  a: static t := [n, 1];\nbegin\nend;\n'
                         'procedure q;\nbegin\nend;\n')
            dpc = os.path.join(d, 'dpc')
            with open(dpc, 'w') as fh:
                fh.write(fake)
            os.chmod(dpc, 0o755)
            mod = srcsurvey.Module(d, 'm.pas')
            mod.stub({'p'})
            out = io.StringIO()
            with unittest.mock.patch('sys.stdout', out):
                entries, stubbed, ok = srcsurvey.survey_stage(dpc, mod, True, [], io.StringIO())
            self.assertEqual((stubbed, ok, mod.no_hoist), ([], True, {'p'}))
            self.assertIn('static data of p does not compile', out.getvalue())
            with open(os.path.join(d, 'm/f.pas')) as fh:
                self.assertNotIn('p_a', fh.read())
            self.assertNotIn('p_a', mod.unit_text('q'))

    def test_module_unit_text(self):
        with tempfile.TemporaryDirectory() as d:
            os.makedirs(os.path.join(d, 'm'))
            with open(os.path.join(d, 'm.pas'), 'w') as fh:
                fh.write("module m;\n%include 'm/f.pas';\n%include 'm/g.pas';\n")
            with open(os.path.join(d, 'm/f.pas'), 'w') as fh:
                fh.write(STATICS)
            with open(os.path.join(d, 'm/g.pas'), 'w') as fh:
                fh.write('procedure c; internal;\nbegin\nend;\n')
            mod = srcsurvey.Module(d, 'm.pas')
            self.assertEqual(mod.dsect, {'a': 'wired_data', 'b': None, 'c': None})
            u = mod.unit_text('c')       # c's data is the module's: wired_data goes
            self.assertNotIn('a_seps', u)
            self.assertIn('b_tbl', u)
            self.assertIn('procedure c; internal;\nbegin', u)
            u = mod.unit_text('b')
            self.assertNotIn('a_seps', u)
            mod.stub({'c'})               # a keeps its body: its statics are its own
            with open(os.path.join(d, 'm/f.pas')) as fh:
                self.assertEqual(fh.read(), STATICS)
            mod.stub({'a'})               # no wired_data routine left
            with open(os.path.join(d, 'm/f.pas')) as fh:
                f = fh.read()
            self.assertNotIn('seps', f)
            self.assertIn('procedure b; internal;\nvar', f)
            self.assertEqual(mod.stubbed, {'a', 'c'})


class SettingsTest(unittest.TestCase):
    """A5 mode and loop alignment from the image surveys (bead
    domain-pascal-xdh.7)."""

    def test_a5_always_from(self):
        # TTY .TEXT 0xE671DC: n (i) (S) n s s S s
        rows = [(0x10, 'A', 'n', True, False), (0x20, 'L', 'i', False, False),
                (0x30, 'DFL', 'S', False, False), (0x40, 'B', 'n', True, False),
                (0x50, 'C', 's', True, False), (0x60, 'D', 's', True, False),
                (0x70, 'E', 'S', True, False), (0x80, 'F', 's', True, False)]
        r = srcsurvey.a5_settings(rows)
        self.assertEqual(r['always_from'], (0x50, 'C'))
        self.assertEqual(r['interval'], (0x40, 0x50))
        self.assertEqual(r['always'], [(0x30, 'DFL')])
        self.assertEqual(r['absolute'], [])

    def test_a5_switch_back_and_absolute(self):
        # PKT-like: an n after the first s; an n with an absolute operand
        rows = [(0x10, 'A', 'n', True, True), (0x20, 'B', 's', True, False),
                (0x30, 'C', 'S', True, False), (0x40, 'D', 'n', True, False),
                (0x50, 'E', 's', True, False), (0x60, None, 's', False, False)]
        r = srcsurvey.a5_settings(rows)
        self.assertIsNone(r['always_from'])
        self.assertEqual(r['always'], [(0x20, 'B'), (0x50, 'E'), (0x60, None)])
        self.assertEqual(r['absolute'], [(0x10, 'A')])

    def test_a5_used_mode(self):
        r = srcsurvey.a5_settings([(0x10, 'A', 'n', True, False), (0x20, 'B', 'S', True, False),
                                   (0x30, 'C', '.', True, False)])
        self.assertEqual((r['always_from'], r['always'], r['absolute']), (None, [], []))

    def pr(self, classes):
        """A fake loopalign.per_routine list for one segment."""
        import collections
        seg = type('Seg', (), {'name': 'S'})()
        cnt = {'F': {'F': 1}, 'u': {'U': 1}, 'a': {'A': 1}, '.': {}, 'X': {'F': 1, 'U': 1}}
        return seg, [(seg, 0x100 + 0x10 * i, 0x110 + 0x10 * i, collections.Counter(cnt[c]))
                     for i, c in enumerate(classes)]

    def test_loop_off_from(self):
        seg, pr = self.pr('.F.a.u.u')
        names = {0x100 + 0x10 * i: 'R%d' % i for i in range(8)}
        r = srcsurvey.loop_settings(pr, seg, names)
        self.assertEqual((r['off_from'], r['interval'], r['error']),
                         ((0x150, 'R5'), (0x110, 0x150), None))
        seg, pr = self.pr('..u.')                     # no aligning routine
        self.assertEqual(srcsurvey.loop_settings(pr, seg, names)['off_from'], (0x120, 'R2'))

    def test_loop_on_and_end_padding(self):
        names = {0x100 + 0x10 * i: 'R%d' % i for i in range(4)}
        seg, pr = self.pr('.F..')
        self.assertIsNone(srcsurvey.loop_settings(pr, seg, names)['off_from'])
        self.assertIsNone(srcsurvey.loop_settings(pr, seg, names, True)['off_from'])
        r = srcsurvey.loop_settings(pr, seg, names, False)   # zero word at the end
        self.assertEqual((r['off_from'], r['interval']), ((0x130, 'R3'), (0x110, 0x130)))
        seg, pr = self.pr('..F')
        self.assertIn('ends in a zero word', srcsurvey.loop_settings(pr, seg, names, False)['error'])
        seg, pr = self.pr('uF')
        self.assertIn('does not align', srcsurvey.loop_settings(pr, seg, names)['error'])

    class Seg:
        def __init__(self, a, b):
            self.addr, self.end = a, b

        def contains(self, x):
            return self.addr <= x < self.end

    def settings(self):
        a5w = {'always_from': (0x150, 'K_FLUSH'), 'interval': (0x140, 0x150),
               'always': [(0x130, 'SET_RAW_MODE')], 'absolute': []}
        lw = {'off_from': (0x110, 'PUT_CHARS'), 'interval': (None, 0x110), 'error': None}
        a5t = {'always_from': None, 'interval': (None, None), 'always': [(0x320, None)],
               'absolute': [(0x310, 'ABS')], 'for_callees': [(0x330, 'READ')]}
        lt = {'off_from': None, 'interval': (None, None), 'error': 'aligns again'}
        return [('WIRED_PROC', self.Seg(0x100, 0x200), a5w, lw, [(0x150, 'K_FLUSH')]),
                ('.TEXT', self.Seg(0x300, 0x400), a5t, lt, [])]

    def test_settings_opts(self):
        opts, problems = srcsurvey.settings_opts(self.settings())
        self.assertEqual(opts, ['--a5-always-from=K_FLUSH', '--a5-always=SET_RAW_MODE',
                                '--a5-absolute=ABS', '--loop-align-off-from=PUT_CHARS',
                                '--park-d0=K_FLUSH'])
        self.assertEqual(problems, [
            '.TEXT: --a5-always needs module-local at 0x320, which has no name',
            '.TEXT: loop alignment: aligns again'])
        out = io.StringIO()
        srcsurvey.report_settings('m.pas', self.settings(), out)
        self.assertIn('  dpc --a5-always-from=K_FLUSH --a5-always=SET_RAW_MODE', out.getvalue())
        self.assertIn('D0 parking where a call follows: K_FLUSH', out.getvalue())

    def test_unit_build_opts(self):
        def r(name, addr, internal=False):
            x = srcsurvey.Routine(name)
            x.addr = addr
            if internal:
                x.options.add('internal')
            return x
        st = self.settings()
        f = srcsurvey.unit_build_opts
        self.assertEqual(f(st, r('a', 0x100, True), 0x100), [])
        self.assertEqual(f(st, r('put_chars', 0x110, True), 0x110), ['--loop-align=off'])
        self.assertEqual(f(st, r('set_raw_mode', 0x130, True), 0x120),
                         ['--a5-always=SET_RAW_MODE', '--loop-align=off'])
        self.assertEqual(f(st, r('k_flush', 0x150), 0x150),
                         ['--a5-always-from=K_FLUSH', '--loop-align=off', '--park-d0=K_FLUSH'])
        self.assertEqual(f(st, r('local', 0x160, True), 0x160), ['--loop-align=off'])
        self.assertEqual(f(st, r('abs', 0x310), 0x310), ['--a5-absolute=ABS'])
        # loads A5 only for its callees (LOG_$READ): said for the fragment
        # only, a whole-module build needs no option (test_settings_opts)
        self.assertEqual(f(st, r('read', 0x330), 0x330), ['--a5-always=READ'])
        self.assertEqual(f(st, r('elsewhere', 0x500), 0x500), [])
        # nested routines before the switch, the routine after it
        st[0][3]['off_from'] = (0x118, 'PARENT')
        self.assertEqual(f(st, r('parent', 0x118), 0x110)[-1], '--loop-align-off-from=PARENT')
        # the first routine that does not align is a nested one (no name):
        # off for the whole unit when none of the nested routines aligns
        st[0][3]['off_from'] = (0x114, None)
        self.assertEqual(f(st, r('parent', 0x118), 0x110)[-1], '--loop-align=off')
        st[0][3]['interval'] = (0x112, 0x114)            # one does: the nearest option
        self.assertEqual(f(st, r('parent', 0x118), 0x110)[-1], '--loop-align-off-from=PARENT')

    def test_report_quotes_for_a_shell(self):
        st = self.settings()
        st[0][2]['always_from'] = (0x150, 'TTY_$K_FLUSH_INPUT')
        out = io.StringIO()
        srcsurvey.report_settings('m.pas', st, out)
        self.assertIn("  dpc '--a5-always-from=TTY_$K_FLUSH_INPUT' ", out.getvalue())



class BaselineTest(unittest.TestCase):
    RES = [('log/log.pas', 'log_$read', 'OK'), ('log/log.pas', 'log_$add', 'STUB'),
           ('kbd/kbd.pas', 'kbd_$put', 'MISMATCH')]

    def test_round_trip(self):
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, 'b.txt')
            with open(p, 'w') as fh:
                fh.write(srcsurvey.format_baseline('abc123', self.RES))
            commit, base = srcsurvey.read_baseline(p)
        self.assertEqual(commit, 'abc123')
        self.assertEqual(base, {(m, n): st for m, n, st in self.RES})

    def test_unknown_status_refused(self):
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, 'b.txt')
            with open(p, 'w') as fh:
                fh.write('# source: abc\nOKAY\tlog/log.pas\tlog_$read\n')
            with self.assertRaises(ValueError):
                srcsurvey.read_baseline(p)

    def test_update_refuses_no_conform(self):
        with unittest.mock.patch('sys.stderr', io.StringIO()), \
                self.assertRaises(SystemExit):
            srcsurvey.main(['run', '--update', '--no-conform'])

    def test_compare(self):
        base = {(m, n): st for m, n, st in self.RES}
        self.assertEqual(srcsurvey.compare_baseline(base, self.RES), ([], 0))
        now = [('log/log.pas', 'log_$read', 'MISMATCH'),       # worse
               ('log/log.pas', 'log_$add', 'OK'),              # better
               ('tty/tty.pas', 'tty_$i_lock', 'OK')]           # new; kbd_$put gone
        lines, worse = srcsurvey.compare_baseline(base, now)
        self.assertEqual(worse, 2)
        self.assertEqual(len(lines), 4)
        self.assertTrue(any(ln.startswith('  WORSE') and 'kbd_$put: MISMATCH -> gone' in ln
                            for ln in lines))
        self.assertTrue(any('log_$add: STUB -> OK' in ln and ln.startswith('  diff')
                            for ln in lines))
        # a run of some modules compares only those
        lines, worse = srcsurvey.compare_baseline(base, now[:2], {'log/log.pas'})
        self.assertEqual((len(lines), worse), (2, 1))

    def test_routine_results(self):
        mod = srcsurvey.Module.__new__(srcsurvey.Module)
        a, b, c, h = (srcsurvey.Routine(n) for n in ('a', 'b', 'c', 'h'))
        h.options.add('extern')
        mod.routines = {'a': ('f', a), 'b': ('f', b), 'c': ('f', c), 'h': ('f', h)}
        mod.stubbed = {'b'}
        f = srcsurvey.routine_results
        self.assertEqual(f('m.pas', mod, True, {'a': 'OK', 'c': 'SETUP: no address'}),
                         [('m.pas', 'a', 'OK'), ('m.pas', 'b', 'STUB'),
                          ('m.pas', 'c', 'SETUP')])
        self.assertEqual([st for _m, _n, st in f('m.pas', mod, False, {})], ['BLOCKED'] * 3)
        self.assertEqual(f('m.pas', mod, True, {})[0][2], 'COMPILED')


if __name__ == '__main__':
    unittest.main()
