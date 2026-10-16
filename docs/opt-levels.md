# `-opt` levels in the SR10.2 kernel (survey, bead domain-pascal-g728)

Question (owner, 2026-10-16): before `dpc` gets a per-unit option for
store merging, find out whether the per-module behaviours already found in
the image (store merging, codegen-patterns 13; loop-head alignment,
section 7; A5 loading, section 4) are the effect of Apollo's `-opt`
level.  Method: for every compiled map segment, look for each
optimisation that ref 6.4.24 assigns to a level, and see whether the
segments fall into groups that one level per compilation would make.

**Result, in short (survey; statuses per item below):** no segment shows
evidence of a level lower than the one seen across the image, but many
cannot tell.  Of the 181 segments with modelled code, 127 (1461
routines) show an optimisation above `-opt 0` (a global CSE, a
shift-and-add product or a stepped pointer); 29 (88 routines) show only
`-opt 0` signs (values kept across statements, a shared pop); 25 (39
routines) show neither, and 18 segments are not modelled at all
(`tools/optsurvey.py tell -v`: PRINT_BUILD_TIME 0xE38000, SIO_IO 0xE725B0,
ATBUS_ 0xE0AABC, KBD 0xE724C4, TIME_Q_ 0xE16AE8, ...).  Those small
segments simply have no code where a higher level would show.  The signs
of a *missing* optimisation are rare (4 to 60 routines each) and are
explained at their sites by source idioms or by limits of the optimiser;
at segment level they do not go together beyond chance (Reported: the
test has little power for rare flags).  None of the three per-module
knobs follows them.  So the image gives no basis for replacing `dpc`'s
per-unit knobs with an `-opt` option.  Which level was used is not
recorded; the features fit `-opt 3` (the default, ref p. 6-26) or
`-opt 4` (Uncertain; see "Levels").

## 1. What the reference says

Ref 6.4.24, "-opt: Optimized Code", pp. 6-26..6-32 (read in the PDF,
pp. 6-27..6-30 checked against the OCR text; it has no table).  "The
-opt 3 option is the default"; "-opt" alone is `-opt 3`; `-nopt` is
`-opt 0`; "Each higher level of optimization includes all optimizations
performed at the lower levels" (p. 6-26).  `-dbs` "sets the -opt
option" and `-dba` sets `-opt 0` whatever `-opt` says (6.4.10, p. 6-17).
Nothing in 6.4.24 or 6.4.10 mentions code or loop alignment, A5, or a
setting for store merging other than its level.

Other settings that change the same optimisations:

