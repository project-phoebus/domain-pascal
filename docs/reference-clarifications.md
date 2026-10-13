# Owner clarifications of the Domain Pascal Language Reference

The reference in this directory
(`000792-A01_Domain_Pascal_Language_Reference_Dec90.txt`) is OCR output, and
some passages are garbled.  When a passage is unclear, agents do this:

1. Read the same page in the searchable PDF:
   `~/src/domainos-archeology/docs/apollo-docs/ocr/000792-A01_Domain_Pascal_Language_Reference_Dec90.ocr.pdf`.
2. If it is still unclear, ask the owner.  Quote the passage and give its
   section and printed page number (e.g. "5.2, p. 5-4").
3. Record the owner's answer here.  Do not guess.

Add one entry per question, newest last, in this format:

```
## <section> <title> (p. <printed page>; PDF page <n>)
Asked: <date>, by <who>
Passage: "<quoted OCR text>"
Question: <what was unclear>
Owner's answer: <verbatim or close paraphrase>
Consequence: <what the compiler/tests do because of it; files touched>
```

## Pages flagged by OCR

The OCR log
(`~/src/domainos-archeology/docs/apollo-docs/ocr/000792-A01_Domain_Pascal_Language_Reference_Dec90.log`)
flags these **PDF** page numbers.  They are not the printed page numbers.

- **"Possibly poor OCR" (lots of diacritics):** 14, 99, 112, 126, 131, 160,
  172, 240, 291, 293, 349, 357, 411, 473, 505, 552, 555, 562, 584, 595,
  603, 611.
- **Empty:** 11, 543.

Check the PDF before you rely on text from these pages.

## Clarifications

(none yet)

## Open questions (awaiting the owner; tracked in domain-pascal-vui)

Conservative choices are implemented and marked `TODO(ref-question)` in
`crates/dpc-syntax/src/`.

1. **4.7, p. 4-45.** "Unlike the other compiler directives, conditional
   directives cannot be used as comments."  Is `{%if ...}` an error or an
   ordinary comment?  *Current: error.*
