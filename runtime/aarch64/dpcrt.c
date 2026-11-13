/* runtime/aarch64/dpcrt.c - dpc's run-time library for aarch64 (macOS,
   user mode; docs/aarch64.md, runtime/README.md).

   The routines compiled Pascal calls on this target, in C under Apple's
   arm64 calling convention, which dpc's aarch64 output follows: argument
   k in xk, a value of less than 32 bits extended to 32 by the caller.
   Names keep Apollo's `$` (clang accepts it in identifiers) and are upper
   case, as dpc's external symbols are.

   Data is little-endian (owner, 2026-11-12): a varying string's length
   word and an enumeration name table's count are native words.

   - PAS_$WRITE / PAS_$WRITELN / PAS_$OUTPUT: `write`/`writeln` to the
     standard output (ref "Write, Writeln", pp. 4-215..4-220), with the
     headings of ../source lib/ftnlib/pasio.pas (`pas_$write(fmt; var f;
     in v; width; digits; in names); options(val_param)`), and the format
     word of dpc-ir's lower/write.rs: high byte the default width, bits
     7..6 the width's form, 5..4 the size, 3..0 the kind.
   - PAS_$NEW / PAS_$DISPOSE: `new` and `dispose` (pp. 4-117, 4-64), the
     headings of ../source lib/ftnlib/new_dispose.pas: `pas_$new(out p:
     univ_ptr; size: linteger); options(val_param)`, `pas_$dispose(in out
     p: univ_ptr; size: linteger)`.  Dispose sets p to nil (p. 4-64). */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* The standard output file: dpc passes its address; its contents are not
   used. */
char PAS_$OUTPUT[16];

static void pad_out(int n)
{
    while (n-- > 0)
        putchar(' ');
}

static int16_t rd16(const void *p) { int16_t v; memcpy(&v, p, 2); return v; }
static int32_t rd32(const void *p) { int32_t v; memcpy(&v, p, 4); return v; }

void PAS_$WRITE(int16_t fmt16, void *f, const void *v, int16_t width,
                int16_t digits, const void *names)
{
    unsigned fmt = (uint16_t)fmt16;
    int form = (fmt >> 6) & 3, size = (fmt >> 4) & 3, kind = fmt & 15;
    int w = form > 0 ? width : (int)(fmt >> 8);
    const uint8_t *b = v;
    char buf[128];
    (void)f;
    switch (kind) {
    case 0: case 1: case 8: case 9: {
        long x;
        if (kind == 1) x = rd32(b);
        else if (kind == 9) x = (uint32_t)rd32(b);
        else if (size == 1) x = kind == 0 ? (long)(int8_t)b[0] : (long)b[0];
        else x = kind == 0 ? (long)rd16(b) : (long)(uint16_t)rd16(b);
        int n = snprintf(buf, sizeof buf, "%ld", x);
        pad_out(w - n);
        fputs(buf, stdout);
        break;
    }
    case 2:
        pad_out(w - 1);
        putchar(b[0]);
        break;
    case 3: case 10: {
        int n = kind == 3 ? width : (uint16_t)rd16(b);
        const uint8_t *c = kind == 3 ? b : b + 2;
        int field = kind == 3 ? digits : width;
        if (form > 0) {
            if (field < 0) { while (n > 0 && c[n - 1] == ' ') n--; }
            else if (field > n) pad_out(field - n);
            else n = field;
        }
        fwrite(c, 1, n, stdout);
        break;
    }
    case 4: {
        const char *t = (int8_t)b[0] ? "TRUE" : "FALSE";
        pad_out(w - (int)strlen(t));
        fputs(t, stdout);
        break;
    }
    case 5: {
        uint32_t u = size == 3 ? (uint32_t)rd32(b) : size == 1 ? b[0] : (uint16_t)rd16(b);
        const uint8_t *p = (const uint8_t *)names + 2;
        for (uint32_t i = 0; i < u; i++) p += p[0] + 1;
        int n = p[0];
        pad_out(w - n);
        fwrite(p + 1, 1, n, stdout);
        break;
    }
    case 6: case 7: {
        /* A single or a double, with `:width:digits' (form 2); without
           digits, the m68k test runner's form is not modelled either. */
        double d;
        if (kind == 6) { float x; memcpy(&x, b, 4); d = x; }
        else memcpy(&d, b, 8);
        if (form != 2) {
            fprintf(stderr, "PAS_$WRITE: a real without digits is not supported\n");
            abort();
        }
        printf("%*.*f", width, digits, d);
        break;
    }
    default:
        fprintf(stderr, "PAS_$WRITE: kind %d not supported\n", kind);
        abort();
    }
}

void PAS_$WRITELN(void *f)
{
    (void)f;
    putchar('\n');
}

void PAS_$NEW(void **p, int32_t size)
{
    *p = calloc(1, size > 0 ? (size_t)size : 1);
    if (!*p) {
        fprintf(stderr, "PAS_$NEW: out of memory\n");
        abort();
    }
}

void PAS_$DISPOSE(void **p, int32_t size)
{
    (void)size;
    free(*p);
    *p = NULL;
}
