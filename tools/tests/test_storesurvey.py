"""Tests for tools/storesurvey.py (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import regalloc  # noqa: E402
import storesurvey  # noqa: E402


class FakeRoutine:
    def __init__(self, texts, start=0x100):
        self.ins = [regalloc.Insn(start + 2 * i, 2, t) for i, t in enumerate(texts)]


def run(texts):
    rt = FakeRoutine(texts)
    return storesurvey.stores(rt, [list(range(len(rt.ins)))])


class StoresTest(unittest.TestCase):
    def test_set_delay(self):
        # TTY_$K_SET_DELAY 0xE67A40..0xE67A4A: the element (destination)
        # first, interleaved with the value's slot load
        res = run(['moveal %d0,%a0', 'movew %a2@,%d1',
                   'moveal %fp@(16),%a1', 'addw %d1,%d1',
                   'movew %a1@,%a0@(64,%d1:w)'])
        self.assertEqual(len(res), 1)
        addr, dsh, ssh, first, nd, ns, inter, _ = res[0]
        self.assertEqual((addr, dsh, ssh, first, nd, ns, inter),
                         (0x108, 'idx', 'ptr', 'dst', 3, 1, True))
        self.assertEqual(storesurvey.predicted(nd, ns), 'dst')

    def test_tie_value_first(self):
        # TTY_$K_GET 0xE1C4D6..0xE1C4DE
        res = run(['lea %a2@(0,%d2:w),%a0', 'moveal %fp@(-4),%a1',
                   'moveb %a0@(721),%a1@'])
        self.assertEqual([r[1:7] for r in res], [('ptr', 'ptr', 'src', 1, 1, False)])
        self.assertEqual(storesurvey.predicted(1, 1), 'src')

    def test_statement_boundary_and_held(self):
        # a register set up before the previous store is not this
        # statement's; a held register (A5/A6) sets nothing up
        res = run(['moveal %fp@(8),%a0', 'movew #1,%a0@', 'moveal %fp@(12),%a1',
                   'movew %a1@,%a5@(4)'])
        self.assertEqual(res, [])


def copies(texts):
    return storesurvey.small_copies(FakeRoutine(texts).ins)


class SmallCopiesTest(unittest.TestCase):
    def test_inq_input_break_mode(self):
        # TTY_$K_INQ_INPUT_BREAK_MODE 0xE678E6..0xE678F2: 8 bytes, the
        # source post-incremented, the destination's slot loaded first
        res = copies(['moveal %d0,%a0', 'moveal %fp@(12),%a3',
                      'lea %a0@(56),%a1', 'movel %a1@+,%a3@',
                      'movel %a1@+,%a3@(4)'])
        self.assertEqual(len(res), 1)
        c = res[0]
        self.assertEqual((c['addr'], c['size'], c['form'], c['moves'], c['src'],
                          c['dst'], c['sreg'], c['dreg']),
                         (0x106, 8, 'pi-disp', 'l+l', 'reg', 'reg', 'a1', 'a3'))
        self.assertEqual(c['first'], 'src')   # the movea.l D0,A0 is the source's

    def test_six_byte_displacement_pair(self):
        # KBD_$RCV 0xE1CDB8: move.l then move.w, both by displacement
        res = copies(['movel %fp@(-16),%a2@(32)', 'movew %fp@(-12),%a2@(36)',
                      'movel %a2@(34),%d1'])
        self.assertEqual([(c['size'], c['form'], c['moves'], c['src'], c['dst'])
                          for c in res], [(6, 'disp', 'l+w', 'a6', 'reg')])

    def test_loops_and_unrelated_moves(self):
        # a dbf copy loop is not a small copy; non-consecutive offsets
        # are not one run
        res = copies(['lea %a5@,%a0', 'movel %a0@+,%a1@+', 'dbf %d0,0x100',
                      'movel %a5@(8),%fp@(-8)', 'movel %a5@(16),%fp@(-4)'])
        self.assertEqual(res, [])

    def test_abs_destination(self):
        # 0xE31C28: into another unit's object by absolute address
        res = copies(['moveal #14775436,%a0', 'movel %a0@+,0xe2453c',
                      'movel %a0@+,0xe24540'])
        self.assertEqual([(c['form'], c['dst'], c['size']) for c in res],
                         [('pi-disp', 'abs', 8)])



def pairs(texts):
    return storesurvey.adjacent_pairs(FakeRoutine(texts).ins)


class MergeSurveyTest(unittest.TestCase):
    def test_i_init_buffer_stores(self):
        # TTY_$I_INIT 0xE3332A..0xE33340: word constants one at a time
        res = pairs(['moveal %fp@(8),%a4', 'movew #1,%a4@(714)', 'movew #1,%a4@(716)',
                     'movew #1,%a4@(718)', 'movew #256,%a4@(720)'])
        self.assertEqual([(p['sizes'], p['base'], p['values'], p['order'], p['mod4'])
                          for p in res],
                         [('2+2', 'reg', 'const+const', 'asc', 2)] +
                         [('2+2', 'reg', 'const+const', 'asc', 0)] +
                         [('2+2', 'reg', 'const+const', 'asc', 2)])
        self.assertEqual(res[0]['basedef'], 'moveal %fp@(8),%a4')

    def test_descending_clears_and_moveq(self):
        # TPAD_$SET_UNIT_CURSOR 0xE69902 (A5, descending); a constant
        # through `moveq' counts as a constant
        res = pairs(['clrw %a5@(378)', 'clrw %a5@(376)'])
        self.assertEqual([(p['base'], p['values'], p['order']) for p in res],
                         [('a5', 'zero+zero', 'desc')])
        res = pairs(['moveq #1,%d0', 'movew %d0,%fp@(-4)', 'movew %a0@,%fp@(-2)'])
        self.assertEqual([p['values'] for p in res], ['const+mem'])

    def test_boundaries(self):
        # not adjacent, too big, a call, a branch target, the base reloaded
        self.assertEqual(pairs(['clrw %a0@(4)', 'clrw %a0@(8)']), [])
        self.assertEqual(pairs(['clrl %a0@(4)', 'clrw %a0@(8)']), [])
        self.assertEqual(pairs(['clrw %a0@', 'jsr 0xe20b12', 'clrw %a0@(2)']), [])
        self.assertEqual(pairs(['clrw %a0@', 'bras 0x104', 'clrw %a0@(2)']), [])
        self.assertEqual(pairs(['clrw %a0@', 'moveal %a1,%a0', 'clrw %a0@(2)']), [])
        # a set-up instruction between them does not end the pair
        res = pairs(['clrw %a0@', 'moveq #3,%d1', 'clrw %a0@(2)'])
        self.assertEqual(len(res), 1)

    def test_copy_class(self):
        # ACL 0xE31EA2: adjacent words copied to adjacent words
        res = pairs(['movew %a1@(2048),%a0@', 'movew %a1@(2050),%a0@(2)'])
        self.assertTrue(res[0]['copy'])
        res = pairs(['movew %a1@(2050),%a0@', 'movew %a1@(2048),%a0@(2)'])
        self.assertFalse(res[0]['copy'])

    def test_merged_candidates(self):
        # TTY_$K_RESET 0xE67322 and 0xE6732A; LOG_$INIT 0xE3017C's
        # `moveq #1,D0; move.l D0,(A0)' (0 and 1) has a zero half: not listed
        rt = FakeRoutine(['movel #65537,%a2@(714)', 'movel #65792,%a2@(718)',
                          'moveq #1,%d0', 'movel %d0,%a0@', 'movel #65536,%a0@',
                          'movew #257,%a0@'])
        res = storesurvey.merged_candidates(rt.ins)
        self.assertEqual([(c['size'], c['value']) for c in res],
                         [(4, 0x10001), (4, 0x10100), (2, 0x101)])

    def test_word_and_longword(self):
        # FIM_$BUILD_DF 0xE0A896 (merged, split again) and 0xE14CBC
        # (apart): counted only when asked for
        texts = ['clrw %fp@(-20)', 'movel #1250000,%fp@(-18)']
        self.assertEqual(pairs(texts), [])
        res = storesurvey.adjacent_pairs(FakeRoutine(texts).ins, (6,))
        self.assertEqual([(p['sizes'], p['values'], p['order']) for p in res],
                         [('2+4', 'zero+const', 'asc')])


class UnitsTest(unittest.TestCase):
    """`units' (bead domain-pascal-qb9q): the per-unit store-merging knob."""

    def pair(self, sizes='2+2', order='asc', base='reg', mod4=0, addr=0x100):
        return {'sizes': sizes, 'order': order, 'base': base, 'mod4': mod4, 'addr': addr}

    def test_knob_class(self):
        f = storesurvey.knob_class
        self.assertEqual(f(self.pair()), 'merge')
        self.assertEqual(f(self.pair(mod4=2)), 'merge')            # TTY_$K_RESET 0xE67322
        # the frame: by the objects, which the image does not show (APP
        # 0xE00BA8 merged, PROC2_$INIT 0xE30412 and ASKNODE_$SERVER
        # 0xE65A3E two locals apart)
        self.assertEqual(f(self.pair(base='a6')), 'frame')
        self.assertEqual(f(self.pair(base='a6', mod4=2)), 'frame')
        # RING 0xE2FAC4: a device's zero
        self.assertEqual(f(dict(self.pair(), text='movew #0,%a0@; movew #0,%a0@(2)')),
                         'device')
        for p in (self.pair(order='desc'), self.pair(sizes='1+1'),
                  self.pair(base='a5'), self.pair(base='abs'), self.pair(sizes='2+4')):
            self.assertEqual(f(p), 'apart or refused', p)

    def test_unit_status(self):
        seg = lambda a: a & ~0xff                                    # noqa: E731
        apart = [self.pair(addr=0x210), self.pair(sizes='1+1', addr=0x120),
                 self.pair(addr=0x310)]
        st = storesurvey.unit_status([0x110, 0x320], apart, seg)
        # 0x100: a merge and only a byte pair apart; 0x200: a word pair
        # apart, no merge; 0x300: both
        self.assertEqual(st, {0x100: 'merges', 0x200: 'apart', 0x300: 'conflict'})
        # the frame counts both ways at 0 mod 4 (NETWORK_$PAGE_SERVER
        # 0xE117C6 apart), not at 2 mod 4 (no frame merge there)
        apart = [self.pair(base='a6', addr=0x410), self.pair(base='a6', mod4=2, addr=0x510)]
        self.assertEqual(storesurvey.unit_status([], apart, seg), {0x400: 'apart'})

    def test_greedy_and_image_stores(self):
        # TTY_$K_RESET 0xE67322: four words as two longwords; KBD_$INIT
        # 0xE33378: not paired from the first
        self.assertEqual(storesurvey.greedy((0x2CA, 0x2CC, 0x2CE, 0x2D0)),
                         [(0x2CA, 4), (0x2CE, 4)])
        self.assertEqual(storesurvey.greedy((0x58, 0x5A, 0x5C)), [(0x58, 4), (0x5C, 2)])
        self.assertEqual(storesurvey.image_stores(
            ('clrw %a2@(56)', 'clrl %a2@(58)', 'movel #65537,%a2@(714)')),
            [(56, 2), (58, 4), (714, 4)])
        self.assertNotEqual(storesurvey.greedy((0x38, 0x3A, 0x3C, 0x3E)),
                            storesurvey.image_stores(('clrw %a2@(56)', 'clrl %a2@(58)',
                                                      'clrw %a2@(62)')))

    def test_site_base(self):
        self.assertEqual(storesurvey.site_base({'texts': ('moveq #1,%d0', 'movel %d0,%a0@')}),
                         'reg')
        self.assertEqual(storesurvey.site_base({'texts': ('movel #1,%fp@(-40)',)}), 'a6')
        self.assertEqual(storesurvey.site_base({'texts': ('lea %pc@(0xe3e21e),%a0',
                                                          'lea %a3@,%a1')}), 'pc')


def addr_sites(texts, syms=()):
    return storesurvey.addr_sites(FakeRoutine(texts).ins, lambda v: v in syms)


class AddrSitesTest(unittest.TestCase):
    def test_net_open(self):
        # NET_$OPEN 0xE5A1B8..0xE5A1C0: an address inside another unit's
        # record into D0, the record's own address subtracted as an
        # immediate in the `90bc' encoding
        res = addr_sites(['subql #2,%sp', 'pea %a3@', 'lea 0xe2451c,%a0',
                          'movel %a0,%d0', 'subl #14828788,%d0',
                          'movew %d0,%sp@-'], syms={14828788})
        self.assertEqual([(r['kind'], r['use']) for r in res],
                         [('areg', 'movel'), ('imm', 'sub')])
        self.assertEqual((res[0]['dst'], res[0]['def']), ('dreg', 'abs'))
        self.assertEqual((res[1]['enc'], res[1]['sym']), ('ea', 'sym'))

    def test_difference_of_frame_addresses(self):
        # ASKNODE_$SERVER 0xE65DA0..0xE65DAA
        res = addr_sites(['lea %fp@(-592),%a0', 'lea %fp@(-582),%a1',
                          'movel %a1,%d1', 'subl %a0,%d1'])
        self.assertEqual([(r['use'], r['dst'], r['def']) for r in res],
                         [('movel', 'dreg', 'a6'), ('subl', 'dreg', 'a6')])

    def test_not_sites(self):
        # a pointer loaded (not an address formed), the filler, an update
        # `lea (d,An),An', a numeric `subi', a branch between
        res = addr_sites(['moveal %fp@(8),%a0', 'movel %a0,%d0',
                          'lea %a1@(4),%a1', 'movel %a1,%d1',
                          'lea %a5@(8),%a0', 'moveal %a0,%a0',
                          'subil #1024,%d2',
                          'lea %a5@(8),%a2', 'bras 0x200', 'movel %a2,%d3'])
        self.assertEqual(res, [])
        res = addr_sites(['subil #14828788,%d0'])
        self.assertEqual([(r['enc'], r['sym']) for r in res], [('i', 'num')])


