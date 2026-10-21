/*
 * tools/exectest/runner.c - run dpc output on the Musashi 68010 emulator.
 *
 * The execution-test harness (tests/exec/README.md, CLAUDE.md section 6.5
 * done on m68k; bead domain-pascal-f03).  tools/exectest.py links a test
 * module with tools/exectest/stubs.s and M$ARITH (../source's arith.s),
 * converts it to a flat binary and runs this program on it:
 *
 *   runner [--trace] [--max-steps N] --bin FILE --base ADDR [--end ADDR]
 *          [--text-end ADDR] [--stub NAME=ADDR]... --call 'ADDR [ARG]... [-> KIND]'...
 *
 * Memory is a flat 16 MB (the 68010's 24-bit bus).  The binary is loaded
 * at ADDR (--end: the end of its bss, past the flat file); the rest is zero except the stack region, which is filled with
 * a 0xA5 pattern so that a read of an uninitialised frame slot shows up.
 *
 *   0x000000..0x0003FF  exception vectors; vector n points to
 *                       VEC_HANDLERS + 2n, so landing there identifies n.
 *   0x000E00            RET_SENTINEL: the return address of every --call.
 *   0x000F00..          VEC_HANDLERS.
 *   0x060000..0x06FFFF  argument cells for by-address --call arguments.
 *   0x070000..0x07FFFF  the stack; SP starts at STACK_TOP.
 *   ADDR..              the test binary (tools/exectest.py links at 0x10000).
 *
 * The CPU is single-stepped.  Before each instruction the PC is checked:
 *
 *   - at a stub (a symbol of stubs.s, named by --stub), the stub is run in
 *     C: it logs its argument and returns as a Pascal-called routine
 *     would, clobbering D0/D1/A0/A1 (except its result) and the CCR
 *     (an EXEC_$NOSAVE* stub also D2-D7/A2-A4, as an options(nosave)
 *     routine may);
 *   - at RET_SENTINEL the call is complete: D2-D7/A2-A6 must hold the
 *     canaries loaded before the call, SP must be just above the return
 *     address (the caller pops the arguments), and a guard area above
 *     the arguments must be intact;
 *   - in VEC_HANDLERS an exception was taken: address error (a word or
 *     longword access at an odd address), illegal instruction (including
 *     68020-only encodings, which Musashi refuses in 68010 mode), zero
 *     divide, ...: the run fails.
 *
 * EXEC_$COUNTER (stubs.s) is a word in the data that behaves like a device
 * register: each word read by the program returns the next value.
 *
 * Accesses outside the binary, the stack and the argument cells are
 * reported as wild and fail the run; so are writes below --text-end (the
 * code and its constants), any access to low memory (a nil pointer), and
 * a stub argument that points outside them.
 *
 * Arguments of --call, pushed right to left after a pad when their slots
 * do not total a multiple of 4 (codegen-patterns 1.3, word slots):
 *
 *   l:N  w:N  b:N        a value in a 4-, 2- or 2-byte slot (a byte in the
 *                        high half of its word slot)
 *   @l:N @w:N @b:N       by address: N in a fresh argument cell, its
 *                        address pushed
 *   suffix !             after the call, print `out N' with the cell's value
 *
 *   -> l | w | uw | b | c | a0   print the result `=> V': D0 (signed long,
 *                        signed word, unsigned word, boolean byte,
 *                        unsigned byte) or A0 (hex).
 *
 * Output: one line per logged value and result.  Errors go to stdout too
 * (prefixed `!! ') so that they appear in the compared output, and the
 * exit status is 1.
 *
 * Musashi (Copyright 1998-2001 Karl Stenerud, free for non-commercial use)
 * is not part of this repository; the Makefile builds it from MUSASHI_DIR.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdarg.h>
#include "m68k.h"

/* The checks below rely on these Musashi build options (m68kconf.h in
   MUSASHI_DIR): without them an odd-address access would pass silently. */
#if !M68K_EMULATE_ADDRESS_ERROR
#error "exectest needs M68K_EMULATE_ADDRESS_ERROR = OPT_ON in Musashi's m68kconf.h"
#endif
#if !M68K_EMULATE_010
#error "exectest needs M68K_EMULATE_010 = OPT_ON in Musashi's m68kconf.h"
#endif