- **`device`** (3.15.3, p. 3-57): "By default, the compiler optimizes
  certain adjacent references by merging them into one large reference.
  The device attribute prevents this optimization."  So a pair kept
  apart may be a `device` object: per object, not per compilation.  In
  the image a `device` zero is `move #0`, not `clr` (codegen-patterns 13,
  WIN_ 0xE19112, RING 0xE758E2), and `true` is `move.b #0xff`, not
  `st`.  Of the 118 constant pairs apart (2m0), 41 contain a `clr` or
  `st` and are not `device` stores by that rule; 1 is RING's `move.w #0`
  (2m0: the device zero); the other 76 are nonzero `move #k` pairs that
  the image cannot classify (among them TTY_$I_INIT 0xE3332E..0xE3334C,
  TPAD 0xE3363E, MST_UNWIRED's descending absolute run 0xE30A04..,
  REM_FILE's 37 frame byte pairs; ad-hoc script `tmp/g728/dev.py`).
  TTY_$I_INIT's fields are the ones TTY_$K_RESET merges (0xE67322), so
  they are not `device` if both use one declaration (Uncertain:
  ../source's reading).
- **`-indexl`** (6.4.15, p. 6-21): "disables some optimizations and
  forces the compiler to use 32-bit indexing in subscript calculations";
  the default `-nindexl` picks 16- or 32-bit indexing from the array's
  bounds.  A compilation with `-indexl` would have no `(d,An,Dn.w)`
  subscripts.  Ad-hoc count (`tmp/g728/idx.py`, not kept): of 129
  segments with indexed operands, 74 use both sizes, 40 only `.w` and 15
  only `.l`, of which only RIP_WIRED 0xE154A4 (5) and HINT_ 0xE49908 (14)
  have five or more; array bounds can explain those.  No `-indexl`
  segment is evident (Uncertain).
- **`-xrs`/`-nxrs`** (6.4.31, p. 6-36; `-xrs` the default): whether the
  compiler "believes that the contents of registers are saved across a
  call to an external routine or a call through a procedure-ptr".  The
  6534 values read from a register after a call (`keep`, Section 3) fit
  `-xrs`; no segment was checked for the `-nxrs` shape (every value
  reloaded after every external call).

| Level | Optimisations (ref) | What it looks like on the m68k | Detector (`tools/optsurvey.py`) |
|---|---|---|---|
| `-dba` (p. 6-27) | stores variables "in memory after every statement"; expressions arranged for fewest registers; short branches; constant expressions folded; one evaluation of an expression repeated within a statement | a store then a reload of the same slot | `streload` (-) ; `kfold` (-) |
| `-opt 0` (p. 6-27) | "values remain in registers across statements"; "Merges identical sequences of instructions" | a value read from a register after a store, join or call; a `bra` into another path's identical tail | `keep` (+), `reread` (-); `xjpop` (+), `duptail` (-) |
| `-opt 1` (pp. 6-27, 6-28) | some global CSE; dead code eliminated; multiplication by a constant as shifts and adds "(where appropriate)"; "simple transformations for speed"; "Merges assignment statements where possible" | a computed value reused across statements; no unreachable code; `lsl`/`add` products; merged adjacent stores (ref 3.15.3, p. 3-57) | `cse` (+); `deadbra` (-); `shmul` (+) / `mulimm`; `merge` (+) / `apart` (-) |
| `-opt 2` (p. 6-28) | "Substitutes constants for reaching definitions" | a constant stored to a variable and read back | `kreread` (-); `kfold` (-) |
| `-opt 3` (pp. 6-28..6-32) | redundant assignment elimination (warning 279); global register allocation; instruction reordering; invariant expressions out of loops; strength reduction; live analysis of locals; "extensive" global CSE; `%begin_inline` expansion | no store nobody reads; locals in registers; stepped pointers instead of `i * size`; no invariant computed inside a loop | `deadst`, `overst` (-); `lfrm` (-); `srstep` (+) / `srmiss` (-); `invld`, `invaddr` (-) |
| `-opt 4` (p. 6-32) | the compiler itself selects routines to expand inline | small routines with one call site gone | `inlcand` (-) |

(+) marks a sign that the optimisation was done, (-) that it was not.
Instruction reordering is left out: `tools/schedsurvey.py` (12.4 R9-R11)
found no ordering rule that the image does not contradict, so the image
cannot say whether reordering happened.  Global register allocation and
live analysis are measured in 12.5 (`tools/localsurvey.py`); here only
`lfrm`, a local kept in the frame inside a loop.

## 2. Method

`tools/optsurvey.py` (tests: `tools/tests/test_optsurvey.py`) takes the
routines of `tools/loopalign.py` in every compiled map segment (199
segments, 1682 routines; `_ASM` and the other hand-assembly segments left
out, section 0) and models the 1588 that `tools/regalloc.py` can model
(a `link`, no `case` jump table).  The store-merging counts come from
`tools/storesurvey.py` (`MERGED_SITES` of bead domain-pascal-2m0, and
`adjacent_pairs` for the constant pairs stored apart); copies are kept
apart from constants, because the review of 2m0 found that copies do not
follow the per-segment pattern (RIP_$TABLE 0xE6905E merges, RIP_$TABLE_D
0xE68ECA in the same segment does not).  The alignment class per routine
is `loopalign.align_classes`, the A5 letter `a5survey.letter`.  The
docstring defines each detector; `sites FEATURE` lists its sites.

Tables (regenerated by the tool, not in git):
`tmp/optsurvey/segments.tsv` (one row per segment: counts and a flag
string) and `tmp/optsurvey/routines.tsv` (one row per routine).

Statistics.  A rare feature's presence grows with a segment's size, so
plain 2x2 tables of segments (`cooc`) mostly measure size.  Two tests
keep the sizes:

- `conc`: is a flag more uniform within segments than chance?  The
  number of *mixed* segments (routines with and without it) against the
  same count with the routines' values shuffled over the segment slots
  (2000 runs).  One setting per compilation would give no mixed segment.
  Calibration: `cse`, which no level setting should turn on and off
  within one compilation, also comes out "more uniform than chance" (79
  mixed segments vs 94.2), so a `conc` "yes" reflects how source and
  routine kinds cluster in modules and is not evidence of a setting.
- `pairs`: do two flags go together by segment beyond what happens
  inside routines?  The weighted correlation of the segments' rates,
  against whole routines shuffled over the slots (1000 runs); p is
  two-sided around the null mean.  Flags held by fewer than 5 routines
  are skipped (`kfold` 1, `duptail` 3, `srmiss` 4, `deadbra` and
  `overst` 0): one or two routines gave spurious p before.  Knobs
  against every flag: 54 tests, 10 with p < 0.05 where 2.7 are expected
  by chance, so there is segment-level structure, driven by a few
  segments (Section 5).  The signs of a missing optimisation against
  each other (`--neg`): 35 tests, 1 with p < 0.05.

**Assumption (Uncertain): a map segment is one compilation.**  Section 7
argues that both sides of the alignment switch are one object module
(`bsr` never crosses a segment).  But one module has several segments
when its routines are in several code sections: TTY's 0xE1AED0 and
0xE671DC merge constant stores and TTY 0xE3324C (TTY_$I_INIT,
OS_INIT_PROC) does not.  Whether those are one compilation (a module with
`section` routine attributes, ref 5.7.1) or several files with the same
module name is not recorded.

## 3. Results per feature

Counts from `tools/optsurvey.py summary` / `conc` (image of SR10.2 SAU2).

### Seen across the image (no segment shows the opposite; many are too small to show either)

- **Values held in registers across statements (`-opt 0`; Verified).**
  10012 reads of a plain memory value from a register after a store, a
  join or a call since the load (6534 after a call), in every one of the
  141 segments that have such reads.  No segment stores and reloads as
  `-dba` describes: `streload` (a store, then a reload of the slot while
  the stored register still holds it) is 11 sites in 10 routines in 10
  segments, each also keeping values elsewhere: AREA_ module-local
  0xE085A6 (0xE085DA `move.w D1,(-166,A6)` ... 0xE085E8 `move.w
  (-166,A6),D3`), MAC_OS_$SEND 0xE0B5DC `move.l A3,(-72,A6); movea.l
  (-72,A6),A3`, NETWORK_$READ_AHEAD 0xE0FEEA, PMAP_ 0xE12EC2, ACL_$DELETE_PROJ
  0xE47FE6, PCHIST_$UNIX_PROFIL_CNTL 0xE5CAAE, RIP_$SEND 0xE871DE.
- **Multiplication by a constant (`-opt 1`; Verified).**  No `mulu.w`/
  `muls.w #k` has a k with fewer than three one bits (255 sites, the
  smallest k 92: RIP_RTWIRED 0xE872F8); products by shifts and adds are
  found in 57 segments, with k up to 257 (AREA_, two one bits) and one k
  with three (×26, NETLOG_$LOG_IT 0xE71BBA..0xE71BC4), while ×268
  (SMD 0xE1D7A2), ×336 (VTOC_ 0xE38C06) and ×580 (RING 0xE2FB04), also
  three one bits, use `mulu.w`.  So "where appropriate" is a cost rule
  not settled here (Uncertain), but no segment multiplies where another
  shifts.
- **Dead code (`-opt 1`; Verified).**  No unreachable `bra` follows an
  unconditional transfer in any modelled routine (`deadbra` 0), the shape
  an `exit` or `return` before an `else` would leave (section 7.5's
  threading drops them in `dpc` for this reason).
- **Constant expressions (`-dba`; Verified).**  One operator on two
  numeric constants in the whole corpus (`kfold`): DISK_$PV_ASSIGN
  0xE6C994 `ori.w #2,D2` where D2 was `moveq #5` on that path (a
  register-held variable set in an `if`, a reaching constant, `-opt 2`).
  Relocated `addi.l #0` and address arithmetic are left out.
- **Stepped pointers: strength reduction or the source's own pointer
  arithmetic (`-opt 3`; Verified that they occur; which of the two at a
  site is not known).**  364 innermost loops step a pointer by an
  element size and define it in no other way inside the loop (`srstep`;
  a loose count of any step was 426, among them source pointer chains
  such as AST_ 0xE01D9E), in 93 of the 128 segments with inner loops;
  the other 35 have no loop that scales its induction variable for an
  index either.  `srmiss` (an induction
  variable scaled in the loop for an address) is 4 loops, and every one of
  them also steps a pointer in the same loop: the index there is the
  induction variable plus a loop-invariant *variable*, `(i - lo) * 4` or
  `(i + k) * 12`, which is left unreduced while `base + i * size` is
  reduced (FILE_$FW_PAGES 0xE5E7AA..0xE5E7B0, RING module-local
  0xE76CF6..0xE76D02, SMD module-local 0xE6E54C..0xE6E558, DIR
  module-local 0xE4F350..0xE4F356).  A limit of the optimiser, not a
  lower level (rule Reported from these 4 sites).
- **Global CSE (`-opt 1`/`-opt 3`; Verified at the sites of 12.4 R7).**
  A computed value reused after a store, a join or a call: 2937 sites in
  472 routines of 97 segments.  The 84 segments without one are small
  (at most 10 modelled routines each).

### Signs of a missing optimisation (rare)

| Feature | Routines | Segments | More uniform by segment than chance (`conc`) | Example sites |
|---|---|---|---|---|
| `reread`: a plain value loaded again while a register still holds it and nothing since may have written it | 60 (113 sites) | 25 of 141 | yes (23 mixed vs 34.0, p < 0.001) | the breakdown below |
| `kreread`: a constant stored to a frame slot, then read from it | 25 | 11 | yes (11 vs 18.9) | AST_ (10 routines): 0xE02FEE `clr.l (-4,A6)` ... 0xE02FF8 `move.l (-4,A6),-(SP)`; PROC2_$AWAKEN_GUARDIAN 0xE3E99A/0xE3E9A2 |
| `deadst`: a store to an unescaped frame slot that nothing reads | 49 | 23 | yes (22 vs 31.1) | FIM_$GET_USER_PC 0xE0AAB0, PKT_$NEXT_ID 0xE124A8, NET_$OPEN 0xE5A1D8: a value stored to its local and used from the register (the "stored locals" of 12.5) |
| `invld`: a frame slot or A5 offset the loop never writes, loaded inside it (`invld_nc`: a loop without calls or stores through pointers) | 34 (24) | 19 (12) | weakly (p 0.05; 0.02) | EC2_$REGISTER_EC1 0xE4296E `movea.l (8,A6),A1`; LOG_$ADD 0xE176BC; DIR (6 routines) |
| `invaddr`: a constant address formed inside a loop with no call (76 loops counting all, 36 call-free) | 29 | 11 | yes (11 vs 18.9) | PROC2 (12 of the 36), FILE_ (6), MMAP_ (4).  Mostly the absolute array base `movea.l #abs,An; lea (An,Dn.l)` (AST_$TOUCH 0xE03422 `movea.l #0xEB4800,A0`, PROC2 0xE40252): a register choice on the 68010 more than a hoist left undone; TTY_$K_PUT vs TTY_$K_DRAIN_OUTPUT (7.4) |
| `lfrm`: a frame local read and written in a loop | 17 | 14 | weakly (p 0.02) | MMAP_ (3), LOG_$ADD, DISK_$SORT |
| `duptail`: identical last instructions on two paths into a join, not merged | 3 | 3 | no | TERM_$GET_REAL_LINE 0xE1A9F0/0xE1AA00 `move.w 0xE2E00E,D0`; FILE_$PRIV_LOCK 0xE5F6E4/0xE5F6FA (the same `move.l 0xE245A4,(-36,A6)` before two `bra`s to 0xE5F716); GET_BUILD_TIME 0xE381D2 (the same `jsr 0xE825E4` on two paths) |
| `srmiss` (above) | 4 | 4 | no | |
| `inlcand`: a module-local routine of 0x30 bytes or less with a frame, one `bsr` site, not named elsewhere | 7 | 7 | no | KBD 0xE1CA62 (a lookup loop), DISK_ 0xE3CCCE, PROC2 0xE4232A, ACL_ 0xE46498, OS 0xE6D222, AUDIT 0xE7128A, RING 0xE76470 |
| `overst`: a frame store overwritten in the block before any read | 0 | 0 | | |

Cross-jumping is also seen done: 34 calls in 28 routines of 19 segments
end in a `bra` to a pop at a join (`xjpop`, the shared pop of section
1.1), against 3 unmerged tails.

**These signs do not form a "lower level" group (Reported; a survey with
little power).**  `pairs --neg` (rare flags skipped): of 35 pairs, one
has p < 0.05 (`reread` with `streload`, r = +0.42 vs +0.15, p 0.009),
about what chance gives.  Most of these flags are held by 10 to 60
routines, so the test could miss a real link.  At routine level some do
go together (`cooc --routine`, Fisher): `reread` with `streload` p
0.0007, `reread` with `invld` p 0.0035, `deadst` with `lfrm` p 0.007,
likely routine size and kind (long routines with loops and many frame
locals), not a setting.  Their clustering by segment (AST_ has 10 of the
25 `kreread` routines, PROC2 12 of the 36 call-free `invaddr` loops)
fits the source (one author's idiom repeated: `x := nil; P(..., x)` in
AST_) as well as a setting, and each is explained at its sites without
one:

- `reread`, 113 sites by what lies between the two loads (ad-hoc script
  `tmp/g728/rr3.py`):
  - nothing (49): 39 indirect, mostly uplevel variables through
    `(8,A2)` with A2 = `(A6)`, the parent's frame, loaded again within
    one argument list (AST_ 0xE0153E/0xE01546; 12.4 R8: uplevel
    variables are re-read even within one statement); 10 frame pointer
    parameters loaded again at a new statement (0xE01584, IO_$ACQUIRE
    0xE1A590, 0xE1A5D2, 0xE1A5E6);
  - a store to another fixed location (37; 12.4 R8, "plain variable
    reads are not kept across statements");
  - only a join or a back edge (21);
  - a store through a pointer (6: DMA_ 0xE0A33A..0xE0A356, each before a
    `device` store; 0xE019B6; LOG_$ADD 0xE176BC).
- `deadst`: the same "stored local" idiom (assign, then use the
  register) in 23 segments; redundant assignment elimination at `-opt 3`
  would have removed the store (ref p. 6-28), so either it does not
  apply to these locals or the kernel was compiled differently
  everywhere (Uncertain).
- `kreread`: constant propagation into a pushed argument is not done;
  with only 25 routines and one other site of the shape (`kfold`), it is
  not known whether `-opt 2` propagation reaches pushes at all.

## 4. Levels

- **Below `-opt 1`: no segment shows evidence of it; 54 cannot tell
  (Reported).**  127 segments show an optimisation of `-opt 1` or higher
  (`tell`); the others have too little code to show one, and 18 are not
  modelled.  No segment shows an `-opt 0` or `-dba` signature: `mulimm`
  never has a constant below three one bits, `deadbra` is 0, `streload`
  is 10 routines in 10 segments that also keep values elsewhere.
- **`-opt 1` or `-opt 2` but not 3: no segment shows evidence of it
  (Reported, as far as the detectors go).**  Stepped pointers are in 93
  of the 128 segments with inner loops; the other 35 have no
  scaled-index loop either, so they cannot tell.  Register-held locals
  and `dbf` counters in registers are the rule in the surveys of 12.5
  and 7.4 (not counted per segment here; `lfrm`, a local kept in the
  frame inside a loop, is 17 routines).  The 4 `srmiss` loops are a
  pattern limit inside reduced loops.
- **`-opt 3` vs `-opt 4`: not decided (Uncertain).**  The 7 inline
  candidates kept as called routines, one per segment, argue against
  `-opt 4` there, unless `%begin_noinline` (ref 4.7) excluded them; the
  other 192 segments show nothing either way.
- **`-dba`: no segment shows evidence of it (Reported).**

## 5. The per-module knobs

- **Store merging (constants)**: 20 segments merge, 29 keep pairs apart,
  5 both (2m0).  Routine-level uniformity by segment is weak (5 mixed
  segments vs 7.9 by chance, p 0.08).  Against the alignment class: the
  merge-only segments are 6 aligning, 3 not, 2 mixed, 4 unknown; the
  apart-only 16, 2, 10, 1: no coincidence (routines: Fisher p 0.06,
  more merges in non-aligning routines).  Against A5 mode (`a5table`,
  named routines): of the merge-only segments that load A5 and have an
  `n` or `s` routine, 7 of 9 are purely in used mode, against 2 of 16
  apart-only ones (Fisher p 0.002; `pairs` r = -0.41, p 0.008).  Only 12
  routines can be classed by both, and the one merging routine among
  them, TTY_$K_RESET 0xE672DE, is in `always` mode, so this is a
  segment-level lead only (Uncertain; one of 54 tests).  A pair kept
  apart may also be a `device` object (Section 1).  No `-opt` feature
  separates merging from non-merging segments (`pairs`: merges has
  p < 0.05 only against `cmerges` and `a5always`).
- **Loop alignment** (section 7): per routine, in runs; 24 mixed
  segments, far fewer than chance (65.7), as section 7 found.  At
  segment level, aligning segments have fewer invariant loads in
  call-free loops (r = -0.37, p < 0.001), fewer rereads (p 0.02) but
  also fewer cross-jumped pops (`xjpop`, a sign of optimisation, p
  0.02).  By routine and size, `invld_nc` is 2 of 263 medium (0x80..0x200
  bytes) aligning routines against 8 of 278 non-aligning; large ones
  4 of 56 against 8 of 91.  The directions disagree and the effects are
  small: alignment does not follow an `-opt` level (Reported: the switch
  is inside one module, section 7, and the reference ties no alignment
  to `-opt`).
- **A5 mode** (section 4): per routine, in runs (20 mixed vs 37.3 by
  chance).  Associated at segment level with call-free `invaddr` (r =
  +0.47, p < 0.001), driven by a few segments (PROC2 0xE3E748 with 48 `s`
  routines and 12 of the 36 loops; FILE_; MMAP_), and with loop
  alignment (r = +0.11 against a null of -0.11, p 0.016).  Neither
  `invaddr` (an addressing idiom, Section 3) nor alignment marks an
  `-opt` level.

**Consequence for `dpc` (recommendation).**  Keep the per-unit knobs as
they are (image-derived: `--loop-align-off-from`, `--a5-always-from`,
and, if added, a store-merging option for constants), each set from the
image per unit.  The survey found no `-opt` level that would set them,
and every optimisation `dpc` reproduces (shift-and-add products,
strength reduction, CSE, register-held locals) is common to all
segments.

## 6. Limits and counterexamples

- Detectors are syntactic over the image; each was checked at its sites
  listed above, not exhaustively.  `invld` cannot tell a variable the
  source re-reads on purpose (`volatile`, `device`; ref 3.15) from an
  invariant left in the loop; `reread` likewise.
- 94 routines are not modelled (47 with a `case` jump table, 47 without
  a `link`); their merges, alignment and A5 class are still counted.
- The reference is the December 1990 edition; whether the compiler that
  built SR10.2 had the same levels is not recorded.
- Counterexamples to "one setting per segment" are those of the earlier
  surveys: alignment and A5 switch inside segments (sections 4, 7), the
  constant stores of ACL_ 0xE44C3C, ASKNODE, FLP_, WIN_ and XNS_ERROR
  merge and stay apart in one segment (2m0), and TTY's segments differ.

## 7. Commands

```
tools/optsurvey.py summary          # totals, flags by segment
tools/optsurvey.py segments [-v]    # per-segment rows; tmp/optsurvey/segments.tsv
tools/optsurvey.py routines [SEG]   # per-routine rows; tmp/optsurvey/routines.tsv
tools/optsurvey.py conc             # uniformity by segment
tools/optsurvey.py pairs [--neg]    # segment-level association beyond routines
tools/optsurvey.py cooc [--routine] # plain 2x2 tables (size-confounded)
tools/optsurvey.py a5table          # constant merging against A5 mode
tools/optsurvey.py tell [-v]        # segments by the level their code shows
tools/optsurvey.py sites FEATURE    # e.g. sites srmiss
```
