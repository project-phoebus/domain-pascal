# Golden tests

Each test is a small Pascal file, `tests/golden/<area>/<name>.pas`, next to
the assembly the compiler is expected to produce, `<name>.s`.  The expected
`.s` is normalised: no comments, canonical spacing.

- There is one test per pattern in `docs/codegen-patterns.md`.  The `.pas`
  header comment names the pattern's section and the image address it
  mirrors.
- A change to an expected `.s` needs a reason in the commit message.

See CLAUDE.md section 6, item 2.  No tests yet.
