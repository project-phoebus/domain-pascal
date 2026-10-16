"""Tests for tools/arithsurvey.py's site recognisers (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import arithsurvey  # noqa: E402


def insns(texts):
    """[(addr, len, text)] at 2-byte steps (lengths do not matter here)."""
    return [(0x100 + 2 * i, 2, t) for i, t in enumerate(texts)]


class CallSiteTest(unittest.TestCase):
    def test_word_argument_with_pad(self):
        # AREA_$FREE_FROM 0xE07FE6..0xE07FF8
        ins = insns(['subql #2,%sp', 'movew #11,%sp@-', 'movel %d2,%sp@-',
                     'jsr 0xe0acc0', 'addqw #8,%sp', 'clrl %d1', 'movew %d0,%d1'])
        c = arithsurvey.call_site(ins, 3)
        self.assertEqual(c['entry'], 'M$OIU$WLW')
        self.assertEqual(c['a'], 'movel %d2,%sp@-')
        self.assertEqual(c['b'], 'movew #11,%sp@-')
        self.assertEqual(c['pad'], 0x100)
        self.assertFalse(c['pad_inside'])
        self.assertEqual(c['pop'], 'addqw #8,%sp')
        self.assertEqual(c['use'], 'movew %d0,%d1')

    def test_pad_scheduled_into_the_operand(self):
        # CAL_$WEEKDAY 0xE686AE..0xE686C6
        ins = insns(['movew %a1@,%d0', 'subql #2,%sp', 'extl %d0', 'addil #9,%d0',
                     'movew #12,%sp@-', 'movel %d0,%sp@-', 'jsr 0xe0ad54', 'addqw #8,%sp',
                     'mulsw #153,%d0'])
        c = arithsurvey.call_site(ins, 6)
        self.assertEqual(c['entry'], 'M$OIS$WLW')
        self.assertTrue(c['pad_inside'])
        self.assertEqual(c['use'], 'mulsw #153,%d0')

    def test_unpopped_longword_call(self):
        # TIME_$GET_ADJUST 0xE16AD0..0xE16ADE
        ins = insns(['movel #250000,%sp@-', 'movel %d1,%sp@-', 'jsr 0xe0ad7c', 'lsll #2,%d0'])
        c = arithsurvey.call_site(ins, 2)
        self.assertEqual(c['entry'], 'M$OIS$LLL')
        self.assertIsNone(c['pad'])
        self.assertIsNone(c['pop'])

    def test_other_calls_are_not_sites(self):
        ins = insns(['subql #2,%sp', 'movew #14,%sp@-', 'jsr 0xe20b12'])
        self.assertIsNone(arithsurvey.call_site(ins, 2))


class InlineSiteTest(unittest.TestCase):
    def test_longword_index(self):
        ins = insns(['movew %d2,%d0', 'mulsw #228,%d0', 'leal %a0@(0,%d0:l),%a2'])
        s = arithsurvey.inline_site(ins, 1)
        self.assertEqual((s['op'], s['src'], s['cls']), ('mulsw', 'imm', 'index.l'))

    def test_product_used_as_a_longword(self):
        # PROC1_$LOADAV_CALLBACK 0xE14C18
        ins = insns(['mulsw #5240,%d2', 'addl %d2,%d0'])
        s = arithsurvey.inline_site(ins, 0)
        self.assertEqual((s['op'], s['cls']), ('mulsw', 'addl'))



class ScaleSiteTest(unittest.TestCase):
    def test_constant_first_through_a_pointer(self):
        # TPAD_$SET_UNIT_MODE 0xE69824..0xE69828
        ins = insns(['moveq #44,%d2', 'muluw %a4@,%d2', 'lea %a5@(0,%d2:w),%a1'])
        [s] = arithsurvey.scale_sites(ins)
        self.assertEqual((s['coef'], s['form'], s['mul'], s['x'], s['idx']),
                         (44, 'moveq*mem', 'mulu', 'ptr', 'w'))

    def test_constant_first_of_a_loaded_parameter(self):
        # module-local 0xE383B0 at 0xE383C8..0xE383D2
        ins = insns(['movew %fp@(12),%d2', 'moveq #100,%d0', 'muluw %d2,%d0',
                     'lea %a5@(0,%d0:w),%a3'])
        [s] = arithsurvey.scale_sites(ins)
        self.assertEqual((s['coef'], s['form'], s['x'], s['idx']),
                         (100, 'const*reg', 'param', 'w'))

    def test_immediate_after_a_copy(self):
        # BAT_$GET_BAT_STEP 0xE3BB64..0xE3BB6E
        ins = insns(['movew %fp@(8),%d0', 'movew %d0,%d1', 'muluw #564,%d1',
                     'lea %a5@(0,%d1:w),%a0'])
        [s] = arithsurvey.scale_sites(ins)
        self.assertEqual((s['coef'], s['form'], s['idx']), (564, 'mulu #imm', 'w'))

    def test_shifts_and_adds(self):
        # PEB_$GET_FP 0xE5AEF6..0xE5AF04 (x28, with `neg')
        ins = insns(['movew %a0@,%d0', 'lslw #2,%d0', 'movew %d0,%d1', 'negw %d0',
                     'lslw #3,%d1', 'addw %d1,%d0', 'movel %a5@(0,%d0:w),%d2'])
        [s] = arithsurvey.scale_sites(ins)
        self.assertEqual((s['coef'], s['form'], s['mul'], s['x']), (28, 'shifts', None, 'ptr'))

    def test_word_product_stored(self):
        # module-local 0xE4519C at 0xE458CC..0xE458D8
        ins = insns(['moveq #44,%d3', 'mulsw %d2,%d3', 'addiw #52,%d3', 'movew %d3,%a2@'])
        [s] = arithsurvey.scale_sites(ins)
        self.assertEqual((s['coef'], s['off'], s['form'], s['mul'], s['x'], s['idx']),
                         (44, 52, 'const*reg', 'muls', 'held', None))

    def test_constant_first_of_a_mask(self):
        # HINT_$GET_HINTS 0xE4998E..0xE4999E
        ins = insns(['moveq #63,%d3', 'moveq #84,%d5', 'andw %d0,%d3',
                     'moveal 0xe2459c,%a1', 'mulsw %d3,%d5', 'moveq #2,%d3',
                     'lea %a1@(0,%d5:l),%a1'])
        [s] = arithsurvey.scale_sites(ins)
        self.assertEqual((s['coef'], s['form'], s['mul'], s['x'], s['idx']),
                         (84, 'const*reg', 'muls', 'computed', 'l'))

    def test_a_label_ends_the_block(self):
        ins = insns(['moveq #44,%d0', 'muluw %a0@,%d0', 'lea %a5@(0,%d0:w),%a1'])
        self.assertEqual(arithsurvey.scale_sites(ins, frozenset([ins[1][0]])), [])

    def test_a_clear_is_not_a_reading(self):
        # 0xE54F2C..0xE54F38: the product read by the index only.
        ins = insns(['movew %d3,%d0', 'muluw #150,%d0', 'lea %a2@(0,%d0:l),%a3', 'clrw %d0'])
        [s] = arithsurvey.scale_sites(ins)
        self.assertEqual((s['coef'], s['idx']), (150, 'l'))

    def test_operands(self):
        self.assertEqual(arithsurvey.operands('movew %a5@(0,%d2:w),%a1@(-40)'),
                         ('movew', ['%a5@(0,%d2:w)', '%a1@(-40)']))
        self.assertEqual(arithsurvey.mem_kind('%fp@(8)'), 'param')
        self.assertEqual(arithsurvey.mem_kind('%fp@(-2)'), 'local')
        self.assertEqual(arithsurvey.mem_kind('%a5@(380)'), 'unit')
        self.assertEqual(arithsurvey.mem_kind('0xe2060a'), 'abs')



class NarrowTest(unittest.TestCase):
    """`narrow' (bead domain-pascal-8td)."""

    def test_lowword(self):
        # MMAP_$FREE_PAGES 0xE0CE98..0xE0CF2C: a longword local, its low
        # word stored
        ins = insns(['movel %fp@(-28),%fp@(-20)', 'movel %fp@(-20),%d2',
                     'movew %fp@(-18),%a4@(-8186)', 'moveb %fp@(-17),%d0',
                     'movew %fp@(-30),%d1'])
        self.assertEqual(arithsurvey.lowword_sites(ins),
                         [('l@d,w@d+2', -20, 0x104), ('w@d,b@d+1', -18, 0x106)])

    def test_narrowed_by_writer(self):
        # DBUF_$GET_BLOCK 0xE3A5F2..0xE3A5F6; a word store of a word value
        # is not a site, nor a register formed before a call
        ins = insns(['movel %d3,%d1', 'lsrl #5,%d1', 'movew %d1,%sp@-',
                     'movew %d4,%d0', 'movew %d0,%fp@(-2)',
                     'movel %a0@,%d5', 'jsr 0xe20b12', 'movew %d5,%sp@-',
                     'movew %fp@(12),%d0', 'moveb %d0,%a0@(45)'])
        got = [(s, c, a) for s, c, a, *_ in arithsurvey.narrowed_sites(ins)]
        self.assertEqual(got, [('w', 'lsr.l', 0x104), ('b', 'load.w', 0x112)])

    def test_widened_pushes(self):
        # APP_$RECEIVE 0xE008E6..0xE008EC; SOCK_$CLOSE 0xE16008..0xE1600E;
        # a sign extension
        ins = insns(['clrl %d1', 'movew %a3@(994),%d1', 'movel %d1,%sp@-',
                     'clrw %d0', 'moveb %a2@(27),%d0', 'movew %d0,%sp@-',
                     'movew %d2,%d0', 'extl %d0', 'movel %d0,%sp@-',
                     'movew %d2,%d1', 'movel %d1,%sp@-'])
        self.assertEqual(arithsurvey.widened_sites(ins),
                         [('clr.l;move.w', 0x104), ('clr.w;move.b', 0x10a), ('ext.l', 0x110)])

    def test_heldop(self):
        # 0xE06302..0xE06312: a longword parameter's low word masked
        ins = insns(['movel %fp@(8),%d4', 'moveq #31,%d3', 'andw %d4,%d3',
                     'movew %d2,%d1', 'subw %d1,%d0', 'extl %d5', 'addw %d5,%d0'])
        got = [(o, c, a) for o, c, a, *_ in arithsurvey.heldop_sites(ins)]
        self.assertEqual(got, [('and', 'load.l', 0x104)])


class WordIndexTest(unittest.TestCase):
    def test_zero_extended_copy(self):
        # AREA_$DELETE 0xE07C4A..0xE07C5E: the `clr.l' two before the copy
        ins = insns(['movew %fp@(10),%d0', 'clrl %d1', 'moveal #14240768,%a0',
                     'movew %d0,%d1', 'subql #2,%sp', 'lsll #4,%d1', 'movel %d1,%d0',
                     'addl %d0,%d0', 'addl %d0,%d1', 'leal %a0@(-48,%d1:l),%a3'])
        chain = arithsurvey.index_chain(ins, 9, '%d1')
        self.assertEqual(chain[:2], ['clrl %d1', 'movew %d0,%d1'])
        self.assertEqual(arithsurvey.zext_base_order(ins, 9, 'a0', chain),
                         ('reg', 'base-between', 0))
        c = arithsurvey.classify_index(chain)
        self.assertTrue(c['zext'])
        self.assertFalse(c['computed'])

    def test_base_after_a_slot_load(self):
        # module-local 0xE085A6 at 0xE085B8..0xE085CC
        ins = insns(['clrl %d0', 'movew %fp@(8),%d0', 'moveal #14240768,%a0', 'lsll #4,%d0',
                     'movel %d0,%d1', 'addl %d1,%d1', 'addl %d1,%d0',
                     'leal %a0@(-48,%d0:l),%a1'])
        chain = arithsurvey.index_chain(ins, 7, '%d0')
        self.assertEqual(arithsurvey.zext_base_order(ins, 7, 'a0', chain),
                         ('param', 'base-after', 0))

    def test_sign_extended_and_multiplied(self):
        ins = insns(['movew %fp@(8),%d2', 'movew %d2,%d0', 'mulsw #228,%d0',
                     'leal %a0@(0,%d0:l),%a3'])
        c = arithsurvey.classify_index(arithsurvey.index_chain(ins, 3, '%d0'))
        self.assertEqual((c['width'], c['mul'], c['computed']), ('w', 'muls', False))
        ins = insns(['movew %d4,%d0', 'extl %d0', 'lsll #2,%d0', 'leal %a0@(0,%d0:l),%a1'])
        c = arithsurvey.classify_index(arithsurvey.index_chain(ins, 3, '%d0'))
        self.assertTrue(c['ext'])

    def test_computed(self):
        # 0xE1B73C: `x - 1' as a `.w' index
        ins = insns(['movew %a3@,%d0', 'subqw #1,%d0', 'leal %a2@(0,%d0:w),%a0'])
        c = arithsurvey.classify_index(arithsurvey.index_chain(ins, 2, '%d0'))
        self.assertTrue(c['computed'])
        self.assertEqual(arithsurvey.wordidx_form(dict(c, idx='w')), 'w computed')


if __name__ == '__main__':
    unittest.main()
