"""Tests for tools/localsurvey.py (no image, dpc, clang or ../source
needed)."""

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..'))
import localsurvey  # noqa: E402


def site(top, seq, nested=False, loop=None):
    return {'top': top, 'seq': seq, 'nested': nested, 'loop': loop}


def local(stores, loads, **kw):
    d = {'name': 'x', 'type': 'integer', 'offset': -6, 'bytes': 2, 'deref': False,
         'other': False, 'for_index': False, 'across_call': False,
         'stores': stores, 'loads': loads}
    d.update(kw)
    return d


def st(top, seq, nested=False, const=False, call=False, loop=None):
    return dict(site(top, seq, nested, loop), const=const, call=call)


def ld(top, seq, nested=False, arg=False, whole=False, cond=False, loop=None):
    return dict(site(top, seq, nested, loop), arg=arg, whole=whole, cond=cond)


class ImageTests(unittest.TestCase):
    INSNS = [(0xE1AF42, 4, 'linkw %fp,#-8'),
             (0xE1AF46, 4, 'moveml %d2/%a2,%sp@-'),
             (0xE1AF60, 4, 'movew %d0,%fp@(-6)'),
             (0xE1AF88, 4, 'movew %fp@(-6),%sp@-'),
             (0xE1AF8C, 4, 'moveb %d2,%a2@(5,%d1:w)')]

    def test_slot_used(self):
        self.assertTrue(localsurvey.slot_used(self.INSNS, -6, 2))
        self.assertTrue(localsurvey.slot_used(self.INSNS, -7, 2))
        self.assertFalse(localsurvey.slot_used(self.INSNS, -8, 2))

    def test_link_and_saves(self):
        self.assertEqual(localsurvey.link_size(self.INSNS), 8)
        self.assertEqual(localsurvey.saved_regs(self.INSNS), {'d2', 'a2'})
        self.assertEqual(localsurvey.saved_regs(
            [(0, 4, 'linkw %fp,#-4'), (4, 2, 'moveml %d2-%d4/%a2-%a3,%sp@-')]),
            {'d2', 'd3', 'd4', 'a2', 'a3'})
        self.assertEqual(localsurvey.saved_regs(
            [(0, 4, 'linkw %fp,#-4'), (4, 2, 'pea %a5@')]), {'a5'})
        self.assertEqual(localsurvey.saved_regs(
            [(0, 4, 'linkw %fp,#-4'), (4, 2, 'movel %d2,%sp@-')]), {'d2'})


class SourceTests(unittest.TestCase):
    def test_decl_notes(self):
        text = """procedure p(x: integer);
var
  token: integer;                         { (-0x6,A6) }
  new_tail: tty_$buf_pos_t;               { D0 }
begin
  token := f(x);                          { 0x00E1AF52 }
end;
"""
        n = localsurvey.decl_notes(text)
        self.assertEqual(n[('p', 'token')], '(-0x6,A6)')
        self.assertEqual(n[('p', 'new_tail')], 'D0')
        self.assertEqual(len(n), 2)


class RuleTests(unittest.TestCase):
    def test_only_pushed(self):
        token = local([st(0, 0, call=True)], [ld(3, 7, arg=True, whole=True)])
        f = localsurvey.features({'calls': [site(0, 0), site(3, 7)]}, token)
        self.assertTrue(f['only_pushed'])
        self.assertTrue(f['narrow'] is False)
        index = local([st(0, 1, nested=True, const=True)], [ld(1, 3, arg=True)])
        f = localsurvey.features({'calls': []}, index)
        self.assertFalse(f['pushed'])
        self.assertTrue(f['arg'])

    def test_plain(self):
        self.assertTrue(localsurvey.plain(local([st(0, 0)], [])))
        self.assertFalse(localsurvey.plain(local([st(0, 0)], [], deref=True)))
        self.assertFalse(localsurvey.plain(local([], [ld(0, 0)], for_index=True)))
        self.assertFalse(localsurvey.plain(local([], [])))

    def test_score(self):
        rows = [('R', 'token', {'only_pushed': True, 'pushed': True, 'arg': True,
                                'narrow': False, 'across_call': False,
                                'loop_load': False, 'loop_store': False}, 'frame'),
                ('R', 'new_tail', {'only_pushed': False, 'pushed': False, 'arg': False,
                                   'narrow': False, 'across_call': False,
                                   'loop_load': False, 'loop_store': False}, 'register')]
        got = {name: (hits, misses) for name, hits, misses in localsurvey.score(rows)}
        self.assertEqual(got['unless every read pushes it'], (2, []))
        self.assertEqual(got['every plain local'], (1, [('R', 'token', 'frame')]))
        self.assertEqual(got['narrow (12.5 before iyd)'][0], 1)


