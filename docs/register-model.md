# A model of the original compiler's register use (design note)

Status: **hypothesis, partly tested** (step 2, 2026-11-04).  Nothing here is implemented as
described.  It is the plan for replacing a growing set of narrow
register rules with two general mechanisms.  Every claim about the
image cites `docs/codegen-patterns.md`, which holds the evidence; this
note only connects the pieces.

## Why

`dpc` reproduces the original's register use through many separate
rules, each fitted to a few image sites.  Each is right where it was
fitted, but they overlap, interact in ways no one designed, and keep
growing.  A compiler of the 1980s would not have been written that way.
The reference names the general features it has (ref 6.4.24, pp.
6-27..6-32; codegen-patterns 12.4):

- "Permits values to remain in registers across statements" (`-opt 0`);
- "Computes multiple occurrences of the same expression within a
  statement only one time" (`-dba`);
- "Eliminates some global common subexpressions" (`-opt 1`);
- "Global register allocation allows local variables to have their
  values placed in machine registers" (`-opt 3`, the default, p. 6-29).

The hypothesis is that most of `dpc`'s register rules are facets of two
mechanisms that match those descriptions:

- **A. A register-contents cache.**  The code generator records which
  register holds the value of which memory location or expression, and
  reuses it until something invalidates it.
- **B. Global allocation of locals.**  Eligible variables are candidates
  for registers, and a colouring allocator with a spill cost decides
  which stay in memory.

## A. The register-contents cache

### The model

An entry says "register R holds the value of K".  K is one of:

- a **variable in memory**: a frame slot, a variable of the unit's data,
  another unit's variable, or an object through a pointer held in a
  register;