#define MEM_SIZE     0x1000000u
#define RET_SENTINEL 0x000E00u
#define VEC_HANDLERS 0x000F00u
#define ARG_BASE     0x060000u
#define ARG_TOP      0x070000u
#define STACK_BASE   0x070000u
#define STACK_TOP    0x07F000u   /* above it: the caller's guard area */
#define GUARD_TOP    0x080000u

static uint8_t *mem;
static uint32_t bin_base, bin_end;
static int trace;
static int failed;
static int in_setup = 1;          /* host writes: no wild-access checks */
static uint32_t wild_addr = 0xffffffffu;
static const char *wild_kind;

static void fail(const char *fmt, ...)
{
    va_list ap;
    printf("!! ");
    va_start(ap, fmt);
    vprintf(fmt, ap);
    va_end(ap);
    printf("\n");
    failed = 1;
}

/* ------------------------------------------------------------------ */
/* Memory                                                               */
/* ------------------------------------------------------------------ */

/* Low memory (the vectors and their handlers) is not valid: a nil or
   small-integer pointer must not read the vector table unnoticed.  The
   CPU's own vector fetch on an exception is flagged too, but do_call
   reports the exception (the PC in VEC_HANDLERS) ahead of it. */
static int valid(uint32_t a)
{
    if (a >= bin_base && a < bin_end) return 1;
    if (a >= ARG_BASE && a < GUARD_TOP) return 1;
    return 0;
}

static uint32_t text_end;         /* --text-end: code is not written */

static void check(uint32_t a, unsigned n, const char *kind)
{
    a &= 0xffffff;
    if (in_setup) return;
    if (!valid(a) || !valid(a + n - 1) ||
        (kind[0] == 'w' && a < text_end && a + n > bin_base)) {
        if (wild_addr == 0xffffffffu) {
            wild_addr = a;
            wild_kind = kind;
        }
    }
}

static uint32_t rd8(uint32_t a)  { return mem[a & 0xffffff]; }
static uint32_t rd16(uint32_t a) { return (rd8(a) << 8) | rd8(a + 1); }
static uint32_t rd32(uint32_t a) { return (rd16(a) << 16) | rd16(a + 2); }
static void wr8(uint32_t a, uint32_t v)  { mem[a & 0xffffff] = (uint8_t)v; }
static void wr16(uint32_t a, uint32_t v) { wr8(a, v >> 8); wr8(a + 1, v); }
static void wr32(uint32_t a, uint32_t v) { wr16(a, v >> 16); wr16(a + 2, v); }

unsigned int m68k_read_memory_8(unsigned int a)  { check(a, 1, "read.b");  return rd8(a); }
static uint32_t counter_addr = 0xffffffffu;   /* EXEC_$COUNTER */
static uint32_t counter;
unsigned int m68k_read_memory_16(unsigned int a)
{
    check(a, 2, "read.w");
    /* a device register: each word read returns the next value */
    if (!in_setup && (a & 0xffffff) == counter_addr) return counter++ & 0xffff;
    return rd16(a);
}
unsigned int m68k_read_memory_32(unsigned int a) { check(a, 4, "read.l");  return rd32(a); }
void m68k_write_memory_8(unsigned int a, unsigned int v)  { check(a, 1, "write.b"); wr8(a, v); }
void m68k_write_memory_16(unsigned int a, unsigned int v)
{
    check(a, 2, "write.w");
    if (!in_setup && (a & 0xffffff) == counter_addr) counter = v & 0xffff;
    wr16(a, v);
}
void m68k_write_memory_32(unsigned int a, unsigned int v) { check(a, 4, "write.l"); wr32(a, v); }
unsigned int m68k_read_disassembler_8(unsigned int a)  { return rd8(a); }
unsigned int m68k_read_disassembler_16(unsigned int a) { return rd16(a); }
unsigned int m68k_read_disassembler_32(unsigned int a) { return rd32(a); }

/* ------------------------------------------------------------------ */
/* Registers                                                            */
/* ------------------------------------------------------------------ */

static uint32_t reg(m68k_register_t r) { return m68k_get_reg(NULL, r); }
static void setreg(m68k_register_t r, uint32_t v) { m68k_set_reg(r, v); }

static uint32_t clobber_seed = 0x5a5a1234u;
static uint32_t garbage(void)
{
    /* xorshift: varied junk so that a value assumed to survive a call is
       caught whatever it is compared with */
    clobber_seed ^= clobber_seed << 13;
    clobber_seed ^= clobber_seed >> 17;
    clobber_seed ^= clobber_seed << 5;
    return clobber_seed | 0x00010001u;   /* never zero in either half */
}

