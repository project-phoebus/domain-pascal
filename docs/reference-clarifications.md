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
3. **4.7, p. 4-46, predicates.** "Optional Boolean keywords not, and, or
   or": no precedence is given.  *Current: Table 4-3 order, not > and > or.*
4. **2.1.2, p. 2-2 (PDF p. 35).** The integer example uses
   `/* default (base 10) */` comments, but 2.1.4 lists only `{ }`,
   `(* *)` and `" "`.  Is `/* */` a comment form?  *Current: rejected,
   per 2.1.4.*

## Reference errata noticed (no ruling needed)

- **Table 4-11, p. 4-45** swaps the descriptions of `%push_alignment` and
  `%pop_alignment`.  The prose on p. 4-58 is followed: push saves, pop
  restores.
- **pp. 4-50/4-51**: the OCR of the `%warning` example is garbled.
  `%warning` takes a plain `'string'`, as Table 4-11 shows.
