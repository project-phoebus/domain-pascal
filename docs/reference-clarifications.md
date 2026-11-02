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
Implemented (domain-pascal-gqp): `dpc-sema` (`Checker::transfer`) accepts a type transfer of an integer constant >= 0 to a pointer type of another size; the result is a pointer-typed constant with that value (an address), so it is stored as `move.l #n`, passed by address as a code-section cell and by value as `pea (n).w`, like a 4-byte integer constant (golden `expr/transfer_const_ptr`; no image site identified).  A negative constant is refused: p. 4-197 says nothing once the sizes differ (superseded 2026-10-21 by the owner's "Allow, 32-bit", below: `p(-1)` is 16#FFFFFFFF).  A run-time integer16 stays refused (the p. 4-197 size rule).  In a const declaration (`c = ptr_t(1)`) the transfer is refused as before, now with its own message: 2.2.2.2 (p. 2-12) names `nil` as the only pointer value.  A static variable's initial value (`p: ptr_t := ptr_t(1)`) is accepted and emitted as the 4-byte value.  An integer32 constant transferred to a pointer type (equal sizes, so p. 4-197 already allowed it, negative or not) is a pointer constant in the same way, so `ptr_t(16#20000)` is also accepted as an initial value.

## "Rshft", p. 4-171: rshft of a negative constant (3.2.3, p. 3-5)
Asked: 2026-10-14, by the lead agent (open question 39; bead domain-pascal-95c)
Passage: "if num is negative and is a 32-bit integer, the result of an rshft is always positive", and the 16-bit example "Suppose num is 16 bits and equals -9 ... = -2"; 3.2.3 says integer constants are represented as 32-bit integers.
Question: in a constant expression such as `c = rshft(-9, 3)`, is the value the 32-bit result 16#1FFFFFFE, or does it depend on where the constant is used?
Owner's answer (2026-10-14): refuse it until something needs it.  (The lead's reading, not adopted: a 32-bit logical shift, as Delphi and Free Pascal also do for wide constant folding.)
Consequence: none; dpc already leaves rshft of a negative constant unfolded, so it is refused in a const declaration and by the existing run-time check elsewhere.  No use in the corpus or in ../source's Pascal at the time of the ruling.

## Narrowing integer32 to integer16 (5.1 p. 5-3; Appendix D pp. D-3, D-4)
Asked: 2026-10-15, by the 8td work (open question 41)
Passage: see open question 41 (5.1: "the second argument must be an integer16 value"; D-3: "you may pass an integer argument that is outside the corresponding parameter's declared subrange"; D-4: no overflow detection).
Question: does integer16 count as a subrange of integer32 for assignment and value arguments, and what value does the target hold?
Owner's answer (2026-10-15): yes, integer16 is a subrange of integer32 for narrowing purposes.
Consequence: no change; dpc already accepts integer32 where integer16 is expected (assignment, value and `in` arguments, function results) and keeps the low bytes with no check (bead domain-pascal-8td; codegen-patterns 8, "Narrowing and widening").