/* ------------------------------------------------------------------ */
/* Stubs                                                                */
/* ------------------------------------------------------------------ */

enum res { R_NONE, R_D0L, R_D0W, R_D0B, R_A0 };

struct stub {
    const char *name;
    uint32_t addr;
    int defined;
};

/* The stub routines tests/exec/exec.ins.pas declares.  Arguments are
   read at 4(SP) (the first argument; pushed last). */
static struct stub stubs[] = {
    { "EXEC_$LOG_L" },    /* (in v: integer32)            by address */
    { "EXEC_$LOG_W" },    /* (in v: integer)              by address */
    { "EXEC_$LOG_UW" },   /* (in v: 0..65535)             by address */
    { "EXEC_$LOG_C" },    /* (in v: 0..255 / char)        by address */
    { "EXEC_$LOG_BOOL" }, /* (in v: boolean)              by address */
    { "EXEC_$LOG_X" },    /* (in v: integer32)            by address, hex */
    { "EXEC_$LOG_MEM" },  /* (in v: univ ...; in n: integer) hex bytes */
    { "EXEC_$LOGV_L" },   /* (v: integer32); val_param */
    { "EXEC_$LOGV_W" },   /* (v: integer); val_param */
    { "EXEC_$LOGV_C" },   /* (v: 0..255); val_param: high byte of slot */
    { "EXEC_$ID_L" },     /* (in v: integer32): integer32 */
    { "EXEC_$ID_W" },     /* (in v: integer): integer, junk in D0's high half */
    { "EXEC_$ID_BOOL" },  /* (in v: boolean): boolean, junk in D0 bits 8-31 */
    { "EXEC_$ID_P" },     /* (in v: univ_ptr): univ_ptr, in A0 */
    { "EXEC_$CLOBBER" },  /* no arguments */
    { "EXEC_$MARK" },     /* (v: integer); val_param: prints `mark N' */
    /* options(nosave) (ref 5.5.8, p. 5-19): these also clobber D2-D7 and
       A2-A4, which such a routine does not preserve (A5 and A6 kept) */
    { "EXEC_$NOSAVE" },   /* no arguments */
    { "EXEC_$NOSAVE_W" }, /* (v: integer); val_param: prints the value */
    { "EXEC_$NOSAVE_ID_L" }, /* (in v: integer32): integer32 */
    { "EXEC_$COUNTER" },  /* data, not a routine: a word "device register";
                             each read.w returns the next value, a write.w
                             sets it */
    { NULL }
};

static void print_bool(uint32_t b)
{
    if (b == 0xff) printf("true\n");
    else if (b == 0) printf("false\n");
    else printf("bool?0x%02x\n", b);
}

