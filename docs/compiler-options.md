# Compiler options: the reference's and dpc's

Bead domain-pascal-zf19.  The reference's compiler options are listed in
section 6.4, "Compiler Options", Table 6-2 (pp. 6-6..6-8, PDF pp.
387..389), and described one by one in 6.4.1..6.4.31 (pp. 6-9..6-36).
The table and the descriptions were read in the PDF (the OCR scrambles
the table and its default marks).  This file lists every option, what
the reference says, and what dpc does with it.  The code is
`crates/dpc/src/refopts.rs` (the option table) and `crates/dpc/src/main.rs`
(the driver); the tests are `crates/dpc/tests/options.rs`, the unit
tests in those two files, and the preprocessor's
(`crates/dpc-syntax/src/preprocess.rs`).

## How dpc reads its command line

- **Spelling.**  A reference option is spelt as in the reference: one
  dash, lower case, in full ("Note that you cannot abbreviate these
  options", 6.3, p. 6-4).  `-CONFIG` is not `-config`.
- **Precedence.**  An argument spelt exactly as a reference option is that
  option.  Otherwise dpc's own options are tried: `-I dir`, `-Idir`,
  `-o file`, `-h`, and the `--long` options (`--strict`,
  `--fragment-origin`, ...), which are unchanged.  No reference option is
  spelt like one of dpc's: `-idir`, `-imap`, `-info`, ... begin with a
  lower-case `i`, and `-I` is upper case, so `-Idir` still means `-I dir`.
  Any other argument beginning with `-` is an error.
- **Operands.**
  - A list (`-config var1 ... varN`, `-idir dir1 ... dirN`) takes every
    following argument up to the next one that begins with `-`.
  - An optional operand (`-b [pathname]`, `-l [pathname]`) is the next
    argument if it does not begin with `-`; `-opt [n]` and `-info [n]`
    take the next argument only if it is a number.
  - So the source file goes **first**, as in the reference's
    `pas source_pathname [options]` (6.3, p. 6-4): `dpc m.pas -config a b`.
    Or it goes after `--`, which ends the options: `dpc -config a b --
    m.pas`.  When a list has taken the file name, the "no input file"
    error says so.
- **Repeats.**  An option of a pair (`-x`/`-nx`) or group
  (`-db`/`-ndb`/`-dba`/`-dbs`) may be given once between them; a second
  one is an error ("`-nwarn` conflicts with `-warn`").  This is dpc's
  choice, as for its own options: the reference does not say which would
  win.  `-config` and `-idir` may be repeated, and their lists add up;
  `-I` and `-idir` directories are searched in command-line order.
- **Refused options** are usage errors (exit status 2), with the reason
  and the reference section.  **Noted options** print one line
  `dpc: note: <option>: <what dpc does not do> (ref ...)` on stderr and
  compile as usual.

## The options

"Default" is the reference's default for the MC680x0 compiler variants.
"dpc" says what dpc does:

- **implemented**: dpc does what the reference describes;
- **accepted**: dpc already behaves this way, or the option has no effect
  on what dpc produces; silent;
- **noted**: accepted, with a note on stderr, because the reference
  describes something dpc does not produce (a listing, debugger tables);
  the generated code is the same;
- **refused**: accepting it would change the generated code in a way the
  reference does not define precisely, or that dpc does not implement.

The default forms keep dpc's output byte-identical to its output without
the option.

| Option | Ref | Default | Meaning (reference) | dpc |
|---|---|---|---|---|
| `-ac` | 6.4.1, p. 6-9 | yes | absolute code: the load address is set at compile time | accepted (dpc's code is absolute) |
| `-pic` | 6.4.1, p. 6-9 | | position-independent code | refused |
| `-alnchk` | 6.4.2, p. 6-9 | Series 10000 only (note 1) | messages about data alignment | noted (dpc issues none) |
| `-b [pathname]` | 6.4.3, pp. 6-9..6-10 | yes | object file `source.bin`, or `pathname.bin` | implemented: the output (assembly) is `pathname.s`, or `pathname` if it ends in `.s`; without pathname, the default name; does not mix with `-o` |
| `-nb` | 6.4.3, p. 6-9 | | no object file ("check your source code for grammatical errors") | implemented: the whole compilation runs (errors are reported) but nothing is written; does not mix with `-o` |
| `-bounds_violation` | 6.4.4, p. 6-10 | yes | references beyond an array's end allowed; less optimisation | accepted |
| `-no_bounds_violation` | 6.4.4, p. 6-10 | | the optimiser may assume no reference beyond an array's end | refused (the code it changes is not described) |
| `-comchk` | 6.4.5, p. 6-11 | | warn of every additional left comment delimiter of the comment's own kind | implemented: warning "Unbalanced comment; another comment start found before end." at each extra `{` in `{ }` or `(*` in `(* *)` (`Lexer::set_comchk`) |
| `-ncomchk` | 6.4.5, p. 6-11 | yes | no comment checking | accepted |
| `-compress`, `-ncompress` | 6.4.6, p. 6-12 | `-compress` | object file data compressed (`.rwdi`) or not | accepted (dpc writes assembly; the object file is the assembler's) |
| `-cond` | 6.4.7, p. 6-12 | | compile the lines marked `%debug` | implemented (`preprocess::Options::cond`, already in the preprocessor) |
| `-ncond` | 6.4.7, p. 6-12 | yes | `%debug` lines are comments | accepted |
| `-config var1 [... varN]` | 6.4.8, pp. 6-12..6-14; 4.7, pp. 4-45..4-52 | (not given) | set conditional variables to true | implemented; see "-config" below |
| `-cpu id` | 6.4.9, pp. 6-14..6-17; Table 6-3, p. 6-16 | `mathlib_sr10` | target workstation (processor, floating-point unit) | implemented for the processor: `any`, `m68k`, `160`, `460`, `660`, `peb`: MC68010 code (dpc's default, the SAU2..SAU5 kernels'); `mathlib_sr10`, `mathlib`, `mathchip`, `3000`, `580`, `570`, `560`, `330`, `90`, `fpa1`, `fpx`: MC68020 code (the SAU6..SAU9 kernels'; codegen-patterns 14), noted since there is no floating-point code to select; `a88k`: refused; others: error.  dpc's default is not the reference's `mathlib_sr10`: the SAU2 build must not change under an unstated option (`dpc_m68k::Cpu`) |
| `-db` | 6.4.10, p. 6-18 | yes | minimal debugging information | noted (dpc emits none) when given; nothing when defaulted |
| `-ndb` | 6.4.10, p. 6-18 | | no debugging information | accepted |
| `-dbs` | 6.4.10, p. 6-18 | | full debugging information; sets `-opt` (3) | noted (no debugging information; `-opt 3` is dpc's level) |
| `-dba` | 6.4.10, p. 6-18; 6.4.24, p. 6-27 | | full debugging information; `-opt 0` whatever `-opt` says; variables stored after every statement | refused |
| `-exp` | 6.4.11, p. 6-18 | | assembly listing interleaved with the source (implies `-l`) | noted (no listing file) |
| `-nexp` | 6.4.11, p. 6-18 | yes | no expanded listing | accepted |
| `-frnd` | 6.4.12, pp. 6-19..6-20 | | round floating point at key points | refused (no floating point) |
| `-nfrnd` | 6.4.12, p. 6-19 | yes | floating point in greater precision | accepted |
| `-idir dir1 [... dirN]` | 6.4.13, pp. 6-20..6-21 | | search these directories for relative include names, after the working directory; at most 63 per option | implemented (as `-I`; the search order of 6.4.13 is the preprocessor's already) |
| `-imap`, `-nimap` | 6.4.14, p. 6-21 | `-nimap` | symbol table maps for included files (implies `-map`) | `-imap` noted (no map); `-nimap` accepted |
| `-indexl` | 6.4.15, p. 6-21 | | 32-bit indexing in every subscript; "disables some optimizations" | refused (which ones is not said) |
| `-nindexl` | 6.4.15, p. 6-21 | yes | 16- or 32-bit indexing from the array's declaration | accepted |
| `-info [n]`, `-ninfo` | 6.4.16, pp. 6-21..6-22 | `-info 2` | information messages up to level n (0..4); `-info` alone is level 2; `-ninfo` is `-info 0` | accepted; levels 1..4 given explicitly are noted (dpc issues no information messages); a level outside 0..4 is an error |
| `-inlib pathname` | 6.4.17, pp. 6-22..6-23 | | the loader loads a PIC library at run time and binds its globals | refused (dpc's output names no library) |
| `-iso` | 6.4.18, p. 6-23 | | ISO rules for `mod`, unmatched comment delimiters, goto into an if; turns on `-std` | refused |
| `-niso` | 6.4.18, p. 6-23 | yes | Domain Pascal rules | accepted |
| `-l [pathname]` | 6.4.19, p. 6-24 | | listing file (source with line numbers, statistics, section summary, error count) | noted (no listing file; the reference does not define its format closely enough to reproduce) |
| `-nl` | 6.4.19, p. 6-24 | yes | no listing file | accepted |
| `-map`, `-nmap` | 6.4.20, pp. 6-24..6-25 | `-nmap` | listing plus a symbol table map | `-map` noted; `-nmap` accepted |
| `-msgs`, `-nmsgs` | 6.4.21, p. 6-25 | text: `-msgs`; Table 6-2: `-nmsgs`; dpc: `-nmsgs` (owner ruling) | final report "xx errors, yy warnings, zz info msgs, Pascal compiler variant Rev n.n" | implemented: `-msgs` prints, on stderr, e.g. `No errors, 1 warning, no info msgs, dpc 0.1.0` (counts "no" or a number, singular for one, as the examples on pp. 4-51, 4-52; dpc's name and version for the compiler's); dpc's default stays no report |
| `-natural` | 6.4.22, pp. 6-25..6-26 | | natural alignment for data without alignment attributes or directives; "the same effect as the %natural_alignment compiler directive" | implemented: the unit starts in natural alignment (`dpc_sema::Options::natural`); `%word_alignment` and `%pop_alignment` act on it as usual |
| `-nnatural` | 6.4.22, p. 6-25 | yes | word alignment | accepted |
| `-nclines` | 6.4.23, p. 6-26 | | no COFF line number tables (made only with `-dba`/`-dbs`) | accepted (dpc makes none) |
| `-opt [n]` | 6.4.24, pp. 6-26..6-32 | `-opt 3` | optimisation level 0..4; `-opt` alone is 3 | `-opt` and `-opt 3` accepted; 0, 1, 2 and 4 refused: dpc generates one level, the kernel's, taken to be the default (`docs/opt-levels.md`: the image fits `-opt 3` or `-opt 4`, undecided); a level outside 0..4 is an error |
| `-nopt` | 6.4.24, p. 6-26 | | obsolete; `-opt 0` | refused |
| `-prasm`, `-nprasm` | 6.4.25, p. 6-33 | `-prasm` (Series 10000 with `-exp` only) | listing format; "If you use these options with the compiler variants that generate MC680x0 code, they have no effect." | accepted |
| `-slib [pathname]` | 6.4.26, pp. 6-33..6-34 | | precompile an include file into a `.plb` library | refused (as `%slibrary`) |
| `-std` | 6.4.27, p. 6-35 | | report nonstandard (non-ISO) elements; Table 6-2 says warnings, the text error messages | noted (dpc does not report them) |
| `-nstd` | 6.4.27, p. 6-35 | yes | no such reports | accepted |
| `-subchk` | 6.4.28, p. 6-35 | | code at every subscript that "signals an error" when out of range | refused (the checking code is not described) |
| `-nsubchk` | 6.4.28, p. 6-35 | yes | no subscript checks | accepted |
| `-version` | 6.4.29, p. 6-35 | | print the compiler's version; no file name may be given | implemented: `dpc -version` alone prints `dpc 0.1.0`; with anything else it is a usage error |
| `-warn` | 6.4.30, p. 6-35 | yes | report warnings | accepted |
| `-nwarn` | 6.4.30, pp. 6-35..6-36 | | suppress warnings, "though it does report on the total number of warnings that would have been issued" | implemented: no warning is printed (`%warning`, `-comchk`, best-effort warnings); `-msgs` still counts them |
| `-xrs` | 6.4.31, p. 6-36 | yes | registers are assumed saved across calls to external routines and through routine pointers | accepted (dpc's calls assume this) |
| `-nxrs` | 6.4.31, p. 6-36 | | registers are not assumed saved | refused |

Note 1: see "Owner rulings" below.

Not options but nearby: the default output name.  The reference names the
object `source.bin`, replacing a `.pas` suffix and otherwise appending
`.bin` (6.3.1, p. 6-4; Table 6-1, p. 6-5: `test.first` gives
`test.first.bin`).  dpc's default output is the input with its extension
replaced by `.s` (`test.first` gives `test.s`), as before this bead.

## -config

What the reference says (6.4.8, p. 6-12; 4.7, pp. 4-45..4-52):

- `-config var1 [... varN]` sets conditional variables to true; "var must
  be a conditional variable declared with %var" (p. 6-12).
- `%var` declares them, false by default; `%enable` and `-config`
  "perform the same function" (p. 4-50).
- Setting the same variable to true more than once, "with %enable or
  -config", is an error (p. 4-50).
- `%config`, usable only in a predicate, is true if the command line
  contains `-config` and false otherwise (p. 4-50).
- The predeclared variables (`_ISP__M68K`, `_ISP__A88K`, `_BFMT__COFF`,
  and by the owner's ruling `isp_$m68k`, `isp_$a88k`, `obj_$coff`,
  `obj_$bin`) cannot be set with `-config` (p. 4-48).
- `%if` tests whether a variable is true, `%ifdef` whether it is declared
  with `%var` (p. 4-49).  A name used in a predicate must be declared
  (p. 4-50).
- Names are case-insensitive (2.1.6).

What dpc does (`crates/dpc-syntax/src/preprocess.rs`):

| Case | Result |
|---|---|
| `-config a`, then `%var a` | `a` is true (`%if a` takes its branch) |
| `-config a b c` | each name is set; `%config` is true |
| `-config A`, then `%var a` | true: names are compared without case |
| `-config a`, `%var a`, then `%enable a` | error "already set to true" (p. 4-50) |
| `-config a A` (or `-config a -config a`) | error "set to true more than once by -config" (p. 4-50) |
| `-config _ISP__M68K` (any predeclared name) | error (p. 4-48) |
| `-config a`, no `%var a` | a warning "-config variable `a` is not declared with %var" (owner ruling); `%ifdef a` is false (`-config` does not declare); `%if a` is the usual "not declared with %var" error (p. 4-50) |
| `-config a`, `%var a` inside a branch not compiled | not declared, as above |
| no `-config` | `%config` is false; every `%var` name is false until `%enable` |
| `-config` with no name, or a name that is not a conditional variable name (`1a`, `a-b`) | usage error |

Tests: `config_*` and `reference_color_mono_examples` in
`preprocess.rs`, and `config_*` in `crates/dpc/tests/options.rs`
(through the `dpc` binary).  Between them they compile the reference's
own examples with and without `-config`:

- the `%error` example (p. 4-51): nothing without `-config` or with
  `-config color`; with `-config mono` the error "I have not finished the
  code for a monochromatic node." and no output file;
- the `%warning` example (p. 4-52): the warning "You forgot to use the
  -CONFIG color option." without `-config color` (also with `-config
  mono`), and an output file; nothing with `-config color`; hidden by
  `-nwarn` but counted by `-msgs`;
- the `%config` example (pp. 4-50..4-51; preprocessor test only): its
  warning only with `-config` naming neither `color` nor `mono`;
- Figure 6-2, `config_example` (p. 6-13), with "-config first third"
  (p. 6-14): the first and third blocks only.


Departures from the printed examples: the `%warning` string on p. 4-52
has no closing quote (an erratum; closed in the tests), and Figure 6-2's
`writeln` statements are replaced by routines (dpc has no I/O).  Figure
6-2's `%VAR first, second, third` and the `%config` example's
`%warning('...');` are compiled as printed (owner rulings below).

## Owner rulings (2026-10-20)

The questions this inventory raised were answered by the owner; the
rulings are recorded in `docs/reference-clarifications.md`:

- "A -config name no %var declares": a warning naming it; the unit
  compiles.
- "Commas between %var and %enable names": blanks, commas, or both.
- "%warning('string') and %error('string')": both forms, for both
  directives.
- "The default of -msgs": `-nmsgs` stays dpc's default, although the
  reference's transcripts (pp. 6-14, 4-51, 4-52) show the summary.

Note 1, `-alnchk` (not a question): Table 6-2 stars the row, but its text
reads "Display messages about alignment of data structures.  Default for
Series 10000 compiler variants." (p. 6-6), which agrees with 6.4.2
(p. 6-9): "The -alnchk option is always set for the compiler variants
that generate Series 10000 code."  It is not a default for the MC680x0
variants.

Errata noticed here (p. 4-52's unclosed `%warning` string; `-std`'s
warnings versus errors) are in the reference-clarifications errata list.

Not reference questions, but open: which `-cpu` each kernel was compiled
with (SAU2..SAU5 are MC68010 code with M$ARITH calls, SAU6..SAU9
MC68020 code: one of the `mathchip` group or `mathlib_sr10`, which
generate the same integer code as far as dpc is concerned; codegen-
patterns 14), and whether it was `-opt 3` or `-opt 4`
(`docs/opt-levels.md`).

## ../source's build: DPC_FLAGS

../source's `domain_os/Makefile` passes its compiler options in the make
variable `DPC_FLAGS`, before `-o`:

```make
DPC_FLAGS := -config sau2

$(PAS_BUILD_DIR)/%.s: %.pas $(PAS_SRCS_ALL) $(DPC)
	@$(DPC) $(DPC_FLAGS) -o $@ $<
```

(and the same `$(DPC) $(DPC_FLAGS) -o $$s $$m` in `pascal-report`).  The
flags go before `-o`: `-config`'s list ends at the next option, so `-o`
ends it, never the module's file name.

`tools/linktest.py` and `tools/srcsurvey.py` compile ../source's modules
as that Makefile does, and pass the same flags first (`dpc $(DPC_FLAGS)
-o ...`, `dpc $(DPC_FLAGS) -I . ...`).  `tools/dpcflags.py` reads them
from the copied tree's Makefile: it asks make for `$(DPC_FLAGS)` (`make
-s -f Makefile -f - <query>`, so any form of assignment counts), falls
back to reading the Makefile's own `DPC_FLAGS` assignments when make
fails, and else uses none.  `linktest.py --make-var DPC_FLAGS=...`
overrides it, as it does for make.  Both tools print the flags they use
(`linktest: DPC_FLAGS -config sau2 (make)`).