class CTests(unittest.TestCase):
    def test_c_scalar(self):
        self.assertTrue(localsurvey.c_scalar({'qualType': 'uint16_t',
                                              'desugaredQualType': 'unsigned short'}))
        self.assertTrue(localsurvey.c_scalar({'qualType': 'tty_desc_t *'}))
        self.assertTrue(localsurvey.c_scalar({'qualType': 'int8_t',
                                              'desugaredQualType': 'signed char'}))
        self.assertFalse(localsurvey.c_scalar({'qualType': 'status_$t',
                                               'desugaredQualType': 'unsigned int'}))
        self.assertFalse(localsurvey.c_scalar({'qualType': 'uid_t',
                                               'desugaredQualType': 'struct uid_t'}))
        self.assertFalse(localsurvey.c_scalar({'qualType': 'char[2]'}))

    def test_c_note_and_class(self):
        text = 'x;\n    uint16_t token;   /* D3 */\n'
        off = text.index('token')
        self.assertEqual(localsurvey.c_note(text, off), 'D3')
        self.assertEqual(localsurvey.c_class('D3', None), 'register')
        self.assertEqual(localsurvey.c_class('A6-0x14', None), 'frame')
        self.assertIsNone(localsurvey.c_class('(-0x6a,A6): D2', None))
        insns = [(0, 4, 'movew %d0,%fp@(-106)')]
        self.assertEqual(localsurvey.c_class('(-0x6a,A6): D2', insns), 'frame')
        self.assertEqual(localsurvey.c_class('(-0x6c,A6): D2', insns), 'register')
        self.assertIsNone(localsurvey.c_class('A6-0x8', []))
        self.assertIsNone(localsurvey.c_class('see above', None))

    def test_walk(self):
        # token = LOCK(&l); if (g) n = 1; UNLOCK(&l, ARCH_PASCAL_WORD_SLOT(token));
        # use(n + 1); M$MIS$LLL(n, 2);
        def ref(i, name='x'):
            return {'kind': 'ImplicitCastExpr', 'inner': [
                {'kind': 'DeclRefExpr', 'referencedDecl': {'id': i, 'name': name}}]}

        def call(name, *args):
            return {'kind': 'CallExpr',
                    'inner': [ref('f_' + name, name)] + list(args)}
        lit = {'kind': 'IntegerLiteral', 'value': '1'}
        sixteen = {'kind': 'IntegerLiteral', 'value': '16'}
        body = {'kind': 'CompoundStmt', 'inner': [
            {'kind': 'BinaryOperator', 'opcode': '=',
             'inner': [{'kind': 'DeclRefExpr', 'referencedDecl': {'id': 't'}},
                       call('LOCK', {'kind': 'UnaryOperator', 'opcode': '&',
                                     'inner': [ref('l')]})]},
            {'kind': 'IfStmt', 'inner': [
                ref('g'),
                {'kind': 'BinaryOperator', 'opcode': '=',
                 'inner': [{'kind': 'DeclRefExpr', 'referencedDecl': {'id': 'n'}}, lit]}]},
            call('UNLOCK', ref('l'),
                 {'kind': 'BinaryOperator', 'opcode': '<<',
                  'inner': [{'kind': 'CStyleCastExpr', 'inner': [ref('t')]}, sixteen]}),
            call('use', {'kind': 'BinaryOperator', 'opcode': '+', 'inner': [ref('n'), lit]}),
            call('M$MIS$LLL', ref('n'), lit),
        ]}
        locs = {k: {'name': k, 'stores': [], 'loads': [], 'deref': False,
                    'other': False, 'for_index': False} for k in ('t', 'n')}
        w = localsurvey._CWalk(locs)
        w.body(body)
        t, n = locs['t'], locs['n']
        self.assertEqual([(s['top'], s['call']) for s in t['stores']], [(0, True)])
        self.assertEqual([(x['arg'], x['whole']) for x in t['loads']], [(True, True)])
        self.assertEqual([(s['nested'], s['const']) for s in n['stores']], [(True, True)])
        self.assertEqual([(x['top'], x['arg'], x['whole']) for x in n['loads']],
                         [(3, True, False), (4, False, False)])
        self.assertEqual(len(w.calls), 3)      # LOCK, UNLOCK, use; not M$MIS$LLL
        # UNLOCK is the read itself; UNLOCK lies between n's store and reads.
        self.assertFalse(localsurvey._across(w.calls, t))
        self.assertTrue(localsurvey._across(w.calls, n))


if __name__ == '__main__':
    unittest.main()