## Subranges and their base type as var arguments; out-of-range constants (Appendix D pp. D-3; messages 83/84 p. 9-16)
Asked: 2026-10-14, by the corpus test (open question 38; bead domain-pascal-4tm); asked again 2026-10-15 when ../source's net_io and route needed it (../source "dpc gap 81", 14 routines in srcsurvey)
Passage: see open question 38.  Appendix D, ISO 6.6.3.5/6.6.3.6, p. D-3: "Domain Pascal treats integer and subrange of integer as identical."  ISO 6.4.6, p. D-3: "You may make an assignment from an integer expression that includes a value outside one of the variables' declared subranges."
Question: for `var`/`out`/`in out` parameters, are a subrange and its base type (or two subranges of one base type) compatible?  Is an out-of-range constant assigned to a subrange accepted?
Owner's answer (2026-10-15): "apparently there's a compiler flag about this.  but we need to support the quiet subrange being accepted."  Out-of-range constant: accept, store the low bytes.
Consequence: by default an integer subrange and an integer type (or two integer subranges) are accepted for each other as `var`/`out`/`in out` arguments, with no message; an out-of-range integer constant assigned to a subrange keeps its low bytes (as question 41's narrowing).  The flag the owner mentions is not identified: the reference's -iso (6.4.18, p. 6-23) and -std (6.4.27) do not name subranges, and dpc has neither, so no strict mode is implemented.  The lead keeps one limit not settled by the ruling: the two types must have the same size, since a by-address argument of 2 bytes for a 4-byte formal would make the callee read or write past it; a size mismatch stays refused.  Implemented under bead domain-pascal-4tm.  Two limits are the lead's reading, from the integer-only wording of Appendix D, p. D-3 ("integer and subrange of integer"; "an integer expression"), and not part of the owner's answer; they are to be confirmed with the owner: (a) both halves are limited to *integer* subranges, so a char, boolean or enumeration subrange and its base type are still refused for each other as `var`/`out`/`in out` arguments, and a char, boolean or enumeration subrange still refuses a constant outside it; (b) `integer16` itself still refuses a constant outside -32768..32767 (`i := 40000`), as it is a type, not a declared subrange (question 41's ruling is about narrowing values, not constants).  A constant for a packed-record field or packed-array element of an integer subrange keeps the field's low bits; a comparison with a constant outside the other operand's type stays refused ("not supported yet").

## Module heading with an empty code section (7.1.1, p. 7-2; 2.2.1, p. 2-10)
Asked: 2026-10-15, by the lead (srcsurvey: PCHIST blocked; ../source pascal-translation-notes gap 35)
Passage: 7.1.1 gives "module name [, code_section_name [, data_section_name]];" and says the module heading "is similar to a program heading"; 2.2.1 shows the program heading's `name, , data_section' form (question 8 accepted it for programs).
Question: may a module heading leave the code section name empty, `module pchist, , wired_data;`?
Owner's answer (2026-10-15): yes; the compiler must parse the form without the code section name, which targets .text for the code section.
Consequence: implemented under the bead filed for it (module heading parse; code in the default section, data in the named one).

## A size attribute on a set type: `[byte] set of 0..7` (3.15.5, p. 3-61; 3.9.3)
Asked: 2026-10-16, by the lead (../source bca1d04b, its "gap 95": acl's rights sets and app's flags byte as `[byte] set of 0..7` fields; dpc refused them, so 17 modules stopped checking)
Passage: 3.15.5, p. 3-61: "A size attribute must specify at least the number of bits required for the type to which it is applied" and "A size attribute can specify a number of bits greater than the minimum for integer, boolean, and set data types. If you use a size attribute to specify a size larger than the minimum for a data type, then the compiler uses all of the bits in the larger object to represent the value of an object of that type."  Table 3-7 lists no minimum for sets; 3.9.3 sizes an unpacked set in whole words (`set of 0..7` is 16 bits).
Question: is `[byte] set of 0..7` legal, a 1-byte set, with element k at bit k of that byte (the big-endian bit string cut to one byte)?
Owner's answer (2026-10-16): legal, 1 byte.
Consequence: a set's minimum size for a size attribute is the number of bits its range needs; `[byte]` on a set of at most 8 elements gives a 1-byte set, element k at bit k; as any one-byte object it may sit at an odd offset of an unpacked record (3.10.5.5).  Code for such sets follows the image (bead filed with this entry).

## Lshft of a 16-bit operand; the sign of a shift of an unsigned word ("Lshft", p. 4-109; "Rshft", p. 4-171; 4.3.3)
Asked: 2026-10-16, by the lead (open question 15, extended for bead domain-pascal-4rfc)
Passage: see open question 15.  "Rshft" (p. 4-171) and "Arshft" (p. 4-29) expand an operand smaller than 32 bits to 32 bits, shift, and convert back; "Lshft" (p. 4-109) does not say, and its example (`i : INTEGER16`, `LSHFT(i, 7)` = 10#-6144) shows a 16-bit result.  4.3.3: `>`, `>=`, `<`, `<=`, max, div and mod are signed unless both operands are unsigned run-time values.
Question: when the result is used as an integer32, is lshft of a 16-bit operand a 16-bit shift then widened, or a 32-bit shift?  Is a shift of an unsigned 0..65535 word itself an unsigned run-time value?
Owner's answer (2026-10-16): 32-bit, like rshft: a 16-bit operand is expanded to 32 bits (by its sign; a 0..65535 value zero-extended), shifted, and the result is integer32, narrowed back only when stored into a 16-bit target (so the p. 4-109 example is the value narrowed back into `i`).  A shift of an unsigned run-time value is unsigned (compares unsigned, div/mod in the unsigned forms).
Consequence: implemented under bead domain-pascal-4rfc.
Implemented: `dpc-sema` types `lshft`/`rshft` as integer32, or as the `0..2147483647` subrange (the type of any unsigned longword) when the operand is an unsigned run-time value; `dpc-ir` widens a 16-bit operand by its sign and shifts in 32 bits (`clr.l; move.w` or `ext.l`, then `lsl.l`/`lsr.l`: AS_$GET_ADDR 0xE58418, UID_$GEN 0xE1A04A, PEB_'s module-local 0xE31DEA), shifting at the word's width only where the result is stored into a 16-bit target and the low word is the same (`lshft`; `rshft` of an unsigned word: module-local 0xE07386); compares, `div` and `mod` of an unsigned shift take the unsigned forms.  The p. 4-109 example: `lshft(i, 7)` of 2000 is 256000 as an integer32 and -6144 stored into `i` (exec test `shift_widen`).  A `[byte] 0..255`'s `lshft` used as a longword (e.g. compared, `lshft(b, 8) > 5`, formerly a 16-bit compare) is refused: the image widens such a word shift through a second register (RING_$GET_ID 0xE2FA66), not implemented; exec test `byte_ints` no longer compares it.  `arshft` is unchanged (p. 4-29's expansion gives the value of the 16-bit `asr` widened by its sign).  Codegen-patterns 8, "Shifts of a 16-bit operand".

## A variable's own address in its initializer (2.2.2.2; 7.2; question 36)
Asked: 2026-10-16, by the lead (../source network/network.pas: `network_$rqst_done_ec: ec_$eventcount_t := [value := 0, waiter_list_head := addr(network_$rqst_done_ec), waiter_list_tail := addr(network_$rqst_done_ec)]`; the image's data at +0x258 holds the eventcount's own address)
Passage: question 36's ruling permits `addr(...)` in initializers; nothing says whether the variable being declared is in scope in its own initializer.
Question: is a variable in scope inside its own initializer, so that `addr(itself)` gives its own address?
Owner's answer (2026-10-16): yes, in scope.  (Variables declared later in the var part are still not visible before their declaration.)
Consequence: implemented (bead domain-pascal-i5mf): dpc-sema declares each name of a var declaration (its type and storage settled) before checking its initial value, for unit-level, `static`, `define`d and overlay-section variables and a routine's statics; `addr(itself)` is a relocated long to the variable's own place, as other `addr(...)` initial values (the data area plus an offset, or the overlay section's symbol plus an offset).  Tests: `own_address_initial_value` (crates/dpc-sema/src/tests/checks.rs), golden `data/self_addr`. ../source network/network.pas now compiles; its NETWORK data block (D 0xE248FC, 0x364 bytes) matches the image byte for byte, +0x258 `0000 0000 00e2 4b54 00e2 4b54` and +0x268 likewise.

## A -config name no %var declares (6.4.8, p. 6-12; PDF p. 393)
Asked: 2026-10-20, by the zf19 agent (bead domain-pascal-zf19; docs/compiler-options.md)
Passage: "-config var1 [... varN] where var must be a conditional variable declared with %var."
Question: the reference does not say what happens when a `-config` name is never declared with `%var`.  Error, warning, or nothing?
Owner's answer (2026-10-20): warn.  The unit compiles, with a warning naming the name.
Consequence: after preprocessing, each `-config` name that no compiled `%var` declared gets one warning without a position, "-config variable `x` is not declared with %var" (crates/dpc-syntax/src/preprocess.rs; tests `config_does_not_declare`, options.rs `accepted_noted_and_refused`).  The name stays undeclared: `%ifdef` of it is false and `%if` of it is an error (p. 4-50).  `-nwarn` hides the warning like any other.

## Commas between %var and %enable names (Figure 6-2, p. 6-13; 4.7, p. 4-50; PDF pp. 394, 183)
Asked: 2026-10-20, by the zf19 agent
Passage: Figure 6-2 writes "%VAR first, second, third"; 4.7 writes "%VAR code.old code.new", "%VAR color mono" and "%ENABLE code.sr9 code.sr7", without commas.
Question: are commas allowed between the names of `%var` (and `%enable`)?
Owner's answer (2026-10-20): yes.  Names may be separated by blanks, commas, or both.
Consequence: the preprocessor's name list (`names`) takes an optional `,` between two names; a `,` must stand between two names (`%var a,` and `%var a,, b` are errors).  The list still ends at the end of the directive's line (open question 2).  Tests: `var_and_enable_lists_with_commas`; Figure 6-2 as printed in crates/dpc/tests/options.rs.

## %warning('string') and %error('string') (4.7 "%config", p. 4-51; PDF p. 184)
Asked: 2026-10-20, by the zf19 agent
Passage: the %config example writes "%warning('You did not set color or mono to true.');", while the directive is given as "%warning 'string'" (and "%error 'string'") on the same page.
Question: is the parenthesised form valid?
Owner's answer (2026-10-20): accept both forms, `%warning 'x'` and `%warning('x')`, and the same for `%error`.
Consequence: `%error` and `%warning` take an optional `(` before the string and then need its `)` (preprocess.rs; tests `error_and_warning_in_parentheses`, `reference_color_mono_examples`).  The erratum entry for pp. 4-50/4-51 below is superseded.

## The default of -msgs (6.4.21, p. 6-25; Table 6-2, p. 6-8; PDF pp. 406, 389)
Asked: 2026-10-20, by the zf19 agent
Passage: 6.4.21: "The -msgs option is the default."  Table 6-2 stars "-nmsgs" as the default.
Question: which is the default?
Owner's answer (2026-10-20): `-nmsgs` stays dpc's default: no summary unless `-msgs` is given.  Evidence noted by the reviewer, which the ruling does not follow: the reference's transcripts show the summary line without `-msgs` on the command line ("No errors, no warnings, Pascal Rev n.nn" after "$ pas config_example", p. 6-14; "1 error, no warnings, Pascal Rev n.nn", p. 4-51; "No errors, 1 warning, Pascal Rev n.nn", p. 4-52), which fits the text's `-msgs` default.
Consequence: none beyond the implementation: `-msgs` prints the report on stderr (crates/dpc/src/main.rs, `Counts::report`); by default nothing is printed.

## 4.7 A `;` after a conditional directive (p. 4-43; pp. 4-47..4-49; PDF pp. 176, 180..182)
Asked: 2026-10-21, by the lead agent (bead domain-pascal-bj6d; found in the SAU5 SMD source)
Passage: p. 4-43: "You must place a semicolon after some directives if you use them as statements. Those directives are: %begin_inline; ... %slibrary 'pathname';" (no conditional directive is listed); the examples write "%ENDIF ;" (pp. 4-47, 4-48, 4-49) and "%IF _ISP__M68K %THEN PROCEDURE do_68K ... %ELSEIF _ISP__A88K %THEN PROCEDURE do_10000 %ENDIF;" (p. 4-48).  SMD's source has `if x = 0 then x := 1 %if b %then else x := 2 %endif ; x := 4;`, where the `;` must separate the statements.
Question: does a `;` written after `%endif` (or `%then`, `%else`) belong to the directive or to the program text?
Owner's answer (2026-10-21): "Context".  In a statement part it is the statement separator; elsewhere, in declarations where a stray `;` is an error, it belongs to the directive.
Consequence: implemented under bead domain-pascal-bj6d.  The preprocessor passes a `;` right after `%then`, `%else` or `%endif` (in a compiled region) on as `TokenKind::CondSemi` instead of swallowing it as before.  The lead reads the ruling as: the `;` counts as a `;` exactly where the parser would accept one, and is dropped everywhere else.  So the parser's cursor does not see it; `Parser::eat` and `Parser::expect` of `;` take it when the next token is not a real `;` (`Parser::eat_cond_semi`).  It is then the separator of statements (the SMD case), declarations, parameters, record fields and variant arms (`(x: integer %if a %then ; y: integer %endif)`, which main also accepted).  Where a statement begins (after `then`, `else`, `do`, a case label's `:`, `begin`), the empty statement is legal, so the `;` is not stray: it ends an empty statement there and separates it (`if c then %if a %then x := 1 %endif; y := 2` without `a` is `if c then ; y := 2`, not `if c then y := 2`).  It is dropped before `else` (`if c then y := 1 %if a %then + 1 %endif; else ...`), right after another `;` (`1: y := 1; %if a %then 2: y := 2; %endif; 3: ...` in a case-arm list, `x: integer; %endif;` after a declaration), and wherever no `;` is expected (crates/dpc-syntax/src/{lexer,preprocess,parser}.rs).  The `;` after the other conditional-family directives (`%var`, `%enable`, `%error`, `%warning`, `%exit`) still belongs to the directive.  Tests: parser `semi_after_conditional_directive_in_statements`, `..._in_declarations` (the p. 4-48 example), `..._where_a_semi_is_an_error`, `..._as_a_declaration_separator` and `..._where_a_statement_begins`; preprocess `semi_after_conditional_is_kept`.

## Type transfer of a negative constant to a pointer type ("Type Transfer Functions", p. 4-197, PDF p. 330; 3.2.3, p. 3-5)
Asked: 2026-10-21, by the lead agent (formerly open question 50; bead domain-pascal-j1bt)
Passage: p. 4-197: "With one exception, the size of the argument must be the same as the size of the destination type. ... This size equality is required because the type transfer function does not change any bits in the argument. ... The one exception is that integer subranges are always compatible regardless of their sizes."  3.2.3: "When you declare an integer constant, Domain Pascal internally represents the value as a 32-bit integer."  The gqp ruling (above, "integer constant to a pointer type") allowed a small constant "expanded to a pointer", but was asked of `1`.
Question: is a negative constant allowed (`univ_ptr(-1)`, `p(pbu_no_handler)` with `pbu_no_handler = -1`), and are its bits 16#FFFFFFFF (the 32-bit representation of 3.2.3) or 16#0000FFFF (a 16-bit -1 widened with zeros)?  Image: the SAU5 PBU DDF handler fields hold 16#FFFFFFFF (PBU_$ACQUIRE_INT 0xE588D4 `movea.l #-1,A1` stored into each of the 16 handlers, `move.l A1,(-0x48,A0)` 0xE588DA; PBU_$SETUP 0xE59842 `cmpi.l #-1,(0x40,A3)`), which cannot show the source spelling.
Owner's answer (2026-10-21): "Allow, 32-bit".  A type transfer of an integer constant to a pointer type is accepted; the constant is a 32-bit integer (3.2.3), so -1 becomes 16#FFFFFFFF, the same as `p(integer32(-1))`.
Consequence: implemented under bead domain-pascal-j1bt: `dpc-sema` (`Checker::transfer`) no longer refuses a negative constant; the result is the pointer constant -1, the same value `p(integer32(-1))` already gave, so it takes the paths of a 4-byte constant: `moveq #-1` and `move.l` stores, a `ffff ffff` code-section cell by address, `pea (-1).w` by value, `cmpi.l #-1` in a comparison.  Tests: sema `integer_constant_to_pointer_transfer`, golden `expr/transfer_neg_const_ptr`.  The gqp entry's "A negative constant is refused" is superseded.

## An unsigned word compared with 0 (4.3.3, p. 4-6)
Asked: 2026-10-21, by the lead agent (bead domain-pascal-wkm5)
Passage: 4.3.3: ">, >=, <, <=, max, div and mod" are interpreted as signed "except under the following circumstances: Both operands are unsigned run-time values.  One operand is an unsigned run-time value and the other is a constant in the positive subrange 0..2147483647.  For these two cases, the compiler uses unsigned operations."
Question: the image compares an unsigned word (a 0..65535-style subrange, zero-extended when widened, its other bound compared unsigned) with 0 by signed branches: sau5 MT 0xE75916 `move.w (22,A6),D1; ble; cmpi.w #0x4000,D1; bhi` (`(length <= 0) or else (length > 16#4000)`), sau2 SOCK_$PUT_INT 0xE161AA `tst.w D0; ble; cmpi.w #224,D0; bls`, and `>`/`<` forms at PKT_$BLD_INTERNET_HDR 0xE122D6, WIN_$DINIT 0xE19EDE and PBU_$GET_DCTE 0xE59212 (sau5).  Under 4.3.3 these would be unsigned (16#8000..16#FFFF above 0).  Does dpc follow the reference or the image?
Owner's answer (2026-10-21): "Reproduce image".  An unsigned word compared with the constant 0 by `<=`, `>`, `<` or `>=` takes the signed branch (`ble`/`bgt`/`bmi`/`bpl`), so 16#8000..16#FFFF count as below 0, as in the original; comparisons with other constants and between unsigned values keep 4.3.3's unsigned forms.  A byte (0..255) against 0 is unchanged (no site).  This departs from the reference: a candidate entry for ../source/docs/original-quirks.md (not ours to edit; the owner was told).
Consequence: implemented under bead domain-pascal-wkm5 (`dpc-ir` `compare`); codegen-patterns 7.1 "Signed or unsigned".


## 3.10.5.5 p. 3-26; 3.15.6.3 p. 3-72: a byte-aligned field crossing two word boundaries (MC68020 targets)
Asked: 2026-10-29, by the dpc session (open question 51; bead domain-pascal-gkrx, then domain-pascal-meel); relayed with the answer by the ../source session (2026-10-30).
Passage: "A byte aligned field may not cross two word boundaries" (p. 3-26); p. 3-72's Figure 3-27 puts a byte-aligned integer32 after a char at offset 2.  The SAU9 (MC68020) image reads longwords at byte 3 of word-aligned records: io_$match_device 0x3C439032 `move.l (3,A1),D3; cmp.l (3,A0),D3` (bytes 3..6 of an 8-byte PROM device record, the configuration record's devices 8 bytes apart), SCSI_WAIT 0x3C4778F2 (SCSI_$CB + 0x13), win_scsi_request_sense 0x3C41B284.
Question: did the MC68020 compiler lay a byte-aligned longword out without the crossing rule, or does another declaration give these offsets?
Owner's answer (2026-10-30): "Whatever is necessary to reproduce the image's layout, go for it."
Consequence: under the MC68020 cpu modes (`-cpu mathchip` and the other Table 6-3 MC68020 ids) an explicit `[aligned(0)]` on a field lifts the crossing rule; in a packed record the field starts at the next free byte instead of the next 16-bit boundary.  The reference's rule stays for a field without the attribute, for the default layout and for every MC68010 target, where an odd longword is an address error.  `crates/dpc-sema/src/types.rs` (`TypeTable::unaligned_access`, `layout_fields`, `layout_packed_fields`, `BitPos::is_whole_object`); tests `byte_aligned_field_crossing_two_words` (attrs.rs, packed.rs); golden `m68020/crossing_field`; codegen-patterns 14 "Odd addresses".  ../source writes `id: [aligned(0)] integer32` at byte 3 of its PROM device record (its reproducer g231: sizeof 8, `r.id` read with `move.l (3,An)`).  What the original source declared is not visible; the layout is the ruling's.
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
8. **Answered 2026-10-15; see Clarifications, "Module heading with an empty code section".**  **7.1.1, p. 7-2.** The empty-code-section form `name, , data_section`
   is shown for programs (2.2.1, p. 2-10) but not for modules.  *Now
   accepted for modules too (domain-pascal-zzru).*

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
    `unsigned_16_bit_subranges_are_words`).*  *Settled by the SAU9 image
    (2026-10-28): in a packed record the 32 bits of 3.4 are allocated
    where the field starts on a 16-bit boundary; after a boolean it takes
    its 31 bits (ast_$load_aote 0x3C403ADA `bfextu (A0){1:31}`, a 4-byte
    page-map entry; codegen-patterns 5, layout evidence; test
    `unsigned_31_bit_subrange_after_a_boolean`).*
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
15. **Answered 2026-10-16; see Clarifications, "Lshft of a 16-bit operand; the sign of a shift of an unsigned word".**  **"Lshft", p. 4-109 (printed 4-108/4-109).** The example uses
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
    *`addr` settled by the reference (no owner ruling needed; bead
    domain-pascal-mbtx):* "Addr", p. 4-13 (PDF p. 146, checked): "x Can
    be a variable declared as any data type except as a procedure or
    function data type having the internal attribute"; the only `addr`
    message of chapter 9 (Diagnostic Messages) is "Cannot take the
    address of internal routine identifier", and the message
    "Attempted assignment to (variable), a FOR-index variable, or
    formal parameter marked as IN" is about assignment only.  Apollo's
    own SR10.2 example `domain_examples/streams/sasc/imex.pas` does it:
    `PROCEDURE sasc_$import(IN mgr_data: UNIV string; ...)` with
    "`hptr := addr( mgr_data );`" (line 81).  So `addr` of an `in` parameter, or of a field or
    element of one, is accepted and compiled like a `var` parameter's
    (codegen-patterns 5.2, "`addr` of a parameter").  Writing through the
    address would break 5.3.2's "not allowed to alter its value", which
    nothing checks (as for any `univ_ptr`).  The other forms stay open.

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
    field's word or longword unit are refused at such addresses too, as
    are a field or compare of a record type with `[aligned(0)]` passed by
    address (or reached through a pointer, or another unit's object; bead
    domain-pascal-epjn), and the entry copy of a value parameter of such
    a record type, or of a 2-byte record that inherits a field's
    `[aligned(0)]` (bead domain-pascal-cr4a).  *MC68020 (`-cpu mathchip`,
    2026-10-29, bead domain-pascal-gkrx): none of these is refused; the
    MC68020 reads and writes words and longwords at odd addresses, and
    the SAU9 kernel does (codegen-patterns 14, "Odd addresses"; golden
    `m68020/odd_access`).  What the original declared where its SAU9
    code has longwords at odd offsets is item 51.*
26. **3.15.3, p. 3-57 (device record assignment).**  Is assigning or
    pushing a whole 4-byte `[device]` record as one 32-bit reference
    allowed, or does "The device attribute prevents this optimization"
    forbid it?  *Current: a device (or volatile) record or array copy
    is the ordinary block copy, longwords ascending, each unit read and
    written once (bead domain-pascal-e07e): the SAU5 image copies
    WIN_'s window descriptor this way, merging its word, byte and
    longword fields into three `move.l` (RESTORE 0xE19A1E..2A and
    0xE19A50..5C, SEEK 0xE19B14 / 0xE19B50), whatever its original
    declaration was (the image does not show it).*
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
    outside it legal?  *Current (bead domain-pascal-sq4a, 2026-10-19;
    before it, bead domain-pascal-o4q refused it): **best effort pending
    the owner's ruling**.  A jump into a `for`, `while`, `repeat`, `with`
    or `case` from outside it compiles as a plain jump to the label,
    with the warning `best effort (not byte-identical): a `goto` into a
    ...` (an error under `dpc --strict`; `crates/dpc-ir/src/lower.rs`,
    `routine_labels`).  The original compiler evidently accepted such
    jumps: ../source's MST_$TOUCH (0xE0DD68 `bcc.w 0xE0DF12`, `goto
    illegal` into a `with`), MST_$INVALIDATE (0xE442B8, `goto no_entry`
    into a `with` in a `repeat`) and MST_$FORK (0xE73A7C, `goto fail`
    into a `with` in two `for` loops) are read so from the image.  The
    statement entered is not set up (a loop's counter, a `with`'s base):
    code at the label that reads it, or a loop end reached from it, is
    undefined, as p. 4-95 leaves it; at the three kernel sites the code
    at the label leaves the statement first.  So dpc refuses a jump
    whose code at the label can run on to an entered `for` loop's end
    (the `dbf` on a count never loaded) or reads an entered `with`'s
    record (a `while` or `repeat` reached at its end only tests its
    condition: accepted).  Jumps between the arms of
    one `case` (p. D-4) and into `if` parts (p. 6-23, `-niso`) are
    accepted; a compound statement is transparent (accepted).  Exec test
    `be_sq4a`.*

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
    the code for them is not settled (bead domain-pascal-4x2).  The
    ruling is for 4-byte subranges; `for` guards of an `integer` index
    against `0..65535` (`pinteger`) bounds (question 47, bead
    domain-pascal-3iws) apply the same 4.3.3 rule to 2-byte unsigned
    words, relying on dpc-ir's existing treatment of `0..65535` (a
    subrange of `integer32` stored in 2 bytes) as unsigned, not on this
    ruling.*

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
38. **Answered 2026-10-15; see Clarifications, "Subranges and their base type as var arguments; out-of-range constants".**  **Messages 83 and 84, p. 9-16 (PDF p. 497); 5.1, p. 5-3.**  "Argument
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
    error, a warning, or accepted?  *Current: both accepted quietly for
    integer subranges of the parameter's size (bead domain-pascal-4tm).*
39. **Answered 2026-10-14 (refuse for now); see Clarifications, "Rshft, p. 4-171: rshft of a negative constant".**  **"Rshft", p. 4-171; 3.2.3, p. 3-5.**  "if num is negative and is a
    32-bit integer, the result of an rshft is always positive", and
    "Suppose num is 16 bits and equals -9. You perform an rshft with sh
    equaling 3 and put the result back in a 16-bit integer ... = -2".
    3.2.3 says integer constants are represented as 32-bit integers.  In a
    constant expression such as `c = rshft(-9, 3)`, is the value the
    32-bit result 16#1FFFFFFE (536870910), or does it depend on where the
    constant is used?  *Current: rshft of a negative constant is not
    folded, so it is refused in a const declaration (bead
    domain-pascal-95c).*
40. **3.15.5, pp. 3-60..3-62 (PDF pp. 115-117, checked); Table 3-2,
    p. 3-24.**  "byte_integer = [byte] 0..255; byte_int_array =
    array[1..1000] of byte_integer;" and "[byte] 0..255 corresponds to
    the unsigned char data type in Domain/C".  Table 3-2 gives the
    guaranteed default (field) and natural alignment of the simple types
    only ("char byte byte", "boolean byte byte", "integer16 word word");
    p. 9-41, message 305: "A value is naturally aligned if it begins at
    an address that is a multiple of its size in bytes."  What is the
    alignment of a `[byte]` integer subrange, freestanding and as a field
    of an unpacked record: byte, like `char`, or word, like the
    `integer16` host?  And is `array[1..3] of [byte] 0..255` 3 bytes
    (3.11.4.1 pp. 3-45..3-46: elements times their size, padding only for
    booleans)?  *Current (bead domain-pascal-5z7): byte aligned
    everywhere, 3 bytes; `../source`'s `tpad_$data_packet_t`
    (tpad/tpad.ins.pas) puts six such fields at 0xA..0xF, read with
    `move.b` in TPAD_$DATA (0xE691DC, 0xE691F0, 0xE69228), which fits
    either if they were `char`s, so the image does not settle it.
    Freestanding, the alignment answer no longer changes a layout:
    every frame object and every static object is at least word aligned
    whatever its type's alignment (codegen-patterns 2 "Frame layout:
    alignment", 11 "Byte-sized module variables", bead
    domain-pascal-yvko); the field and array-element cases, and the
    array's size, stay open.*

41. **Answered 2026-10-15; see Clarifications, "Narrowing integer32 to integer16".**  **5.1, p. 5-3 (PDF p. 358, checked); messages 79 and 83, pp. 9-15,
    9-16; 3.8, p. 3-14 (PDF p. 69); Appendix D, ISO 6.7.2.2, p. D-4 (PDF
    p. 561); "Type Transfer Functions", p. 4-197.**  5.1: "The data type
    of each argument must match the data type of its corresponding
    parameter. For example, if the second parameter is declared as
    integer16, the second argument must be an integer16 value."  3.8:
    "Currently, Domain Pascal does not support subrange checking."
    D-4: "Domain Pascal does not detect an overflow or underflow on
    integer arithmetic."  p. 4-197: "integer subranges are always
    compatible regardless of their sizes" (for type transfers).
    Appendix D, ISO 6.4.6, p. D-3 (PDF p. 560, checked): "You may make an
    assignment from an integer expression that includes a value outside
    one of the variables' declared subranges. Also, you may pass an
    integer argument that is outside the corresponding parameter's
    declared subrange."  ISO 6.6.3.5, 6.6.3.6, same page: "Domain Pascal
    treats integer and subrange of integer as identical."  ISO 6.6.6.4,
    p. D-4: "Succ, pred, and chr do not detect overflow."  So an
    out-of-range integer value may be assigned or passed to a declared
    subrange (`0..65535`, `[byte] 0..255`) without an error, but no
    section says whether `integer16` itself counts as such a subrange of
    `integer32`, nor what value the target then holds.  The image
    narrows such values (the low word
    of a longword stored or pushed: APP_$DEMUX 0xE00AF6, LOG_$ADD
    0xE1775E, DBUF_$GET_BLOCK 0xE3A5F6), but cannot show whether the
    source wrote a type transfer (`integer16(rshft(x, 5))`) or relied
    on an implicit conversion.  Question: (a) is an `integer32` expression
    accepted without a type transfer where an `integer16` is expected (a
    smaller declared subrange: D-3 above), in an assignment, a value or
    `in` argument and a function result?  (b) If so, is the value its low
    bytes, with no
    check or warning (also for a constant out of range, question 38)?
    *Current (bead domain-pascal-8td; codegen-patterns 8, "Narrowing and
    widening"): accepted (`dpc-sema` already treated all integer types as
    compatible, `ordinal_compatible`), the low bytes kept; widening by the
    value's sign; the forms the image does not show are refused.*

42. **3.11.4.2, Table 3-5, p. 3-47 (PDF p. 102, read in the page image;
    the OCR scrambles the table).**  The row "Subranges of integers /
    Enumerated / Subrange of enumerated" reads "Exact number of bits
    required *; bit aligned **", the row "Set" "Exact number of bits
    required ¹; bit aligned ²", and the table's only footnotes are "¹ If
    the element size is less than 16 bits, the element is padded up to
    the nearest power of 2 bits.  ² If the element size is greater than 16
    bits, then the elements are word aligned."  The text below the table
    applies footnote 1 to every element ("In packed arrays, if the element
    size is less than 16 bits, then the element is padded up to the
    nearest power of 2 bits"), and 3.15.5 p. 3-62 (PDF p. 117) gives the
    order ("from most significant bit to least significant bit") and the
    whole size of 1-, 2- and 4-bit arrays ("3 bytes" for six 4-bit
    elements), so elements of up to 16 bits are settled.  Question: do *
    and ** mean footnotes 1 and 2, and if so how many bits does an element
    of a subrange that needs 17 to 31 bits take (`packed array[1..4] of
    0..100000`, 17 bits): 17 bits, each element starting at the next word
    boundary (32 bits apart), or 32 bits?  *Current: refused (bead
    domain-pascal-51e; `crates/dpc-sema/src/types.rs`,
    `packed_array_elem`); a subrange of 32 bits (also an unsigned one of
    31, ref 3.4 p. 3-10) is stored as unpacked.  Sets (the row "Set",
    the same footnotes): 5 to 8 members one byte (SAU9 SCSI_$PPN_OWNERS,
    `packed array[...] of set of 0..7`, one byte per element in the
    image; codegen-patterns 5.1 "One-byte sets"; bead domain-pascal-nb0x),
    9 to 16 a word and more whole words as unpacked; 1 to 4 members (1,
    2 or 4 bits) refused, their bit order not settled.*

Raised by the overlay-section work (bead domain-pascal-raxl):

43. **7.2.4, Method 4, p. 7-7 (PDF p. 432, checked); 3.14, p. 3-52 (PDF
    p. 107, checked); 7.9.6.2, p. 7-45.**  Method 4 has `Program Math;
    VAR (my_sec) x : integer16 := 5; y : real := 6.2; q : array[1..3]
    of char := 'cat';` and `Module Sub2; VAR (my_sec) x : integer16; y :
    real; q : array[1..3] of char;`: "the variables x, y, and q from
    Math occupy the same memory locations as x, y, and q from Sub2".
    3.14: "A Domain Pascal overlay section is just like a named common
    block in FORTRAN"; 7.9.6.2: "the binder ensures that overlay sections
    with the same name refer to the same memory locations".  Neither
    says what happens when two units both give initial values to one
    overlay section (the same variable, or different ones), nor when the
    units declare it with different sizes.  Which unit's values does the
    binder keep, and is it an error?  *Current: a unit with initial
    values for the section defines it (`.data.overlay.<NAME>`), the
    others' common symbols resolve to it, and two defining units fail to
    link ("multiple definition"); sizes: the linker's common-block rule,
    the largest common size, but a defining unit's size when there is
    one, even if a common is larger (GNU ld says nothing when the
    definition comes first in link order, "size of symbol ... changed"
    when a larger common comes first; "common of ... overridden by
    definition" only with `--warn-common`; codegen-patterns 11, "Overlay
    sections").*
44. **3.14, p. 3-52; 7.1.1, p. 7-2; 5.7.1, p. 5-24.**  3.14: "To
    specify an overlay data section, place the section name inside a set
    of parentheses after the reserved word var"; 7.1.1 and 5.7.1 name
    the data section of a module heading (`module name, code, data;`)
    and of a routine (`[section(c, d)]`), whose static data the compiler
    puts there (5.7.1: "VAR seed : static real; { In "npd" }").  When a
    `var (name)` part names the unit's own data section (`module m, ,
    wired_data; var (wired_data) x: integer;`), is `x` part of the
    unit's contribution to wired_data, like its other static data, or
    a separate overlay section of the same name?  *Current: part of the
    unit's block in that section (dpc's reading; tools/srcsurvey.py
    relies on it to keep a stubbed routine's statics in their block).*
45. **7.2.4, p. 7-7 (PDF p. 432, checked) vs 3.3.4, p. 3-9 (PDF p. 64,
    checked) and 3.2.4, p. 3-5.**  7.2.4, of `VAR (my_sec) x :
    integer16 := 5; y : real := 6.2;` in Math against `y : real; x :
    integer16;` in Sub2: "when compiling Sub2, the compiler sees y as
    being the first four bytes in my_sec.  However, when compiling Math,
    the compiler sees y as being bytes 2 through 5 in my_sec", i.e. a
    `real` at offset 2 after an `integer16`.  3.3.4: "By default, Domain
    Pascal stores single-precision floating-point numbers (types real
    and single) on longword boundaries"; 3.2.4: "aligns ... 32-bit
    integers on longword boundaries".  The image's modules agree with
    3.2.4: all 102 map-named variables in compiled modules' data blocks
    that are accessed as longwords lie at 0 mod 4 (`tools/datasurvey.py`'s
    scan, ad-hoc query, not kept), none at 2 mod 4.  Is 7.2.4's "bytes 2
    through 5" an error, or are variables of an overlay section (or of a
    main program, Math) laid out differently?  Three other passages
    lean toward 7.2.4's reading, none states it for overlay sections:
    4.7, `%natural_alignment`/`%word_alignment`, p. 4-57 (PDF p. 190,
    checked): "By default, the Domain Pascal compiler aligns all objects
    larger than a byte in word-alignment mode"; 7.6, p. 7-17 (PDF p.
    442, checked): `VAR (my_sec) x : INTEGER16; d : DOUBLE;` shares
    memory with FORTRAN `INTEGER*2 x / REAL*8 d / COMMON /my_sec/ x,d`
    ("A named section in Domain Pascal is identical to the named common
    area of Domain FORTRAN"; the reference does not give the FORTRAN
    common's layout, but a common block's members are normally
    consecutive, d at byte 2); 7.9.6.2, p. 7-45: `VAR (example) x :
    INTEGER16; y : DOUBLE;` beside C's `struct { short x; double y; }
    example` (the C layout not given).  The image cannot settle it: no
    map symbol lies inside an overlay section, so where its variables
    start, and any padding between them, are not known
    (`tools/sectsurvey.py overlays -v` lists the accesses).  *Current:
    every static variable at its type's freestanding alignment (at least
    2), overlay sections included: y at offset 4.*

46. **"Type Transfer Functions", p. 4-197 (PDF p. 330, checked); C.3.19,
    p. C-11.**  "Domain Pascal type transfer functions enable you to
    change the type of a variable or expression within a statement" and
    "{Type transfer functions are functions.}"; the argument is "An
    expression."  The entry never says whether a transfer of a variable
    is itself a variable.  Can `T(v).f` (or `T(v)`) be an assignment
    target, be passed to a `var`/`out`/`in out` parameter, or have
    `addr` taken?  Nothing in ../source needs it yet (bead
    domain-pascal-w0rj).  *Current: reading `T(v).f` is the bytes of v
    at f's offset, read in place (PROC1_$CREATE_P 0xE15156); the three
    uses above are refused ("not supported yet").*

47. **"For", pp. 4-89..4-90 (PDF pp. 222-223, checked).**
    "start_exp: An expression matching the type of the index_variable";
    "If you set up a meaningless relationship between the start_exp and
    the stop_exp (for example, for x := 8 to 5 or for x := 10 downto 20),
    Pascal does not execute the loop even once."  The entry does not say
    what happens when a bound lies outside the index's type, as in `for
    b := 0 to n - 1` with `b: [byte] 0..255` and n = 0 (stop_exp -1), or
    `for i := 0 to n - 1` with `i: 0..2147483647` and an `integer32` n of
    0.  ISO 7185 6.8.3.9 compares the bounds' values and runs no pass.
    The image guards a `0..65535` index against a bound of the same width
    unsigned (PCHIST_$CNTL's nested 0xE5CD1A `tst.w; beq`, the bound
    declared `integer` in ../source's reconstruction), so a bound of -1
    there would run 65536 passes.  *Current (best effort, bead
    domain-pascal-1mtr): a run-time bound wider than the index is
    compared in a type holding both (no pass); a bound of the index's
    width in the index's sign, as the image (so `0..65535` against an
    `integer` -1 runs 65536 passes), except a `0..2147483647` index
    against a signed longword, compared signed.  Word indices held in a
    register, signed, are unchanged (the bound narrowed as an assignment
    narrows it), except against unsigned-word bounds (bead
    domain-pascal-3iws): an `integer` index whose run-time bounds are all
    unsigned words (`0..65535`, `pinteger`) and whose constant bound is
    non-negative is guarded unsigned (ref 4.3.3; question 32), as the
    image's flp_format_track 0xE3DC9E (`tst.w; beq`), so `for i := 1 to
    n` with n = 40000 runs 40000 passes; a bound computed from unsigned
    words whose value may leave the word (`n - 1`, `a + 1`) is kept at 32
    bits and compared there; from a negative constant the guard stays
    signed (codegen-patterns 7.4).  A run-time lower bound (bead domain-pascal-sq4a,
    codegen-patterns 7.4 "Run-time lower bounds") takes the same path:
    `sub.w from,T; bcs` for a `0..65535` index, so `for i := lo to k - 1`
    with lo = 0 and an `integer` k of 0 also runs 65536 passes.*  Awaiting
    the owner.

48. **3.14, p. 3-52; 7.2.2, Method 2, p. 7-4; 7.2.4, Method 4, p. 7-7.**
    3.14: "All the variables named in all the identifier_lists will be
    stored in section_name" and "Domain Pascal allocates variables
    defined in a var declaration part sequentially within the specified
    section"; 7.2 (p. 7-3): `extern` "tells Pascal to not allocate the
    variable"; Method 4 shares a section between units by position.
    Neither says where an `extern` member of a `var (section_name)` part
    lies: does it reserve its slot at its declaration (so `var (s) a:
    extern integer; b: integer;` puts b at +2 in every unit, whether or
    not the unit `define`s a), or does it take no slot until a `define`
    allocates it (b at +0 in a unit that only references a, and a placed
    after b, in `define` order, in one that defines it)?  *Current
    (bead domain-pascal-m15q): dpc allocates a `define`d `extern` member
    at the `define`, in `define` order, and refuses a unit whose overlay
    members would then not lie in their declaration-order slots (an
    `extern` member before others that are allocated, or `define`s out
    of declaration order); an undefined `extern` member after all the
    allocated ones, or one alone in its part (../source's planned
    ROUTE_$PORT in NET_PORT_TABLE), is accepted.  Awaiting the owner.*
49. **3.15.3, p. 3-57 (device read-modify-write).**  "The device
    attribute also prevents the compiler from generating gratuitous
    read-modify-write references for device registers.  That is,
    specifying a variable as device causes the compiler to avoid using
    instructions that do unnecessary reads."  The SR10.2 SAU5 image
    updates device registers in place with read-modify-write
    instructions: MBUS_$ENABLE_DEV 0xE0AD5E `or.w D0,(0xff7c02).l`,
    MBUS_$DISABLE_DEV 0xE0AD76 `and.w D0,(0xff7c02).l`, VME_$INT
    0xE0ABCC `andi.w #-0x4001,(0xff7c00).l` on PBU_CTL, which MBUS_$INIT
    stores with `move.w #0` (not `clr`); SM_$CB's packed unit field
    `andi.b #-4,(0x31,A0); ...; or.b D3,(0x31,A0)` (sm_$command
    0xE6F1A2).  Is a read the statement needs (`x := x op c`, a packed
    field's neighbouring bits) "gratuitous"?  *Current (bead
    domain-pascal-eaw7): not gratuitous: a device object is updated in
    place with the ordinary forms, and a packed field of a device record
    is stored with the field forms; only `clr`/`st`, which read an
    operand nothing needs, are never used on a device, and a
    read-modify-write instruction in a statement that does not read the
    object is refused.  A store into a packed field of a `device(write)`
    record (it reads the unit; p. 3-59 makes reading such an object an
    error) is refused.  The image shows the code, not the declarations,
    so whether the original declared these objects `[device]` is
    inferred from their `move #0` stores (codegen-patterns 5.5).*

50. **Answered 2026-10-21; see Clarifications, "Type transfer of a negative constant to a pointer type".**  **"Type Transfer Functions", p. 4-197 (PDF p. 330, checked); 3.2.3,
    p. 3-5.**  p. 4-197: "With one exception, the size of the argument
    must be the same as the size of the destination type. ... This size
    equality is required because the type transfer function does not
    change any bits in the argument. ... The one exception is that
    integer subranges are always compatible regardless of their sizes."
    3.2.3: "When you declare an integer constant, Domain Pascal
    internally represents the value as a 32-bit integer."  The gqp
    ruling (above, "integer constant to a pointer type") allows a small
    constant "expanded to a pointer", but it was asked of `1`.  Is a
    negative constant allowed (`univ_ptr(-1)`, `p(pbu_no_handler)` with
    `pbu_no_handler = -1`), and are its bits 16#FFFFFFFF (the 32-bit
    representation of 3.2.3, i.e. sign extension) or 16#0000FFFF (the
    bits of a 16-bit -1 kept and widened with zeros)?  The SAU5 PBU DDF
    handler fields hold 16#FFFFFFFF (PBU_$SETUP, PBU_$ACQUIRE_INT), but
    the image cannot show the source spelling (bead domain-pascal-j1bt).
    *Before the ruling: refused ("only constants >= 0 are accepted");
    `p(integer32(-1))` was accepted and was 16#FFFFFFFF.  Now `p(-1)` is
    the same constant.*