class SitesTest(unittest.TestCase):
    """`sites' (bead domain-pascal-2m0): merged stores against the
    declared objects."""

    def test_covers_tty_buffers(self):
        # TTY_$K_RESET 0xE67322: input_head with the first field of the
        # input buffer, then tail+size; output head+tail, size alone
        lay = storesurvey._TTY_BUF
        self.assertEqual(storesurvey.covers(lay, 0x2CA, 4),
                         [('input_head', 'whole'), ('input.head', 'whole')])
        self.assertEqual(storesurvey.covers(lay, 0x3D6, 2), [('output.size', 'whole')])
        self.assertFalse(storesurvey.one_object(lay, ((0x2CA, 4),)))
        self.assertTrue(storesurvey.one_object(lay, ((0x3D6, 2),)))

    def test_split_object(self):
        # FIM_$BUILD_DF 0xE0A896: a word and a longword cleared l+w; the
        # longword stored in two parts
        lay = ((0x54, 2, 'sr'), (0x56, 4, 'pc'))
        self.assertEqual(storesurvey.covers(lay, 0x54, 4), [('sr', 'whole'), ('pc', 'part')])
        self.assertEqual(storesurvey.covers(lay, 0x58, 2), [('pc', 'part')])
        self.assertIsNone(storesurvey.one_object((), ((0, 4),)))

    def test_verdict(self):
        s = storesurvey._site(0x100, 'const', ('movel #65537,%a0@',), ((0, 4),),
                              storesurvey._HEAD_TAIL, 'x', ((0x200, 'tstw %a0@(2)'),))
        self.assertEqual(storesurvey.site_verdict(s, True), 'two objects')
        # the access apart not found, or none listed: not shown
        self.assertEqual(storesurvey.site_verdict(s, False), 'one object possible')
        s['apart'] = ()
        self.assertEqual(storesurvey.site_verdict(s, True), 'one object possible')
        # Apollo's layout is enough
        s['decl'] = storesurvey.APOLLO + ' sys/ins/x.ins.pas'
        self.assertEqual(storesurvey.site_verdict(s, True), 'two objects')
        # one whole object per store: one object, whatever the evidence
        s['layout'] = ((0, 4, 'l'),)
        self.assertEqual(storesurvey.site_verdict(s, True), 'one object possible')

    def test_overlapping_copies(self):
        # ROUTE_$SHORT_PORT 0xE69C16 copies route_$port_t's 0x2C..0x2F,
        # HINT_$ADD_NET 0xE49CA6 its 0x2E..0x31: not both one object
        site = storesurvey._site
        a = site(0xE69C16, 'copy', ('x',), ((4, 4),), (), 'd', src=('port', 0x2C, 4))
        b = site(0xE49CA6, 'copy', ('x',), ((8, 4),), (), 'd', src=('port', 0x2E, 4))
        c = site(0xE6905E, 'copy', ('x',), ((8, 4),), (), 'd', src=('port', 0x2E, 4))
        d = site(0xE16108, 'copy', ('x',), ((42, 4),), (), 'd', src=('other', 0x2E, 4))
        over = storesurvey.overlapping([a, b, c, d])
        self.assertEqual(over, {0xE69C16: [0xE49CA6, 0xE6905E],
                                0xE49CA6: [0xE69C16], 0xE6905E: [0xE69C16]})
        self.assertEqual(storesurvey.site_verdict(b, True, over[0xE49CA6]), 'overlaps')
        self.assertEqual(storesurvey.site_verdict(d, True, ()), 'one object possible')

    def test_texts_match(self):
        ins = [(0x100, 6, 'movel #65537,%a2@(56)'), (0x106, 4, 'clrw %a2@(60)')]
        self.assertTrue(storesurvey.texts_match(ins, ('movel #65537,%a2@(56)',)))
        self.assertFalse(storesurvey.texts_match(ins, ('movel #65537,%a2@(58)',)))
        self.assertFalse(storesurvey.texts_match(ins[:1], ('movel #65537,%a2@(56)',
                                                           'clrw %a2@(60)')))

    def test_segment_rows(self):
        rows = storesurvey.segment_rows([1, 2, 11], [12, 13, 21],
                                        lambda a: 'S%d' % (a // 10))
        self.assertEqual(rows, {'S0': [2, 0], 'S1': [1, 2], 'S2': [0, 1]})

    def test_tables_are_consistent(self):
        # every refused site the image merges is a listed merged site
        merged = {s['addr'] for s in storesurvey.MERGED_SITES}
        for f, kind, a, texts, v in storesurvey.REFUSAL_SITES:
            self.assertIn(v, ('merged', 'apart', 'not a pair'), f)
            self.assertIn(kind, ('const', 'copy'), f)
            if v == 'merged':
                self.assertIn(a, merged, f)
        for s in storesurvey.MERGED_SITES:
            self.assertEqual(s['addr'] % 2, 0)
            self.assertTrue(s['texts'] and s['stores'], hex(s['addr']))


if __name__ == '__main__':
    unittest.main()
