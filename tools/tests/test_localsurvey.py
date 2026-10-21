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


class ParamCopyTests(unittest.TestCase):
    # FILE_$SET_REFCNT 0xE5E40A (held in D0) and SMD_$SET_TP_CURSOR
    # 0xE6E972.. (copied), as objdump prints them
    HELD = [(0xE5E3F4, 4, 'linkw %fp,#-68'),
            (0xE5E40A, 4, 'moveal %fp@(12),%a0'),
            (0xE5E40E, 2, 'movel %a0@,%d0')]
    COPIED = [(0xE6E964, 4, 'linkw %fp,#-8'),
              (0xE6E972, 4, 'moveal %fp@(12),%a0'),
              (0xE6E976, 4, 'movel %a0@,%fp@(-4)'),
              (0xE6E97A, 4, 'moveal %fp@(16),%a1'),
              (0xE6E97E, 4, 'movew %a1@,%fp@(-6)')]

    def test_image_copy(self):
        self.assertIsNone(localsurvey.image_copy(self.HELD, 12))
        self.assertEqual(localsurvey.image_copy(self.COPIED, 12), -4)
        self.assertEqual(localsurvey.image_copy(self.COPIED, 16), -6)
        self.assertEqual(localsurvey.slot_loads(self.HELD, 12), 1)
        self.assertEqual(localsurvey.slot_loads(self.HELD, 8), 0)

    def test_rules(self):
        rules = dict(localsurvey.PARAM_RULES)
        rule = rules['unless every read pushes it, or a pointer']
        f = localsurvey.features({'calls': []}, local([], [ld(0, 0, cond=True)]))
        self.assertTrue(rule(f))
        f = localsurvey.features({'calls': []}, local([], [ld(0, 0, arg=True, whole=True)]))
        self.assertFalse(rule(f))
        f = localsurvey.features({'calls': []}, local([], [ld(0, 0)], type='pointer'))
        self.assertFalse(rule(f))
        dpc = rules['never assigned, not every read pushes it, no pointer, no read in a loop']
        f = localsurvey.features({'calls': []}, local([], [ld(0, 1, loop=[0, 2])]))
        self.assertFalse(dpc(f))
        f = localsurvey.features({'calls': []}, local([st(0, 0)], [ld(1, 1)]))
        self.assertFalse(dpc(f))
        f = localsurvey.features({'calls': []}, local([], [ld(1, 1)]))
        self.assertTrue(dpc(f))


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


def ref(name, pid):
    return {'kind': 'DeclRefExpr', 'name': name,
            'referencedDecl': {'id': pid, 'kind': 'ParmVarDecl'}}


class OneStmtTests(unittest.TestCase):
    """`onestmt' (bead domain-pascal-s4a5)."""

    def test_param_bytes_and_slots(self):
        b = localsurvey.c_param_bytes
        self.assertEqual(b({'qualType': 'tty_desc_t *'}), 4)
        self.assertEqual(b({'qualType': 'uint8_t', 'desugaredQualType': 'unsigned char'}), 1)
        self.assertEqual(b({'qualType': 'int16_t', 'desugaredQualType': 'short'}), 2)
        self.assertEqual(b({'qualType': 'uint32_t', 'desugaredQualType': 'unsigned int'}), 4)
        self.assertIsNone(b({'qualType': 'status_$t'}))
        self.assertIsNone(b({'qualType': 'uid_t', 'desugaredQualType': 'struct uid'}))
        # TTY_$I_ENABLE_CRASH_FUNC: pointer (8), char (0xc), boolean (0xe)
        self.assertEqual(localsurvey.param_slots([4, 1, 1]), [8, 12, 14])
        self.assertEqual(localsurvey.param_slots([2, None, 2]), [8, None, None])

    def test_statements(self):
        body = {'inner': [{'kind': 'DeclStmt', 'inner': [{'kind': 'VarDecl'}]},
                          {'kind': 'IfStmt'}, {'kind': 'NullStmt'},
                          {'kind': 'ReturnStmt'}]}
        self.assertEqual([x['kind'] for x in localsurvey._stmts(body)], ['IfStmt'])
        body['inner'][0]['inner'][0]['init'] = 'c'
        self.assertEqual(len(localsurvey._stmts(body)), 2)

    def test_param_reads(self):
        st = {'kind': 'IfStmt', 'inner': [
            ref('enable', 'p2'),
            {'kind': 'CallExpr', 'inner': [
                {'kind': 'ImplicitCastExpr', 'inner': [ref('f', 'x')]},
                {'kind': 'ImplicitCastExpr', 'inner': [ref('ch', 'p1')]},
                {'kind': 'UnaryOperator', 'opcode': '&', 'inner': [ref('ch', 'p1')]}]},
            {'kind': 'BinaryOperator', 'inner': [ref('ch', 'p1')]}]}
        reads, call = localsurvey.param_reads(st, {'p1': 1, 'p2': 2})
        self.assertTrue(call)
        self.assertEqual(reads, [('p2', 'cond', 'value'), ('p1', 'body', 'arg'),
                                 ('p1', 'body', 'addr'), ('p1', 'body', 'value')])

    def test_classify(self):
        insns = [(0xE67292, 4, 'linkw %fp,#0'),
                 (0xE67298, 4, 'moveal %fp@(8),%a0'),
                 (0xE6729C, 4, 'moveb %fp@(12),%d0'),
                 (0xE672A0, 4, 'moveb %fp@(14),%d1'),
                 (0xE672A4, 2, 'bpls 0xe672bc'),
                 (0xE672A6, 4, 'moveb %d0,%a0@(1240)'),
                 (0xE672C0, 4, 'cmpiw #3,%fp@(16)'),
                 (0xE672C4, 4, 'movew %fp@(18),%sp@-'),
                 (0xE672C8, 4, 'movew %fp@(20),%d2')]
        refs = localsurvey.slot_refs(insns, 14, 1)
        self.assertEqual(refs, [(3, True, 'load', 'moveb %fp@(14),%d1')])
        c = localsurvey.classify_param
        self.assertEqual(c(refs), 'entry')
        self.assertEqual(c(localsurvey.slot_refs(insns, 16, 2)), 'frame')
        self.assertEqual(c(localsurvey.slot_refs(insns, 18, 2)), 'pushed')
        self.assertEqual(c(localsurvey.slot_refs(insns, 20, 2)), 'late')
        self.assertEqual(c(localsurvey.slot_refs(insns, 22, 2)), 'none')