51. **Answered 2026-10-30; see Clarifications, "3.10.5.5 p. 3-26; 3.15.6.3 p. 3-72: a byte-aligned field crossing two word boundaries".**  **3.10.5.5, p. 3-26; 3.15.6.3, p. 3-72 (byte-aligned longwords at
    odd offsets in the MC68020 kernels).**  "A byte aligned field may not
    cross two word boundaries": an `[aligned(0)] integer32` after three
    chars goes to offset 4, and dpc places it there for both CPUs.  The
    SAU9 image reads and writes longwords at odd offsets: SCSI_WAIT
    0x3C4778F2 `move.l (35,A5),(-116,A6); sub.l (-116,A6),D5` (SCSI_$CB +
    0x13, used as a number), win_scsi_request_sense 0x3C41B284 `move.l
    (-53,A6),(0xcdb,A5,D0.l)` and 0x3C41B2A4 `move.l (-53,A6),(64,A2)`
    (a REQUEST SENSE answer's bytes 3..6, stored at byte 5 of a 10-byte
    log entry; ../source/docs/pascal-translation-notes.md gap 301), and
    IO_$INIT's module-local helper 0x3C439010 (io_$match_device in
    ../source) at 0x3C439032 `move.l (3,A1),D3; cmp.l (3,A0),D3`: bytes
    3..6 of two 8-byte PROM device records, reached through its two
    by-address parameters, whose word at offset 0 is tested for bit 8
    and whose byte at offset 2 is compared elsewhere (gap 231).  The
    image cannot show the declarations: a byte-aligned longword the
    MC68020 compiler lays out without the crossing rule, a four-char
    array copied as one longword, or a type transfer.  Did the rule
    differ for the MC68020 compiler (`-cpu` of ref 6.4.9), or does another
    declaration give these offsets?  *Current: the reference's rule for
    both CPUs; ../source declares `[aligned(0)] integer32` in a variant
    of its own and keeps a TODO(pascal) (win_scsi/request_sense.pas).
    An odd-address reference itself is no longer refused under `-cpu
    mathchip` (item 25).  After the ruling: the MC68020 modes lay an
    explicit `[aligned(0)]` field out across two word boundaries, at the
    next free byte of a packed record (bead domain-pascal-meel).*