/* Run stub `s' with SP pointing at the return address; then return. */
static void run_stub(struct stub *s)
{
    uint32_t sp = reg(M68K_REG_A7);
    uint32_t a1 = rd32(sp + 4);       /* first slot as a longword */
    uint32_t w1 = rd16(sp + 4);       /* first slot as a word */
    const char *n = s->name + 6;      /* after EXEC_$ */
    enum res res = R_NONE;
    uint32_t rv = 0;

    /* A by-address argument must point at memory the program may use,
       and a word or longword at an even address (a real callee's access
       would take an address error): the stubs read in setup mode, so
       check here what the CPU would have. */
    if (strncmp(n, "LOGV_", 5) && strcmp(n, "MARK") && strcmp(n, "CLOBBER") &&
        strcmp(n, "NOSAVE") && strcmp(n, "NOSAVE_W")) {
        unsigned sz = (!strcmp(n, "LOG_C") || !strcmp(n, "LOG_BOOL") ||
                       !strcmp(n, "ID_BOOL") || !strcmp(n, "LOG_MEM")) ? 1 :
                      (!strcmp(n, "LOG_W") || !strcmp(n, "LOG_UW") ||
                       !strcmp(n, "ID_W")) ? 2 : 4;
        if (!valid(a1 & 0xffffff) || !valid((a1 + sz - 1) & 0xffffff)) {
            fail("%s: argument address 0x%08x is wild", s->name, a1);
            return;
        }
        if (sz > 1 && (a1 & 1)) {
            fail("%s: argument address 0x%08x is odd", s->name, a1);
            return;
        }
        if (!strcmp(n, "LOG_MEM")) {
            uint32_t pn = rd32(sp + 8);
            if (!valid(pn & 0xffffff) || (pn & 1)) {
                fail("%s: count address 0x%08x is wild or odd", s->name, pn);
                return;
            }
            uint32_t cnt = rd16(pn);
            if (cnt && !valid((a1 + cnt - 1) & 0xffffff)) {
                fail("%s: %u bytes at 0x%08x run wild", s->name, cnt, a1);
                return;
            }
        }
    }

    in_setup = 1;
    if (!strcmp(n, "LOG_L")) printf("%d\n", (int32_t)rd32(a1));
    else if (!strcmp(n, "LOG_W")) printf("%d\n", (int16_t)rd16(a1));
    else if (!strcmp(n, "LOG_UW")) printf("%u\n", rd16(a1));
    else if (!strcmp(n, "LOG_C")) printf("%u\n", rd8(a1));
    else if (!strcmp(n, "LOG_BOOL")) print_bool(rd8(a1));
    else if (!strcmp(n, "LOG_X")) printf("0x%08x\n", rd32(a1));
    else if (!strcmp(n, "LOG_MEM")) {
        uint32_t cnt = rd16(rd32(sp + 8));
        printf("mem");
        for (uint32_t i = 0; i < cnt && i < 256; i++) printf(" %02x", rd8(a1 + i));
        printf("\n");
    }
    else if (!strcmp(n, "LOGV_L")) printf("%d\n", (int32_t)a1);
    else if (!strcmp(n, "LOGV_W")) printf("%d\n", (int16_t)w1);
    else if (!strcmp(n, "LOGV_C")) printf("%u\n", w1 >> 8);
    else if (!strcmp(n, "MARK")) printf("mark %d\n", (int16_t)w1);
    else if (!strcmp(n, "ID_L")) { res = R_D0L; rv = rd32(a1); }
    else if (!strcmp(n, "ID_W")) { res = R_D0W; rv = rd16(a1); }
    else if (!strcmp(n, "ID_BOOL")) { res = R_D0B; rv = rd8(a1); }
    else if (!strcmp(n, "ID_P")) { res = R_A0; rv = rd32(a1); }
    else if (!strcmp(n, "CLOBBER")) { }
    else if (!strcmp(n, "NOSAVE")) { }
    else if (!strcmp(n, "NOSAVE_W")) printf("%d\n", (int16_t)w1);
    else if (!strcmp(n, "NOSAVE_ID_L")) { res = R_D0L; rv = rd32(a1); }
    else fail("unknown stub %s", s->name);
    in_setup = 0;

    /* The scratch registers a call may change (codegen-patterns 2). */
    setreg(M68K_REG_D0, garbage());
    setreg(M68K_REG_D1, garbage());
    setreg(M68K_REG_A0, garbage());
    setreg(M68K_REG_A1, garbage());
    if (!strncmp(n, "NOSAVE", 6)) {
        static const m68k_register_t nosave[] = {
            M68K_REG_D2, M68K_REG_D3, M68K_REG_D4, M68K_REG_D5, M68K_REG_D6,
            M68K_REG_D7, M68K_REG_A2, M68K_REG_A3, M68K_REG_A4,
        };
        for (unsigned i = 0; i < sizeof nosave / sizeof nosave[0]; i++)
            setreg(nosave[i], garbage());
    }
    switch (res) {
    case R_D0L: setreg(M68K_REG_D0, rv); break;
    case R_D0W: setreg(M68K_REG_D0, (garbage() & 0xffff0000u) | rv); break;
    case R_D0B: setreg(M68K_REG_D0, (garbage() & 0xffffff00u) | rv); break;
    case R_A0:  setreg(M68K_REG_A0, rv); break;
    default: break;
    }
    /* condition codes: junk too (X N Z V C) */
    setreg(M68K_REG_SR, (reg(M68K_REG_SR) & 0xff00u) | (garbage() & 0x1f));
    /* rts */
    setreg(M68K_REG_PC, rd32(sp));
    setreg(M68K_REG_A7, sp + 4);
}

/* ------------------------------------------------------------------ */
/* Calls                                                                */
/* ------------------------------------------------------------------ */

#define MAX_ARGS 16
struct arg {
    int byaddr, out;
    char size;            /* 'l', 'w', 'b' */
    int64_t val;
    uint32_t cell;
};