2. **4.7, p. 4-50, `%var` / `%enable`.** "`%VAR code.old code.new`": the
   reference never says how the name list ends.  `module` is not reserved,
   so `%var color` followed by `module m;` is ambiguous.  *Current: names
   on the directive's own line, then an optional `;`.*
   *Evidence note (Apollo's SR10.2 customer inserts, `~/src/domainos-archeology/sr10.2-install/install/ri.apollo.os.v.10.2/sys/ins/`;
   not an owner ruling):* two of them guard against double inclusion
   with the whole conditional on one line, the `%var` list ended by the
   next directive and no `;`: `gpr.ins.pas` "`%IFDEF gpr_$gpr_ins_pas
   %THEN %EXIT %ELSE %VAR gpr_$gpr_ins_pas %ENDIF`", and
   `gsr_types.ins.pas` "`%IFDEF gsr_$$gsr_types %THEN %EXIT %ELSE %VAR
   gsr_$$gsr_types %ENDIF`", which also has an empty `%THEN` part:
   "`%IFDEF ISP_$M68K %THEN %ELSE %VAR ISP_$M68K %ENDIF`".  Others use
   one directive per line (`prsvr.ins.pas`: "`%IFDEF prsvr_ins_pas
   %THEN` / `%EXIT` / `%ELSE` / `%VAR prsvr_ins_pas` / `%ENDIF`").
   `dpc --check-only` accepts all three forms (checked).  This shows
   that a directive ends a `%var` list on the same line; it does not
   settle how a list ends before an ordinary token such as `module`.
3. **4.7, p. 4-46, predicates.** "Optional Boolean keywords not, and, or
   or": no precedence is given.  *Current: Table 4-3 order, not > and > or.*
4. **2.1.2, p. 2-2 (PDF p. 35).** The integer example uses
   `/* default (base 10) */` comments, but 2.1.4 lists only `{ }`,
   `(* *)` and `" "`.  Is `/* */` a comment form?  *Current: rejected,
   per 2.1.4.*
   *Evidence note (not an owner ruling):* none of the 56 `.ins.pas`
   files of Apollo's SR10.2 customer inserts (`~/src/domainos-archeology/sr10.2-install/install/ri.apollo.os.v.10.2/sys/ins/`) contains `/*`;
   they use `{ }`, and `(* *)` in 3 files (e.g. the header of
   `prsvr.ins.pas`).  Consistent with 2.1.4; it does not rule `/* */`
   out.

Raised by the parser work (domain-pascal-0sp.4; no bead filed yet):

5. **4.7, p. 4-43.** Directives may appear "anywhere a comment is valid".
   What does an alignment directive in the middle of a declaration mean
   (e.g. `%natural_alignment;` between two fields of a record)?  *Current:
   a directive inside a declaration item or routine heading is an error;
   between declarations it is kept in order; inside a statement part it
   takes effect after the routine.*
6. **Table 4-3, p. 4-4.** The precedence table lists `not` but not `~`
   (bit not, p. 4-33), nor unary `-`/`+`.  *Current: `~` binds like `not`;
   a sign applies to the first term of a simple expression (standard
   Pascal).*  Partly settled by "Expressions", p. 4-82 (checked in the
   PDF): "You cannot follow a binary operator with a unary operator of
   lower precedence", with `9 DIV -3` and `5 * -100` marked improper, so
   `a * -b` is rejected as implemented.  Still open: whether a sign may
   follow an operator of the *same* level (`a + -b`; currently rejected),
   and where `~` sits.
7. **Case, p. 4-36.** A constantlist is "One or more values (separated by
   commas)".  Are ranges (`1..3:`) allowed in case labels?  *Current:
   rejected.*
8. **7.1.1, p. 7-2.** The empty-code-section form `name, , data_section`
   is shown for programs (2.2.1, p. 2-10) but not for modules.  *Current:
   accepted for programs only.*

Raised by the semantics work (domain-pascal-0sp.5; no bead filed yet).
Conservative choices are implemented and marked `TODO(ref-question)` in
`crates/dpc-sema/src/`.

9. **5.2 NOTE, p. 5-4 vs. 5.5.7, p. 5-19.** p. 5-4: "All character arrays
   are treated like arguments with fewer than four bytes, regardless of
   their size."  p. 5-19: "all arguments four bytes or less in size (on
   680x0 workstations) ... (except for character arrays) are passed by
   value".  Under `val_param`, does a character array go by value or by
   address?  *Current: by address (5.5.7; codegen-patterns 1.2).*
10. **5.5.7, p. 5-19.** "all arguments four bytes or less in size ... are
    passed by value".  Does this include records and non-character arrays
    of 4 bytes or less, or are records passed by value only under
    `c_param` (5.5.12, p. 5-21: "In addition, the c_param option tells the
    compiler to pass all record data types by value")?  *Current: the
    literal reading; small records and arrays go by value under
    `val_param`.*  Image evidence so far does not settle it: the
    module-local worker the C calls `smd_$cursor_op` (0xE6DFFA, a by-value
    convention routine) receives a 4-byte position by value from
    SMD_$DISPLAY_CURSOR (`movea.l (0xc,A6),A0; move.l (A0),-(SP)` at
    0xE6E092..0xE6E096, call 0xE6E09E), but its Pascal type (record or
    integer32) cannot be seen: the worker only passes `pea (0xa,A6)` on
    (0xE6E05E).  No `../source` prototype passes a `_Static_assert`ed
    2- or 4-byte struct by value.
11. *Settled by the reference (no owner ruling needed).*  An unsigned
    subrange whose upper bound is in 32768..65535 is 16 bits.  5.3.4.1,
    p. 5-13 (PDF p. 368): "pinteger = 0...65535; ... The type pinteger is
    defined as a subrange of base type integer. ... Even though pinteger
    and integer are both 16-bit integers".  Also Table 7-2, p. 7-30 (PDF
    p. 455): "0..65335 [sic] | unsigned short"; and the varying-string
    record, p. 3-44, "length : 0..65535", which p. 3-46 calls "a 2-byte
    length field".  3.4, p. 3-10 ("large enough to require 31 bits, the
    compiler allocates 32 bits") covers only the 32-bit case.
    *Implemented: such a subrange has an integer32 host in the checker
    (its values do not fit integer16) and 2-byte storage
    (`crates/dpc-sema/src/types.rs`, `layout_of`; test
    `unsigned_16_bit_subranges_are_words`).*
12. **5.6.4, p. 5-22.** default_routine_options apply to "every subsequent
    routine in that module that does not specify a routine option".  Does a
    routine that specifies only `extern`, `forward` or `internal` (which
    5.6.1, p. 5-22, bars from routine_option parts) still get the
    defaults?  *Current: yes; only interface options cancel the defaults.*
13. **7.2, p. 7-3.** Is a module-level variable declared with no
    allocation clause (and no section) exported, like `define`, or local,
    like `static`?  *Current: allocated in the module's data section,
    local symbol.*  7.2 (p. 7-3) lists `define` as the clause that makes a
    name "externally accessible", and Method 4 (7.2.4, p. 7-7) shares plain
    `var (my_sec)` variables between modules by section, not by name
    ("There's no requirement that the variables in Sub2 have the same
    names"), which fits "local" but is not stated for the default `.data`
    section.
14. **"Pointer Operations", p. 4-144.** "Compare or assign the value of one
    pointer variable to another compatible pointer variable": are two
    pointer types compatible when they point to the same type, or only
    when they are the same named type (p. 4-207 says the latter for
    pointers to varying strings)?  *Current: same target type.*
15. **"Lshft", p. 4-109 (printed 4-108/4-109).** The example uses
    `i : INTEGER16` and shows `LSHFT(i, 7)` giving "2#1110100000000000 =
    10#-6144", i.e. a 16-bit result.  Unlike "Rshft" (p. 4-171) and
    "Arshft" (p. 4-29), the entry does not say whether a 16-bit operand
    is expanded to 32 bits first.  When the result is used as an
    integer32 (`l := lshft(w, 7)` with `w : integer16`), is it -6144
    (16-bit shift, then sign-extend) or 256000 (32-bit shift)?
    *Current: refused (review of domain-pascal-5aw.13).*

Raised by the built-ins work (beads domain-pascal-5aw.7, -0wc; no bead
filed for the questions yet).  Conservative choices are implemented and
marked `TODO(ref-question)` in `crates/dpc-sema/src/check.rs`.

16. **Appendix E, "Set_sr", p. E-5 (PDF p. 568, checked).** "oldsr :=
    set_sr(newsr); {set_sr is a function.}  ... oldsr The old value of the
    hardware status register.  newsr The new value of the hardware status
    register."  The entry names no types for `newsr` and `oldsr`.  Is the
    argument any 16-bit integer (`integer`, `0..65535`)?  Is the result
    `integer` (signed) or unsigned, and what happens when it is widened?
    *Current: a 16-bit integer argument, or a constant in -32768..65535
    taken as its 16 bits; an `integer16` result (the `move.w`s of the
    expansion).*
17. **"Firstof", p. 4-87 (PDF p. 220, checked).** The table gives
    "integer or integer16 | -32767" and "integer32 | -2147483647", one
    above the least values of the types (3.2.1, p. 3-4: integer16 is
    -32768..32767; `ordinal_range` in `dpc-sema`).  Is `firstof(integer)`
    really -32767, or -32768?  *Current: `firstof` of integer, integer16
    and integer32 is refused; subranges, enumerations, char, boolean and
    arrays are supported.*  Related, not a question: neither table lists
    subranges; `dpc` gives a subrange's bound, after "returns the first
    [final] possible value of x" (pp. 4-87, 4-106).
18. **5.3.2, p. 5-7, `in` parameters.** "the routine is not allowed to
    alter its value. If the called routine does attempt to change its
    value (that is, use the parameter on the left side of an assignment
    statement), the compiler issues an 'Assignment to IN argument'
    error."  Is assigning to a *field or element* of an `in` parameter,
    a `with` on it, passing it to a `var`/`out`/`in out` parameter, or
    `addr` of it also that error (or allowed)?  *Current: whole
    assignment is the error; the other forms are refused as "not
    supported yet" (review of domain-pascal-pe9).*

Raised by the routine pointer work (bead domain-pascal-5aw.8; no bead
filed for the questions yet).  Conservative choices are implemented and
marked `TODO(ref-question)` in `crates/dpc-sema/src/check.rs`
(`signature_options`).

19. **"Addressing Procedure and Function Pointers", p. 4-146 (PDF
    p. 279, checked); Appendix B messages 181, 182.**  The example
    declares `pptr : ^Procedure(IN i, j : integer; OUT a : char; VAR r :
    real); EXTERN;`, and messages 181 ("INTERNAL option is illegal for
    PROCEDURE^ or FUNCTION^ types") and 182 ("The type declaration did
    not specify val_param, but the function heading did") imply routine
    options on routine pointer types, but no section gives their syntax
    or meaning.  (a) Is the long form `options(val_param)` allowed after
    the type as well as the short form?  (b) What does `extern` (or
    `forward`) mean on a type?  (c) Which other options (`d0_return`,
    `a0_return`, `c_param`, `abnormal`, ...) may be given?  *Current: both
    forms are parsed after the type's `;` in `type`, `var` and record
    field declarations (5.5.1's two forms; `../source`'s
    `tty/tty.ins.pas`, an agent-written reconstruction rather than an
    Apollo file, uses `options(val_param)`); `val_param` sets the
    convention; `internal` is message 181; every other option is refused
    as "not supported yet".*
    *Evidence note (Apollo's SR10.2 customer insert `~/src/domainos-archeology/sr10.2-install/install/ri.apollo.os.v.10.2/sys/ins/prsvr.ins.pas`;
    not an owner ruling):* Apollo's own declarations use the long form,
    after the closing parenthesis (or result type) and a `;`, with
    `extern` among the options, on `TYPE` declarations and on record
    fields:

    ```
    pr_$init_t =  ^FUNCTION (
                   IN interface_db : db_ptr;
                   OUT status : status_$t
                   ) : db_ptr ;options(d0_return,val_param,extern);

    pr_$monitor_t = ^PROCEDURE (
                       IN interface_db : db_ptr;
                       OUT status : status_$t
                       ) ;options(val_param,extern);
    ...
        prsvr_filter_$epv_t = RECORD
           process : ^FUNCTION(
                IN  obj_id : UNIV_PTR;
                IN  obj_db : db_ptr;
                OUT status: status_$t
                ) : int32_ptr;options(d0_return,val_param,extern);
    ```

    (also `pr_$render_t`, `pr_$end_job_t`, `pr_$close_t`, `inquire`).
    So (a) the long form is used, and (c) `d0_return` and `val_param`
    occur on pointer types.  Still open: (b) what `extern` means on a
    type (every routine pointer type in the file has it, so it may be
    required, or a no-op), whether `forward` or other options are
    allowed, and what `d0_return` changes for a call through the
    pointer (5.5.10: a pointer result in D0 instead of A0, presumably).
20. **5.6.4, p. 5-22.**  default_routine_options apply to "every
    subsequent routine in that module that does not specify a routine
    option".  Do they apply to a routine pointer type declared without
    options (its calls' conventions depend on it)?  *Current: such a
    type is refused while default_routine_options are in effect.*
21. **4.7, p. 4-46 (predicates).**  The reference names the predeclared
    conditional variables "`_BFMT__COFF` ... set to true whenever the
    compiler is generating COFF ... Beginning with $R10 [OCR for SR10], the
    Domain Pascal compiler generates COFF files" and "`_ISP__M68K` (for 68000
    code generation) / `_ISP__A88K` (for Series 10000 code generation)",
    and "Since `_ISP__M68K` and `_ISP__A88K` are predeclared, you cannot
    use the %enable directive or the -config option with them" (p. 4-48).
    Apollo's SR10.2 customer inserts (`~/src/domainos-archeology/sr10.2-install/install/ri.apollo.os.v.10.2/sys/ins/`) test other names:
    - `base.ins.pas` (01/02/89): "`%ifdef not isp_$m68k %then` / `%var
      isp_$m68k` / `%endif`" (and the same for `isp_$a88k`), then
      "`%IF ISP_$A88K or ISP_$M68k %THEN  { SR10 or later }`" around
      `%PUSH_ALIGNMENT; %WORD_ALIGNMENT;` and `time_$clock_t =
      [aligned(1)] PACKED RECORD ...`, "`%ELSEIF ISP_$M68K %THEN  { SR10
      or later 68k }`" for `ec2_$eventcount_t`, and "`%IFDEF obj_$coff
      %THEN                  { If sr10 or later }`" for `ios_$errout = 2`
      (never `%var`ed in the file).  Its history: "09/17/87 gms define
      isp_$?* if necessary" and "10/09/87 gms removed default enable of
      isp_$m68k".
    - `cal.ins.pas`, `ms.ins.pas`, `mutex.ins.pas`, `trait.ins.pas` and
      others: "`%if isp_$m68k or isp_$a88k %then`" (`trait.ins.pas`
      history: "09/17/87 gms s/m68k/isp_$m68k/.").
    - `gsr_types.ins.pas` (May 1989) defaults both spellings: "`%IFDEF
      ISP_$M68K %THEN %ELSE %VAR ISP_$M68K %ENDIF`" ... "`%IFDEF
      _ISP__M68K %THEN %ELSE %VAR _ISP__M68K %ENDIF`", then tests
      "`%IF _ISP__M68K OR _ISP__A88K %THEN`".

    Question: do the SR10 compilers (the kernel's included) also
    predeclare `isp_$m68k` / `isp_$a88k` (true for the target) and
    `obj_$coff`, besides the documented `_ISP__M68K` / `_BFMT__COFF`?
    If not, `base.ins.pas` would take its pre-SR10 branches
    (`time_$clock_t` without `[aligned(1)]` and word alignment, `ios_$errout
    = 3`) unless the user enables `isp_$m68k`.  *Current: `dpc`
    predeclares only `_BFMT__COFF`, `_ISP__M68K` (true) and `_ISP__A88K`
    (false) (`crates/dpc-syntax/src/preprocess.rs`); `isp_$m68k` is undeclared,
    so Apollo's `base.ins.pas` would declare it with `%var` (false) and
    take the pre-SR10 branches, and `%IFDEF obj_$coff` is false.  No
    `.ins.pas` of `../source` tests these names.*

22. **3.10.7, Table 3-3, p. 3-36 (PDF p. 91, read in the page image; the
    OCR scrambles the table), with 3.9.3, p. 3-16.**  Table 3-3, row
    "Set": "If fewer than 32 elements, then exactly one bit per element;
    if more then 32 elements, then same size as set.  Bit aligned."
    3.9.3: "Sets are stored as bit masks, with one bit representing one
    element of the set. ... The one exception to this is small sets within
    a packed records.  See Section 3.10.7 for a technique that packs small
    sets within a packed record."  Unpacked, a set is a bit string indexed
    by ordinal from 0, and its high bound decides its size (`set of
    80..170` is 11 words, p. 3-17; codegen-patterns 5).
    Question: how many bits does a packed record field of `set of lo..hi`
    with lo > 0 take, and where is element k: `hi - lo + 1` bits (one per
    element, k at position k - lo), or `hi + 1` bits (ordinals 0..hi, as
    unpacked)?  Is "fewer than 32 elements" counted the same way, and what
    does a set of exactly 32 elements take (the table covers "fewer than
    32" and "more then 32")?  *Current: a set whose base type starts at
    ordinal 0 takes `hi + 1` bits when that is under 32 (Apollo's
    `streams.ins.pas` `unused6 : set of 0..21; { nxt field needs word
    alignment }` needs 22 bits, and `streams.ins.c` declares the same
    bits as `unused6: 6` plus a `short`), else its unpacked size; exactly
    32 gives 32 bits either way.  A set whose base type does not start at
    ordinal 0 is refused (`crates/dpc-sema/src/types.rs`,
    `packed_field_bits`; test `packed_sets_not_from_zero_are_refused`).
    No such field has been found in the SR10.2 inserts.*

## Reference errata noticed (no ruling needed)

- **Table 4-11, p. 4-45** swaps the descriptions of `%push_alignment` and
  `%pop_alignment`.  The prose on p. 4-58 is followed: push saves, pop
  restores.
- **pp. 4-50/4-51**: the OCR of the `%warning` example is garbled.
  `%warning` takes a plain `'string'`, as Table 4-11 shows.
- **3.11.1.4, p. 3-41**: the example `[3..4 := 2, 7 := -1 1 := 3]` (checked
  in the PDF) lacks a comma after `-1`.  The parser requires commas between
  initialiser items, as every other example does.
- **3.11.1.6, p. 3-43**: the example declares `array_example ; ARRAY
  [subr] OF integer := ...` (checked in the PDF) with `;` where a var
  declaration needs `:`.  The parser requires `:`.
- **5.7.1, p. 5-26**: the main program of the section example ends with
  `END;` (checked in the PDF); a main program ends with `END.` (2.2,
  Figure 2-1, p. 2-8).  The parser requires `.`.
- **Table 5-1, p. 5-5** (PDF p. 360): the IN row says "UNIV: Cancels the
  effect of var_param and internal"; the no-mode row says "val_param".
  `var_param` is read as `val_param`.
- **3.11.4.1, p. 3-46**: "The compiler adds 2 or 3 bytes of padding to make
  the total size of the string an even number of longwords" contradicts the
  examples on the same page (`varying[8]` is 14 bytes).  The summary rule
  and the examples ("add either 5 or 6 bytes ... such that the total number
  of bytes is an even number") are followed.
- **3.10.5.7, p. 3-29**: "there are 4 bytes of gaps" in record FOO, but
  Figure 3-12 (PDF p. 85) shows two 1-byte gaps.  The figure's offsets are
  followed.