52. **3.15.5, p. 3-61 (PDF checked: the text is as the OCR has it) with
    3.10.7, p. 3-35 (a `[byte]` size attribute on a packed record).**
    P. 3-61: "Every data type has a minimum size.  A size attribute must
    specify at least the number of bits required for the type to which
    it is applied", and Table 3-7 lists only simple types; 3.10.7: "The
    minimum size of a packed record is 16 bits."  The SAU9 SCSI driver
    reads the 53C90's one-byte registers (a status byte of five booleans
    and a 3-bit phase, the interrupt byte of eight booleans) with
    `move.b` into 2-byte locals it then tests as words: SCSI_$C90_INT
    0x3C4A57A4 `move.b (16,A0),(-72,A6)`, 0x3C4A57AA `move.b
    (20,A0),(-68,A6)`, then 0x3C4A57B0 `move.w (-68,A6),D1; btst #8,D1`
    and 0x3C4A57F2 `tst.w (-68,A6); bpl.s`.  A packed record of 8 bits
    is 2 bytes by 3.10.7, so the register fields must be one-byte objects
    of the same packed type: `[byte] c90_status_t` on the field (p. 3-60
    lists record fields among the objects a size attribute applies to),
    the one byte copied into the plain local.  Is "the number of bits
    required for the type" of a packed record the bits its fields occupy
    (so that `[byte]` is legal when they fit 8), or 3.10.7's 16-bit
    minimum (so that the original declared something else)?  *Current:
    the fields' bits; `[byte]` on a packed record of at most 8 bits is a
    one-byte object, byte aligned like `[byte] 0..255`, and the
    assignment between it and the plain type copies the one byte
    (`crates/dpc-sema/src/check.rs` `sized`, `crates/dpc-ir/src/lower.rs`
    `copy`; test `byte_sized_packed_record`; bead domain-pascal-7kpw).
    Uncertain.*