static uint32_t arg_next = ARG_BASE;

static int parse_arg(const char *t, struct arg *a)
{
    memset(a, 0, sizeof *a);
    if (*t == '@') { a->byaddr = 1; t++; }
    if ((t[0] != 'l' && t[0] != 'w' && t[0] != 'b') || t[1] != ':') return -1;
    a->size = t[0];
    char *end;
    a->val = strtoll(t + 2, &end, 0);
    if (*end == '!') { a->out = 1; end++; }
    if (*end) return -1;
    if (a->out && !a->byaddr) return -1;
    return 0;
}

static const uint32_t canary[16] = {
    0, 0, 0xd2d2d2d2u, 0xd3d3d3d3u, 0xd4d4d4d4u, 0xd5d5d5d5u,
    0xd6d6d6d6u, 0xd7d7d7d7u,
    0, 0, 0x0a2a2a2au, 0x0a3a3a3au, 0x0a4a4a4au, 0x0a5a5a5au,
    0x0a6a6a6au, 0,
};

static long max_steps = 2000000;
static struct stub *stub_at(uint32_t pc)
{
    for (struct stub *s = stubs; s->name; s++)
        if (s->defined && s->addr == pc && s->addr != counter_addr) return s;
    return NULL;
}

static const char *exc_name(unsigned v)
{
    switch (v) {
    case 2: return "bus error";
    case 3: return "address error (word/long access at an odd address)";
    case 4: return "illegal instruction";
    case 5: return "zero divide";
    case 6: return "CHK";
    case 7: return "TRAPV";
    case 8: return "privilege violation";
    case 9: return "trace";
    case 10: return "line 1010 (A-line)";
    case 11: return "line 1111 (F-line)";
    default: return (v >= 32 && v < 48) ? "TRAP" : "exception";
    }
}