- a **computed expression** over such variables: a widening, a scaled
  index, a product by a constant (R7's kinds).

An entry is **created** when:

- a load puts a variable into a register;
- a store writes a register to a variable (the register then holds it);
- an expression is computed.

An entry is **used** for a non-destructive read: a compare, a test, a
push, or a base or index of an address.  A destructive use, such as
two-address arithmetic into the register, copies a computed value (it
has no memory home) but reloads a variable from memory.  This is
**Uncertain**; see P4.

An entry is **killed** by:

- a write of its register;
- a store that may alias K (R7's `store_may_write` rules);
- a label that a branch from before targets (a join);
- a loop's back edge;
- a call, for scratch registers.  Whether entries in callee-saved
  registers survive a call is open (P5).

### What it subsumes in `dpc` today

| `dpc` rule (code) | codegen-patterns | As a cache effect |
|---|---|---|
| Compare right after a store reads the register (`Selector::compare`) | 7.1, "A compare with a nonzero constant right after a store" (23 / 0) | store-created entry, used by the next compare |
| Stored locals pushed from the register (`Selector::forwarded`) | 12.5, "Stored locals pushed from the register" (86 / 5 with no call) | store-created entry, used by a push; killed by a call (38 / 10) |
| Operator results reused across statements (`cse_value`, `cse_valid`, `store_may_write`) | 12.4 R7 | expression entries, with R7's kill rules exactly |
| One-variable compare chains (`chain_variable`, e71f68b) | 7.1, "A chain of comparisons of one variable" (88 / 4) | load-created entry used by the chain's later compares |
| Within-statement double reads | 12.4 R8 (0xE0283C reads `(2,A2)` once for two compares) | the same, inside one statement |
| A framed pointer local dereferenced through A0 while A0 holds it (`a0_holds`) | 3, 12.5 (bead domain-pascal-500) | store-created entry from a call's A0 |
| Load flags used by the test after it (`assigned_flags`, the chain's first test against 0) | 7.1, "Flags of a load" | the condition codes as one more cached fact |
| Another data section's block reloaded after a call (`other_base`) | 11 | an address entry killed by a call |
| Frame-cached locals (not modelled; bead domain-pascal-g67r) | 12.5; ../source's register-allocation survey, 37 sites | a slot a callee wrote, loaded once; the entry then lives across statements |

### Predictions to test against the image (step 2)

- **P1. Labels kill entries.**  AST_$PURIFY's `(chunks = 0 and then s
  = more) or else s = incompat` compares `s` in memory twice
  (0xE05B14, 0xE05B1E), the one memory-form chain the 88 / 4 scan could
  not explain.  0xE05B1E is the target of the `bne.s` at 0xE05B12: the
  entry would be killed there.  **Checked** at this site.  Still to
  check: ASKNODE_$WHO_REMOTE 0xE6668E.
- **P2. Plain-load entries and statement boundaries.**  R8 says plain
  variable reads are not kept across statements: PROC1_$AS_ID is re-read
  for each use (FIM_$ACKNOWLEDGE 0xE0A972, 0xE0A980, 0xE0A99A).  Yet
  frame-cached locals are loaded once and reused across statements.
  Candidate split: frame slots stay cached across statements, while
  memory through a pointer or in another unit is re-read.  R8's sites
  are another unit's variable and an object through a pointer.  To
  check: every R8 site and every frame-cached site, by kind of memory.
- **P3. Store-created entries live across statements.**  Forwarded
  pushes say yes (86).  To check: the 5 slot-pushed exceptions (0xE02D6E,
  0xE141CA, 0xE611E2, 0xE6B828, 0xE771F4) for a kill this model
  predicts, such as a label or an aliasing store.
- **P4. A destructive use reloads a variable rather than copying its
  cached register.**  0xE085E8 `move.w (-0xa6,A6),D3` after `move.w
  D1,(-0xa6,A6)` at 0xE085DA, with D1 unchanged, then `lsr.w #3,D3`
  (also 0xE30D5A, 0xE871DE).  A computed value, which has no memory
  home, is copied (0xE177D0 `move.l D1,D3; add.l (0x14,A5),D3`).  To
  check: the count of each form.
- **P5. Calls.**  R7 counts, after a call, sums 25 reused against 162
  computed again, and sign extensions 11 against 6.  Under the model,
  entries in scratch registers die and entries in callee-saved
  registers may survive.  To check: whether every reuse after a call is
  in D2-D7/A2-A4 and every recomputation was in D0/D1/A0/A1.  If so,
  "calls kill scratch entries" is the whole rule.

### Step 2 results (2026-11-04)

**Method.** `tools/cachesurvey.py` walks each compiled routine
(first instruction `link`, no `jmp` table).  It tracks entries "register
R holds memory operand M" created by loads and stores, and kills them at
writes of R, at writes of M or of memory that may alias it, and at writes
of a register M's address uses.  It does *not* kill at joins or calls; it
flags them instead, so the counts show what the original did there.  A
join flags an entry only when some branch into it comes from before the
entry was created, or from after the join (R7's rule).  Calls kill the
scratch registers.  At each later use it records a *reuse* (the register
is read for M again) or a *reread* (M is read from memory while a
register still holds it), by the kind of use and by the entry's origin.

It runs over the image (`tools/alldis.py 2 68000`; `9 68020`) and over
dpc's objects for the same routines (`tools/objdis.py`, ../source
096ca5c7 built with dpc e71f68b): 1164 routines on SAU2, the same count
on SAU9.  The image counts alone mix the cache with mechanism B: a local
held in a register looks like a cached slot.  The dpc counts are the
control.  dpc already models B to 96.8% (`tools/regalloc_check.py`), so
what differs between the two is mostly the cache.

**Straight-line code** (no killing join, no call between), the image
against dpc, reuse / reread.  Frame slots, parameter slots and objects
through a pointer; unit and absolute variables show the same shape on
smaller counts:

| entry from | use | SAU2 image | SAU2 dpc | SAU9 image | SAU9 dpc |
|---|---|---|---|---|---|
| a store | push | 83 / 4 | 23 / 25 | 83 / 7 | 23 / 25 |
| a store | operand of an operation | 43 / 0 | 5 / 0 | 63 / 0 | 10 / 1 |
| a store | stored elsewhere (`move Rn,<mem>`) | 67 / 3 | 22 / 3 | 53 / 3 | 24 / 4 |
| a store | compare / test | 44 / 11 | 31 / 18 | 65 / 8 | 43 / 24 |
| a store | into another register, changed in place | 20 / 8 | 5 / 45 | 20 / 9 | 5 / 55 |
| a store | into another register | 40 / 16 | 13 / 58 | 30 / 15 | 6 / 63 |
| a load | compare / test | 295 / 4 | 165 / 6 | 305 / 3 | 177 / 11 |
| a load | bit test | 62 / 1 | 12 / 3 | 104 / 10 | 42 / 9 |
| a load | operand of an operation | 39 / 0 | 6 / 5 | 65 / 1 | 8 / 6 |
| a load | into another register | 45 / 89 | 18 / 313 | 44 / 111 | 14 / 384 |
| either | read-modify-write of the memory itself | 3 / 43 | 1 / 23 | 12 / 30 | 1 / 15 |

What the rows say:

- **F1. A stored value stays in its register and serves later reads**
  (P3 confirmed).  After an assignment stores register R into M, the
  image reads R for M at 88% of later uses (305 / 42 on SAU2), of every kind: pushes,
  operands, copies to other memory, tests, and copies into another
  register.  dpc does this only for a push or a compare right after the
  store (12.5, 7.1): its pushes are 23 / 25, and its copies 5 / 45 and
  13 / 58.  This is the largest gap.
- **F2. A loaded value serves the uses of its own expression, but a
  later load rereads memory** (P2 replaced).  A load-created entry is
  reused by compares, bit tests and operands (295 / 4, 62 / 1, 39 / 0).
  But where the value is wanted in a register again, the image mostly
  loads it from memory (45 / 89).  That is R8, "plain variable reads are
  not kept across statements" (AST_$INVALIDATE 0xE066BC `movea.l
  (-20,A6),A0` again with A0 unchanged since 0xE066AE).  It holds even
  between the arguments of one call (AST_$UPDATE 0xE0191C, 0xE01924,
  0xE01930 load `(8,A2)` three times).  So a plain load's entry lives for
  its expression.  The kind of memory does not matter: frame slots 4 /
  34.  The P2 split (frame slots cached, other memory not) is
  **refuted**.  The frame-cached locals (bead domain-pascal-g67r) need
  another explanation, probably B (a local becoming a candidate once its
  address no longer escapes).
- **F3. Destructive uses copy** (P4 refuted as stated).  A cached value
  needed in another register to be changed in place is copied far more
  often than reloaded (20 / 8 from stores, 26 / 3 from loads on SAU2).
  0xE085E8 and its like are the exceptions, not the rule.
- **F4. Joins** (P1 consistent).  Across a join that some path enters
  without the entry, the image's reuse rates match dpc's (frame stores
  81 / 82 against 25 / 44; pointer loads 69 / 36 against 46 / 25).  The
  reuses there are mechanism B's held variables, which dpc reproduces;
  nothing shows a cache entry surviving such a join.  AST_$PURIFY
  0xE05B1E is the direct case.
- **F5. Calls** (P5 not settled).  Entries in callee-saved registers
  mostly survive a call: SAU2 frame stores 31 / 9, pointer loads 24 / 2,
  frame loads 17 / 3.  Those counts mix in B's held locals, so "calls
  kill only the scratch registers' entries" is not yet separated from
  B.
- Read-modify-write instructions work on the memory (`addq.l #1,(d,A6)`,
  `bset #n,(An)`) even while a register holds the value (memory 43
  times, the register 3 on SAU2).  That is a form choice, not a cache
  miss.

**The revised model of A.**

- Entries created by **stores** (assignments) live until killed: a
  write of the register, an aliasing store, a join entered from before,
  or a call for a scratch register.  This is the reference's "permits
  values to remain in registers across statements".
- Entries created by **loads** live for the expression that loaded
  them (F2), as do common subexpressions within it (ref p. 6-27,
  `-dba`).
- **Computed values** (R7) live across statements, under R7's kill
  rules.
- A use that changes the value in place copies the cached register
  (F3).  Read-modify-write instructions use memory.

## B. Global allocation of locals

### The model

- **Candidates:** scalar locals of 1, 2 or 4 bytes and pointers whose
  address is not taken, not read in part and not reached by a nested
  routine; `for` indices; parameter values and parameter addresses
  (12.2); held expression values (A); stepped pointers (7.4).
- **Colouring:** a candidate that crosses a call needs a callee-saved
  register; otherwise D0/D1/A0/A1 are allowed (12.4 R1, R3; the survey:
  79% of scratch-held locals cross no call).  The colouring order is not
  identified (12.4 R1a: no published order explains it).
- **Spill cost:** a candidate stays in memory when a register gains it
  nothing, or when it loses the colouring.
  - Gains nothing: a value only pushed or passed as `p^` is as cheap
    from the frame (12.5 exception (a); `register_locals`'s "every read
    pushes it", 125 / 5).
  - Loses the colouring: under pressure, the candidate with fewer
    references is spilled (33 / 13 on SAU2, 11 / 12 on SAU9: weak).

### What it subsumes in `dpc` today

| `dpc` rule (code) | codegen-patterns | As an allocation effect |
|---|---|---|
| Data locals in registers (`register_locals`) | 12.5 | candidates; push-only ones gain nothing |
| Pointer locals assigned from calls (`call_pointer_locals`) | 3, 12.5 | candidates |
| Loop pointer locals (`loop_pointer_locals`) | 12.5 | candidates |
| Other pointer locals (`free_pointer_locals`, exceptions (a) and (b), `forwarded_push`) | 12.5, "Pointer locals, both CPUs" | candidates; (a) gains nothing; `forwarded_push` is a one-use candidate |
| Dropping the fewest-read pointer local when the body does not colour (`DROPPED_PTRS`) | 12.5 | the spill choice |
| Parameters loaded at entry vs. reloaded | 12.2 | parameter values and addresses as candidates (survey: "no rule fitted") |
| Holds through calls | 12.4 R4 | live ranges longer than liveness: allocation granularity |
| Value-parameter copies held | 1.2 | candidates |

### Open questions

- The colouring order (12.4 R1a) and the spill cost under pressure.
- **Register class.** 11% of SAU2's held pointer locals are in data
  registers, and some parameter addresses are kept in D registers and
  copied to A registers at each dereference (MAC_OS_$OPEN 0xE0B246
  D6/D7).  This looks like a class choice by use counts: copies and
  pushes against dereferences.
- Whether A's cache and B's candidates are one structure.  Code
  generators of the period commonly kept a register descriptor (A) and
  added global allocation (B) on top; that is background, not evidence,
  and the image cannot tell.

## Plan

1. **This note** (done).
2. **Test P1-P5 against the image** (done 2026-11-04: "Step 2
   results" above; `tools/cachesurvey.py`).  P1 consistent, P2 replaced
   by F2, P3 confirmed (F1), P4 refuted (F3), P5 open (F5).
3. **Build A as one pass** in the selector: a register descriptor
   updated per instruction, with the revised model's lifetimes and kill
   rules.  F1 is the largest gap (store-created entries serve every kind
   of read), so it comes first.  Then retire the
   special cases of A's table one at a time.  Each retirement must keep
   the 149 conformance routines byte-identical, the goldens unchanged
   (or changed with a reason) and `tools/regalloc_check.py` no worse.
   Measure the whole tree with `routine_compare.py` before and after.
4. **Fit B's spill cost** against ../source's per-local tables
   (`tests/regalloc/sau{2,9}_high.tsv`), replacing the exceptions of
   B's table with the cost.

Beads: epic domain-pascal-suii; domain-pascal-w4dm and domain-pascal-g67r (the open items that
started this) wait on steps 2-3.
