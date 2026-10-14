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

## 4.7 Predeclared conditional variables isp_$m68k / isp_$a88k / obj_$coff (p. 4-46, 4-48; PDF pp. 179, 181)
Asked: 2026-10-13, by the lead agent (open question 21; review of domain-pascal-xdh.3)
Passage: "_BFMT__COFF ... set to true whenever the compiler is generating COFF"; "_ISP__M68K (for 68000 code generation) / _ISP__A88K (for Series 10000 code generation)" (p. 4-46).  Apollo's SR10.2 inserts test `isp_$m68k`, `isp_$a88k` and `obj_$coff` instead (base.ins.pas and others; see question 21's former text below).
Question: do the SR10 compilers also predeclare `isp_$m68k` / `isp_$a88k` and `obj_$coff`?
Owner's answer: "yes. isp_$m68k and isp_$a88k are target-specific and pre-declared. obj_$coff I'm not sure about, but let's use it."  Follow-up: "also list `obj_$bin`, which is the pre-domainos binary type."
Consequence: dpc predeclares `isp_$m68k` (true) and `isp_$a88k` (false) like `_ISP__M68K`/`_ISP__A88K`, `obj_$coff` (true, provisionally: the owner is not sure the compiler predeclared it) like `_BFMT__COFF`, and `obj_$bin` (false: the pre-Domain/OS object format, which dpc does not generate); Apollo's base.ins.pas then takes its SR10 branches.  Implementation: bead domain-pascal-mhg (crates/dpc-syntax/src/preprocess.rs).
Implemented (domain-pascal-mhg): the list is target-specific, so `dpc_target::Target::predeclared_cond_vars` supplies it (`dpc_m68k::M68k`: `_ISP__M68K`, `isp_$m68k`, `_BFMT__COFF`, `obj_$coff` true; `_ISP__A88K`, `isp_$a88k`, `obj_$bin` false) and the driver passes it to the preprocessor in `preprocess::Options::predeclared`; the preprocessor has no built-in list.  `%enable` and `-config` of any of them are errors (p. 4-48), and so is `%var` (already declared, p. 4-49).  `%ifdef` of any of them is true, since `%ifdef` tests "declared with %var", not true (p. 4-49): so `%IFDEF obj_$bin` is true while `%IF obj_$bin` is false.  With this, `dpc --check-only` over a copy of SR10.2 base.ins.pas preprocesses cleanly and takes the SR10 m68k branches; the remaining errors are semantic (the `[aligned(1)]` and `[volatile]` attributes, not supported yet).  Since domain-pascal-l9p/-t36 (alignment and `volatile`/`atomic`/`device` attributes) it checks clean (codegen-patterns 5.4).

## 4.7 Precedence of not / and / or in %if predicates (p. 4-46; PDF p. 179)
Asked: 2026-10-11, by the lead agent (open question 3; lexer/preprocessor work)
Passage: "A predicate can contain ... Optional Boolean keywords not, and, or or" (no precedence given).
Question: what precedence do `not`, `and` and `or` have in a predicate?
Owner's answer (2026-10-13): standard Pascal operator precedence: `not` (with the other unary operators) highest, then the multiplying operators including `and`, then the adding operators including `or`, then the relational operators.
Consequence: no change; dpc already evaluates `not` > `and` > `or` (Table 4-3, p. 4-4).  The TODO(ref-question) in `crates/dpc-syntax/src/preprocess.rs` (`Pred`) is replaced by a citation of this ruling.

## 2.1.4 C-style /* */ comments in the 2.1.2 example (p. 2-2; PDF p. 35)
Asked: 2026-10-11, by the lead agent (open question 4; lexer work)
Passage: the integer example writes "half_life := 5260; /* default (base 10) */", but 2.1.4 lists only `{ }`, `(* *)` and `" "`.
Question: is `/* */` a Domain Pascal comment form?
Owner's answer (2026-10-13): a copy-edit miss in the manual; `/* */` is not supported.  Revisit only if the wider Domain Pascal corpus turns out to use C-style comments (unlikely).
Consequence: no change; the lexer already rejects `/* */` per 2.1.4.  Errata list updated.