/* Run one call; return 0 on success. */
static int do_call(char *spec)
{
    char *toks[MAX_ARGS + 4];
    int nt = 0;
    for (char *t = strtok(spec, " \t"); t && nt < MAX_ARGS + 4; t = strtok(NULL, " \t"))
        toks[nt++] = t;
    if (nt < 1) { fail("empty --call"); return -1; }
    uint32_t entry = (uint32_t)strtoul(toks[0], NULL, 0);
    struct arg args[MAX_ARGS];
    int na = 0;
    const char *rkind = NULL;
    for (int i = 1; i < nt; i++) {
        if (!strcmp(toks[i], "->")) {
            if (i + 1 >= nt) { fail("-> needs a kind"); return -1; }
            rkind = toks[++i];
            continue;
        }
        if (na == MAX_ARGS || parse_arg(toks[i], &args[na]) < 0) {
            fail("bad argument `%s'", toks[i]);
            return -1;
        }
        na++;
    }

    in_setup = 1;
    /* Fresh stack, filled with a pattern; a guard area above it. */
    memset(mem + STACK_BASE, 0xa5, STACK_TOP - STACK_BASE);
    for (uint32_t a = STACK_TOP; a < GUARD_TOP; a += 4) wr32(a, 0x6a6a0000u | (a & 0xffff));

    /* Argument cells */
    for (int i = 0; i < na; i++) {
        struct arg *a = &args[i];
        if (!a->byaddr) continue;
        arg_next = (arg_next + 3) & ~3u;
        if (arg_next + 8 > ARG_TOP) { fail("out of argument cells"); return -1; }
        a->cell = arg_next;
        arg_next += 4;
        if (a->size == 'l') wr32(a->cell, (uint32_t)a->val);
        else if (a->size == 'w') wr16(a->cell, (uint32_t)a->val);
        else wr8(a->cell, (uint32_t)a->val);
    }
    /* Pushes: slot sizes, a pad first when they do not total 4n. */
    uint32_t sp = STACK_TOP;
    unsigned total = 0;
    for (int i = 0; i < na; i++)
        total += (args[i].byaddr || args[i].size == 'l') ? 4 : 2;
    if (total % 4) sp -= 2;
    for (int i = na - 1; i >= 0; i--) {
        struct arg *a = &args[i];
        if (a->byaddr) { sp -= 4; wr32(sp, a->cell); }
        else if (a->size == 'l') { sp -= 4; wr32(sp, (uint32_t)a->val); }
        else if (a->size == 'w') { sp -= 2; wr16(sp, (uint32_t)a->val); }
        else { sp -= 2; wr8(sp, (uint32_t)a->val); }   /* high byte */
    }
    uint32_t sp_args = sp;
    sp -= 4;
    wr32(sp, RET_SENTINEL);

    for (int r = 0; r < 16; r++) {
        if (r == 15) continue;
        uint32_t v = canary[r];
        if (r == 0 || r == 1 || r == 8 || r == 9) v = garbage();
        setreg((m68k_register_t)(M68K_REG_D0 + r), v);
    }
    setreg(M68K_REG_A7, sp);
    setreg(M68K_REG_PC, entry);
    in_setup = 0;
    wild_addr = 0xffffffffu;

    long steps = 0;
    uint32_t last_pc = entry;
    for (;;) {
        uint32_t pc = reg(M68K_REG_PC);
        if (pc == RET_SENTINEL) break;
        if (pc >= VEC_HANDLERS && pc < VEC_HANDLERS + 0x200) {
            unsigned v = (pc - VEC_HANDLERS) / 2;
            fail("%s (vector %u) at pc 0x%06x", exc_name(v), v, last_pc);
            if (v == 3) {
                extern unsigned int m68ki_aerr_address;
                printf("!! access address 0x%06x\n", m68ki_aerr_address);
            }
            return -1;
        }
        struct stub *s = stub_at(pc);
        if (s) {
            run_stub(s);
            if (failed) return -1;
            continue;
        }
        if (pc < bin_base || pc >= bin_end) {
            fail("pc 0x%06x outside the binary (from 0x%06x)", pc, last_pc);
            return -1;
        }
        if (trace) {
            char buf[128];
            in_setup = 1;
            m68k_disassemble(buf, pc, M68K_CPU_TYPE_68010);
            in_setup = 0;
            fprintf(stderr, "%06x  %-34s D0=%08x D1=%08x A0=%08x A1=%08x SP=%06x\n",
                    pc, buf, reg(M68K_REG_D0), reg(M68K_REG_D1), reg(M68K_REG_A0),
                    reg(M68K_REG_A1), reg(M68K_REG_A7));
        }
        last_pc = pc;
        m68k_execute(1);
        /* an exception first: its vector fetch is a "wild" low read */
        pc = reg(M68K_REG_PC);
        if (pc >= VEC_HANDLERS && pc < VEC_HANDLERS + 0x200) continue;
        if (wild_addr != 0xffffffffu) {
            fail("wild %s at 0x%06x (pc 0x%06x)", wild_kind, wild_addr, last_pc);
            return -1;
        }
        if (reg(M68K_REG_A7) < STACK_BASE + 0x100) {
            fail("stack overflow: SP 0x%06x (pc 0x%06x)", reg(M68K_REG_A7), last_pc);
            return -1;
        }
        if (++steps > max_steps) {
            fail("no return after %ld instructions (pc 0x%06x)", max_steps, reg(M68K_REG_PC));
            return -1;
        }
    }

    in_setup = 1;
    int ok = 0;
    for (int r = 2; r < 16; r++) {
        if (r == 8 || r == 9 || r == 15) continue;
        uint32_t v = reg((m68k_register_t)(M68K_REG_D0 + r));
        if (v != canary[r]) {
            fail("%c%d not preserved: 0x%08x, was 0x%08x", r < 8 ? 'D' : 'A', r & 7, v, canary[r]);
            ok = -1;
        }
    }
    if (reg(M68K_REG_A7) != sp_args) {
        fail("SP 0x%06x after return, expected 0x%06x", reg(M68K_REG_A7), sp_args);
        ok = -1;
    }
    for (uint32_t a = STACK_TOP; a < GUARD_TOP; a += 4)
        if (rd32(a) != (0x6a6a0000u | (a & 0xffff))) {
            fail("caller's stack above the arguments written at 0x%06x", a);
            ok = -1;
            break;
        }
    if (reg(M68K_REG_SR) & 0x0700) {
        fail("returned with interrupt mask %u", (reg(M68K_REG_SR) >> 8) & 7);
        ok = -1;
    }

    for (int i = 0; i < na; i++) {
        struct arg *a = &args[i];
        if (!a->out) continue;
        if (a->size == 'l') printf("out %d\n", (int32_t)rd32(a->cell));
        else if (a->size == 'w') printf("out %d\n", (int16_t)rd16(a->cell));
        else printf("out %u\n", rd8(a->cell));
    }
    if (rkind) {
        uint32_t d0 = reg(M68K_REG_D0);
        if (!strcmp(rkind, "l")) printf("=> %d\n", (int32_t)d0);
        else if (!strcmp(rkind, "w")) printf("=> %d\n", (int16_t)d0);
        else if (!strcmp(rkind, "uw")) printf("=> %u\n", d0 & 0xffff);
        else if (!strcmp(rkind, "c")) printf("=> %u\n", d0 & 0xff);
        else if (!strcmp(rkind, "b")) { printf("=> "); print_bool(d0 & 0xff); }
        else if (!strcmp(rkind, "a0")) printf("=> 0x%08x\n", reg(M68K_REG_A0));
        else { fail("bad result kind %s", rkind); ok = -1; }
    }
    in_setup = 0;
    return ok;
}