53. **5.3.3, p. 5-10 (`univ`) with 5.4, p. 5-13 (a routine's name as
    an argument).**  5.4: "You pass the name of a procedure or function
    just as you would any other parameter", for a procedure or function
    parameter; 5.3.3 lets `univ` suppress the type check of an argument
    against an ordinary parameter, and says nothing of routine names.
    SAU3 IIC_$EXISTS passes PBU_$ACQUIRE_INT's `handler: univ_ptr` (a
    `val_param` routine, so the pointer by value) as `movea.l
    #IIC_$INT,A3; move.l (A3),-(SP)` (0xE1C76C, 0xE1C78A): the first
    longword of IIC_$INT's code, where iic_setup_pbu in the same unit
    pushes the address (`move.l #IIC_$INT,-(SP)`, 0xE7136E).  That is
    what a compiler gives for a routine's name taken as a variable (its
    code being its storage) passed by value to a 4-byte `univ` formal;
    likely an Apollo bug, but the code is what it is (../source finding
    400; bead domain-pascal-tbb4).  Is a routine's name accepted as the
    argument of a `univ` value parameter, and does it pass the routine's
    first bytes, not its address?  *Current: refused ("procedure `hand`
    used as a value"); ../source writes `iic_code_ptr_t(addr(iic_$int))^`,
    which compiles to the image's two instructions.*

54. **3.10.6, p. 3-33 (an array of `aligned record`s as a field of a
    record laid out by the word rules).**  3.10.6 says an aligned
    record is laid out naturally "even if [it is] in programs compiled
    with a %word_alignment directive", and Table 3-2 (p. 3-24) aligns
    "structured types, except for sets" on word boundaries; neither says
    where an *array* of aligned records goes as a field.  The SAU3 IIC
    line record (0x1B4 bytes) has its 10-entry packet-type table at
    +0x13C, after a 16-byte table that ends at +0x13A, and every entry
    is 12 bytes of which the last word is never addressed: an
    `aligned record` entry (10 bytes, rounded to 12 by its natural
    alignment) fits the stride, and the array's place fits the
    element's alignment (4).  The same reading (an `aligned record`
    ml_$exclusion_t, 18 bytes rounded to 20) explains every exclusion
    lock of ../source's records starting at a multiple of 4 with the
    next field at +0x14 (../source finding 402; bead
    domain-pascal-yur2).  Is an array whose elements are aligned
    records aligned on the elements' alignment when it is a field?
    *Current: word aligned (+0x13A); ../source keeps a stand-in word.*

## Reference errata noticed (no ruling needed)

- **2.1.2, p. 2-2**: the integer example uses C-style `/* ... */`
  comments; owner ruling (question 4): a copy-edit miss, not a comment form.

- **Table 4-11, p. 4-45** swaps the descriptions of `%push_alignment` and
  `%pop_alignment`.  The prose on p. 4-58 is followed: push saves, pop
  restores.
- **pp. 4-50/4-51**: the OCR of the `%warning` example is garbled.
  `%warning` takes a plain `'string'`, as Table 4-11 shows.  *Superseded
  (2026-10-20):* the PDF shows `%warning('...');`, and the owner ruled
  both forms valid ("%warning('string') and %error('string')" above).
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
- **4.7 "%warning", p. 4-52** (PDF p. 185): the example's string
  "'You forgot to use the -CONFIG color option." has no closing quote, and
  the text after it says "prints out the following error message" for a
  warning.  The tests close the quote (bead domain-pascal-zf19).
- **Table 6-2, p. 6-8** (PDF p. 389): `-std` "Issue warning messages when
  nonstandard language elements are encountered"; 6.4.27 (p. 6-35) and
  6.4.18 (p. 6-23) say error messages.  No consequence: dpc does not
  implement `-std` (docs/compiler-options.md).