## 5.1 Attribute lists on parameter types (p. 5-2; meaning from 3.15.6.8, p. 3-76)
Asked: 2026-10-13, by the lead agent (open question 23; corpus evidence from bead domain-pascal-lmw)
Passage: 5.1 gives a parameter's type as a type name, but Apollo's `rws.ins.pas` declares "`OUT Status: [aligned(1)] Status_$T`" and `error.ins.pas` "`IN a1,...,a10: UNIV [aligned(0)] error_$integer32`" (both under `%if isp_$m68k or isp_$a88k`).
Question: did the compiler accept an attribute list on a parameter type, and with what meaning?
Owner's answer (2026-10-14): yes, support attribute lists on parameter types.
Meaning (from the reference, not a separate ruling): 3.15.6.8 "Passing Arguments by Reference", p. 3-76: "The compiler is unable to determine the alignment of objects passed by reference as arguments to routines. By default, the compiler assumes that such objects are naturally aligned. Therefore, if you know that an argument passed by reference is not naturally aligned, you should specify its alignment."  So `[attrs] T` in a parameter declares the same type as a named `= [attrs] T` (cf. `error_$status_t = [aligned(1)] status_$t`, error.ins.pas line 22): the attributes describe the actual argument's object, and the callee may assume only that alignment.
Consequence: implemented under bead domain-pascal-lmw.

## 7.2.2 Lists in the define statement (p. 7-5; PDF page 430)
Asked: 2026-10-14, by the corpus agent (open question 33; bead domain-pascal-hsb)
Passage: "The define statement takes the following syntax: define variable1 [:= initial_value1]; ... [variableN [:= initial_valueN];]" (one name per `;`), while Apollo's examples write `DEFINE sasc_$get, sasc_$locate, ...;` (streams/sasc io.pas, imex.pas, misc.pas, open.pas, seek.pas; hardcopy driver_ex.pas, filter_ex.pas).
Question: is a comma-separated list accepted, and may listed names carry `:= initial_value`?
Owner's answer (2026-10-14; the owner notes not every case was checked): `DEFINE var1, var2, ..., varN;` is allowed; it defines the names and leaves them uninitialised.  A name with an initialiser needs its own `;`: `DEFINE var1 := init1; var2 := init2; ... varN := initN;`.  So `:=` is not allowed inside a comma list.
Consequence: implemented under bead domain-pascal-hsb.