/* ------------------------------------------------------------------ */

static void usage(void)
{
    fprintf(stderr, "usage: runner [--trace] [--max-steps N] --bin FILE --base ADDR [--end ADDR] "
                    "[--text-end ADDR] [--stub NAME=ADDR]... --call 'ADDR [ARG]... [-> KIND]'...\n");
    exit(2);
}

int main(int argc, char **argv)
{
    const char *bin = NULL;
    char *calls[64];
    int ncalls = 0;
    int have_base = 0;
    uint32_t end_addr = 0;

    mem = calloc(1, MEM_SIZE);
    if (!mem) { perror("calloc"); return 2; }
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--trace")) trace = 1;
        else if (!strcmp(argv[i], "--max-steps") && i + 1 < argc) max_steps = atol(argv[++i]);
        else if (!strcmp(argv[i], "--bin") && i + 1 < argc) bin = argv[++i];
        else if (!strcmp(argv[i], "--text-end") && i + 1 < argc) text_end = (uint32_t)strtoul(argv[++i], NULL, 0);
        else if (!strcmp(argv[i], "--end") && i + 1 < argc) end_addr = (uint32_t)strtoul(argv[++i], NULL, 0);
        else if (!strcmp(argv[i], "--base") && i + 1 < argc) {
            bin_base = (uint32_t)strtoul(argv[++i], NULL, 0);
            have_base = 1;
        }
        else if (!strcmp(argv[i], "--stub") && i + 1 < argc) {
            char *s = argv[++i], *eq = strchr(s, '=');
            if (!eq) usage();
            *eq = 0;
            struct stub *st;
            for (st = stubs; st->name; st++)
                if (!strcmp(st->name, s)) break;
            if (!st->name) { fprintf(stderr, "runner: unknown stub %s\n", s); return 2; }
            st->addr = (uint32_t)strtoul(eq + 1, NULL, 0);
            st->defined = 1;
            if (!strcmp(st->name, "EXEC_$COUNTER")) counter_addr = st->addr;
        }
        else if (!strcmp(argv[i], "--call") && i + 1 < argc && ncalls < 64) calls[ncalls++] = argv[++i];
        else usage();
    }
    if (!bin || !have_base || bin_base < 0x1000 || bin_base >= ARG_BASE) usage();

    FILE *f = fopen(bin, "rb");
    if (!f) { perror(bin); return 2; }
    size_t n = fread(mem + bin_base, 1, ARG_BASE - bin_base, f);
    fclose(f);
    bin_end = bin_base + (uint32_t)n;
    /* bss past the end of the flat image */
    if (end_addr > bin_end) bin_end = end_addr;
    if (bin_end > ARG_BASE) { fprintf(stderr, "runner: image too large\n"); return 2; }

    /* vectors */
    wr32(0, STACK_TOP);
    wr32(4, RET_SENTINEL);
    for (unsigned v = 2; v < 256; v++) {
        wr32(v * 4, VEC_HANDLERS + 2 * v);
        wr16(VEC_HANDLERS + 2 * v, 0x60fe);     /* bra.s * */
    }

    m68k_init();
    m68k_set_cpu_type(M68K_CPU_TYPE_68010);
    m68k_pulse_reset();
    /* supervisor mode, IPL 0 */
    setreg(M68K_REG_SR, 0x2000);

    for (int i = 0; i < ncalls && !failed; i++)
        do_call(calls[i]);
    fflush(stdout);
    return failed ? 1 : 0;
}