## 5.1 Empty argument lists (p. 5-3; PDF page 358)
Asked: 2026-10-14, by the corpus agent (open question 34; bead domain-pascal-9iq)
Passage: "To call a routine, simply specify its name. If the procedure has a parameter list, you must also specify arguments."  Apollo writes `pgm_$exit( );` (usr/X11/examples/pascal/xhw1.pas) and `ext_db := alloc_db();` (hardcopy driver_ex.pas).
Question: is `name()` a valid call of a routine with no parameters?
Owner's answer (2026-10-14): both `simple;` and `simple();` call the procedure `simple` with no arguments and are valid syntax; leaving out the `()` is just an (arguable) ergonomic convenience.
Consequence: implemented under bead domain-pascal-9iq; the lead applies the same reading to function calls such as `alloc_db()` (Apollo's own example), pending the owner's confirmation.

## 3.13.4 addr(...) as an initial value (p. 3-51)
Asked: 2026-10-14, by the corpus agent (open question 36; bead domain-pascal-bjz)
Passage: "Domain Pascal permits you to initialize the values of pointer variables within its variable declaration in most cases"; the only example is `NIL`.  Apollo initialises routine-pointer fields with `init := addr(my_driver_$init),` (hardcopy driver_init_ex.pas, filter_init_ex.pas, recno.pas, scramble.pas).
Question: is `addr(routine)` (or `addr(variable)`) allowed as an initial value?
Owner's answer (2026-10-14): permit `addr(...)` in initialisers; Apollo's code clearly uses it.
Consequence: implemented under bead domain-pascal-bjz.  The initial value is static data, so the lead reads this as `addr` of objects with a static address (routines; module-level, static, define and extern variables), emitted as an address with a relocation; `addr` of a routine's ordinary local is still refused.  As in any `addr`, a nested or `internal` routine's address stays refused (3.13.3, p. 3-50: "you cannot obtain the addresses of nested or explicitly declared internal procedures and functions").  Fields and elements of static variables at constant offsets are accepted; an element at a variable index is refused.

## "Type Transfer Functions" (p. 4-197): integer constant to a pointer type
Asked: 2026-10-14, by the lead agent (bead domain-pascal-gqp; found by the corpus test in domain_examples/streams/sasc/misc.pas)
Passage: "With one exception, the size of the argument must be the same as the size of the destination type. ... The one exception is that integer subranges are always compatible regardless of their sizes."  Apollo writes `ecp := ec2_$ptr_t( ec2_$always_ready_ec )`, where ec2.ins.pas line 24 declares `ec2_$always_ready_ec = 1;  { as ec2_$ptr to ec2_$wait is an always ready EC2 }`: a 2-byte integer constant transferred to a 4-byte pointer type.
Question: is a type transfer from an integer constant to a pointer type of a different size allowed?
Owner's answer (2026-10-14): yes, allow it; the constant is a sentinel value expanded to a pointer so that type checking works.
Consequence: to be implemented under bead domain-pascal-gqp.  The lead reads the ruling as covering integer *constants* (the value is known, so the widening is exact); a run-time integer16 transferred to a pointer stays refused until something needs it.  Related: open question 37 (`char(16#9E)`), the narrowing counterpart.  Side note: 2.2.2.2 (p. 2-12) lists "A type transfer function" among the terms a const value may contain but names `nil` as the only pointer value, which may be why Apollo declared the sentinel as an integer and transferred it at each use.
Implemented (domain-pascal-gqp): `dpc-sema` (`Checker::transfer`) accepts a type transfer of an integer constant >= 0 to a pointer type of another size; the result is a pointer-typed constant with that value (an address), so it is stored as `move.l #n`, passed by address as a code-section cell and by value as `pea (n).w`, like a 4-byte integer constant (golden `expr/transfer_const_ptr`; no image site identified).  A negative constant is refused: p. 4-197 says nothing once the sizes differ.  A run-time integer16 stays refused (the p. 4-197 size rule).  In a const declaration (`c = ptr_t(1)`) the transfer is refused as before, now with its own message: 2.2.2.2 (p. 2-12) names `nil` as the only pointer value.  A static variable's initial value (`p: ptr_t := ptr_t(1)`) is accepted and emitted as the 4-byte value.  An integer32 constant transferred to a pointer type (equal sizes, so p. 4-197 already allowed it, negative or not) is a pointer constant in the same way, so `ptr_t(16#20000)` is also accepted as an initial value.

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
3. **ANSWERED 2026-10-13 (see Clarifications above).  4.7, p. 4-46, predicates.** "Optional Boolean keywords not, and, or
   or": no precedence is given.  *Current: Table 4-3 order, not > and > or.*
4. **ANSWERED 2026-10-13 (see Clarifications above).  2.1.2, p. 2-2 (PDF p. 35).** The integer example uses
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
21. **ANSWERED 2026-10-13 (see Clarifications above).  4.7, p. 4-46 (predicates).**  The reference names the predeclared
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

23. **Answered 2026-10-14; see Clarifications, "5.1 Attribute lists on parameter types".**  **5.1, p. 5-2 (attributes on parameter types).**  A parameter's type
    must be a type name, but Apollo's `rws.ins.pas` declares `OUT Status:
    [aligned(1)] Status_$T`.  Did the compiler accept an attribute list on
    a parameter type, and with what meaning?  *Current: refused.*
    *Evidence note (not an owner ruling; bead domain-pascal-lmw):*
    Apollo's `error.ins.pas` has the form too, after `UNIV`: "`IN
    a1,a2,a3,a4,a5,a6,a7,a8,a9,a10:  UNIV [aligned(0)] error_$integer32`",
    so every program that includes error.ins.pas is refused today.
24. **3.15.7, p. 3-77 (device inheritance).**  "if you designate any one
    field within a record as having the device attribute, the entire
    record itself inherits the device attribute."  With a `device(read)`
    field, do the record's other fields become device and read-only?
    *Current: the whole record is `device(read)`, so writing a plain
    sibling field is an error.*
25. **3.15.6.3 / 3.15.6.6, pp. 3-70, 3-74 (byte-aligned words on m68k).**
    For `b, c: [ALIGNED(0)] integer16` at odd offsets (Figure 3-24), what
    did the 68000-family compiler emit?  Split references are described
    as "Series 10000 Only".  *Current: an odd-address word/long access is
    refused (the 68010 traps).*  The same refusal covers a parameter of a
    byte-aligned type passed by address (`IN a1: UNIV [aligned(0)]
    error_$integer32`, error.ins.pas): a word or longword reference
    through it, and the entry copy of a value parameter of such a type
    (bead domain-pascal-lmw).  A record comparison (`cmpm`) and a packed
    field's word or longword unit are refused at such addresses too.
26. **3.15.3, p. 3-57 (device record assignment).**  Is assigning or
    pushing a whole 4-byte `[device]` record as one 32-bit reference
    allowed, or does "The device attribute prevents this optimization"
    forbid it?  *Current: a 4-byte device record moves as one `move.l`;
    larger ones are refused.*
27. **Table 3-6 p. 3-55 / 3.15.2 p. 3-57 (atomic "when possible").**
    "perform updates with single instruction when possible"; the value is
    "not changed by other processes while it completes" the assignment.
    When no single instruction exists (`x := x * 2`), did the compiler
    bracket the update (disable/enable) or emit a plain sequence?
    *Current: refused.*
28. **3.10.5.2 (natural alignment of arrays, sets, varying strings).**
    Natural alignment is given for simple types and records only.  What
    applies to arrays, sets and varying strings under
    `%natural_alignment`?  *Current: such fields without an attribute are
    refused under the natural rules.*

29. **Table 4-1, p. 4-2 (= / <> on records).**  The table allows `=` and
    `<>` on "All types".  For a record with variant arms of different
    sizes, the bytes past the active arm are as undefined as padding.  Is
    such a compare a whole-record byte compare, an error, or undefined?
    And for records with padding between fields?  *Current: records with
    padding are refused; variant records whose arms together cover every
    byte are compared bytewise (review of domain-pascal-aaj).*

30. **"Goto", p. 4-95 vs. Appendix D, p. D-4 (jumps into structured
    statements).**  p. 4-95: "It is illegal to use goto to jump inside a
    structured statement (for example, a for, while, case, with, or
    repeat) from outside that statement" (with the `bad_gotos` example,
    p. 4-96, "ILLEGAL: cannot jump to a label inside the for loop").
    p. D-4 (ISO 6.8.1): "Domain Pascal permits jumps between branches of
    a case statement and from one structured statement into the middle
    of another."  Is a `goto` from one structured statement into a loop,
    `with` or `case` arm of another legal?  And is jumping into a
    compound statement (`begin ... end`, not in p. 4-95's list) from
    outside it legal?  *Current (bead domain-pascal-o4q): a jump into a
    `for`, `while`, `repeat`, `with` or `case` from outside it is refused
    ("not supported yet", `crates/dpc-ir/src/lower.rs`,
    `routine_labels`); jumps between the arms of one `case` (p. D-4) and
    into `if` parts (p. 6-23, `-niso`) are accepted; a compound statement
    is transparent (accepted).*

31. **4.3.1 p. 4-5 vs "Type Transfer Functions" p. 4-198 vs the image
    (word arithmetic in comparisons).**  p. 4-198: for `x + y > 5` on
    integer16 operands "the compiler expands operands x and y to 32-bit
    integers and performs 32-bit addition before making the comparison";
    `INTEGER16(x + y)` asks for the 16-bit form.  But the image compares
    word sums at word width at 41 sites against 7 that widen first (bead
    domain-pascal-9js; e.g. 0xE0F9D0..D8 widens).  The source types at the
    41 sites are not known.  Should dpc follow the reference (widen) or
    the image's majority (16-bit, wrapping on overflow)?  *Current: 16-bit
    compare; tests/exec/wide_compare.pas marks the reference's behaviour as
    an expected failure (domain-pascal-f03).*

32. *Settled by the reference (no owner ruling needed; bead
    domain-pascal-5j9).*  A non-negative subrange stored in 4 bytes
    (`0..2147483647`) is unsigned.  3.4, p. 3-9: "Through the use of large
    unsigned subranges (from 0..2^30 up to 0..2^31-1), you can achieve
    much of the effect of having true unsigned types"; 4.3.3, pp. 4-6..4-7
    (PDF pp. 139-140, checked: the OCR text drops the first four items of
    the list, which are `>`, `>=`, `<`, `<=`, before `min`, `max`, `div`,
    `mod`): these operations are signed "except" when "Both operands are
    unsigned run-time values" or "One operand is an unsigned run-time value
    and the other is a constant in the positive subrange 0..2147483647";
    its example's `u_type = 0..2147483647` gives "an unsigned divide".
    "Max", p. 4-111, and "Min", p. 4-113: "The unsigned scalar data types
    are non-negative subranges of integers, Boolean, character, and
    enumerated."  *Implemented for 4-byte subranges (codegen-patterns 5,
    "Signedness", with image sites).  Not changed: 2-byte `integer16`
    subranges such as `0..100`, and booleans, which the same sentence
    calls unsigned: the image cannot tell which words had such types, so
    the code for them is not settled (bead domain-pascal-4x2).*

Raised by the front-end corpus test (bead domain-pascal-qia,
`tools/corpus.py`: `dpc --check-only` over Apollo's SR10.2 inserts and
example programs, `~/src/domainos-archeology/sr10.2-install/install/ri.apollo.os.v.10.2/`).
Each is refused today; the corpus baseline (`tests/corpus/baseline.txt`)
records the files it stops.

33. **Answered 2026-10-14; see Clarifications, "7.2.2 Lists in the define statement".**  **7.2.2, p. 7-5 (PDF p. 430, checked; the OCR scrambles the
    syntax).**  "The define statement takes the following syntax: define
    variable1 [:= initial_value1]; ... [variableN [:= initial_valueN];]":
    one name per `;`.  Apollo's examples write lists:
    `domain_examples/streams/sasc/io.pas` "`DEFINE` / `sasc_$get,` /
    `sasc_$locate,` ...", also `imex.pas`, `misc.pas`, `open.pas`,
    `seek.pas`, `hardcopy/driver/driver_ex.pas` ("`DEFINE` /
    `my_driver_$init,` / `my_driver_$render,` ..."), `filter_ex.pas`.  Is
    a comma-separated list accepted in a define statement, meaning one
    item per name?  May listed names carry `:= initial_value`?  *Current:
    a parse error (bead domain-pascal-hsb).*
34. **Answered 2026-10-14; see Clarifications, "5.1 Empty argument lists".**  **5.1, p. 5-3 (PDF p. 358, checked).**  "To call a routine, simply
    specify its name. If the procedure has a parameter list, you must also
    specify arguments."  Apollo's examples call parameterless routines
    with empty parentheses: `usr/X11/examples/pascal/xhw1.pas`
    "`pgm_$exit( );`", `domain_examples/hardcopy/driver/driver_ex.pas`
    "`ext_db := alloc_db();`".  Is `name()` accepted as a call (statement
    and expression) of a routine with no parameters?  *Current: a parse
    error (bead domain-pascal-9iq).*
35. **"Addressing Procedure and Function Pointers", p. 4-146 (PDF p. 279,
    checked).**  "the declaration for proc2 must exactly match the
    template for the procedure type of pptr. If not, the compiler reports
    an error."  Apollo's type-manager examples assign routines whose
    handle parameter is a typed pointer to EPV fields whose template has
    `univ_ptr`: `domain_examples/streams/sasc/init.pas` "`open :=
    addr(sasc_$open),`", where `sasc.ins.pas` declares "`PROCEDURE
    sasc_$open( IN xoid: xoid_$t; IN opts: ios_$open_options_t; OUT h:
    sasc_handle_ptr_t; OUT st: status_$t); EXTERN;`" and `io_traits.ins.pas`
    the field "`open: ^PROCEDURE( IN xoid: xoid_$t; IN opts:
    ios_$open_options_t; OUT h: univ_ptr; OUT S: status_$t );`" (also
    `scramble.pas`, `recno.pas`).  Does a parameter of a pointer type
    match a `univ_ptr` parameter of the template (3.13.2, p. 3-50: univ_ptr
    as a "Formal or actual parameter for any pointer type"), in either
    direction, and for every mode?  *Current: refused, "does not match the
    routine pointer type" (bead domain-pascal-ble).*
36. **Answered 2026-10-14; see Clarifications, "3.13.4 addr(...) as an initial value".**  **3.13.4, p. 3-51.**  "Domain Pascal permits you to initialize the
    values of pointer variables within its variable declaration in most
    cases"; the only example is `NIL`.  Apollo's examples initialise
    routine-pointer fields of module-level records with `addr`:
    `domain_examples/hardcopy/driver/driver_init_ex.pas` "`init :=
    addr(my_driver_$init),`" (also `filter_init_ex.pas`, `recno.pas`,
    `scramble.pas`).  Is `addr(routine)` (or `addr(variable)`) allowed as
    an initial value?  *Current: refused, "initial value must be a
    constant" (bead domain-pascal-bjz).*
37. **"Type Transfer Functions", p. 4-197.**  "With one exception, the
    size of the argument must be the same as the size of the destination
    type. ... The one exception is that integer subranges are always
    compatible regardless of their sizes."  Apollo's `sys/ins/kbd.ins.pas`
    has "`KBD_$NUMERIC_KEYPAD = char(16#9E);    {KEYPAD introducer}`" and
    four more such constants (`sys/source/emt/emt.pas` too).  An integer
    constant is 2 bytes and `char` 1.  Is `char(k)` of an integer
    constant legal (with the value `chr(k)`), and for which `k`?
    *Current: refused, "type transfer needs equal sizes" (bead
    domain-pascal-qvy).*
38. **Messages 83 and 84, p. 9-16 (PDF p. 497); 5.1, p. 5-3.**  "Argument
    n to routine is not compatible with the declared argument type." /
    "Argument n to routine is not within the declared argument subrange."
    / "The data type of each argument must match the data type of its
    corresponding parameter."  Apollo's code passes subranges and their
    base types to each other's `var`/`out` parameters:
    `sys/source/emt/emt.pas` an `integer32` variable to a `linteger`
    (`base.ins.pas` "`linteger= 0..2147483647;`") parameter and a
    `pinteger` to an `integer16` one; `usr/X11/examples/dmwin/dmwin.pas`
    `ios_$id_t` (`0..ios_$max`, `ios_$max = 127`) variables to the
    `integer16`-typed `OUT` parameter of `pad_$create_window`.  It also
    assigns a constant outside a subrange: dmwin.pas "`connv[ios_$stderr]
    := ios_$no_stream;`" with `ios_$no_stream = 16#7FFF` and `connv` an
    array of `ios_$id_t`.  For `var`/`out`/`in out` parameters, are a
    subrange and its base type (or two subranges of one base type)
    compatible?  Is an out-of-range constant assigned to a subrange an
    error, a warning, or accepted?  *Current: both refused (bead
    domain-pascal-4tm).*
39. **"Rshft", p. 4-171; 3.2.3, p. 3-5.**  "if num is negative and is a
    32-bit integer, the result of an rshft is always positive", and
    "Suppose num is 16 bits and equals -9. You perform an rshft with sh
    equaling 3 and put the result back in a 16-bit integer ... = -2".
    3.2.3 says integer constants are represented as 32-bit integers.  In a
    constant expression such as `c = rshft(-9, 3)`, is the value the
    32-bit result 16#1FFFFFFE (536870910), or does it depend on where the
    constant is used?  *Current: rshft of a negative constant is not
    folded, so it is refused in a const declaration (bead
    domain-pascal-95c).*

## Reference errata noticed (no ruling needed)

- **2.1.2, p. 2-2**: the integer example uses C-style `/* ... */`
  comments; owner ruling (question 4): a copy-edit miss, not a comment form.

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
