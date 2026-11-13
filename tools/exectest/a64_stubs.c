/* tools/exectest/a64_stubs.c - the execution tests' stub routines
   (tests/exec/exec.ins.pas) for aarch64 (tools/exectest_a64.py): native C
   under Apple's arm64 calling convention, printing what
   tools/exectest/runner.c prints for the m68k. */

#include <stdint.h>
#include <stdio.h>

void dpc_print_bool(unsigned b)
{
    if (b == 0xff) printf("true\n");
    else if (b == 0) printf("false\n");
    else printf("bool?0x%02x\n", b);
}

void EXEC_$LOG_L(const int32_t *v) { printf("%d\n", *v); }
void EXEC_$LOG_W(const int16_t *v) { printf("%d\n", *v); }
void EXEC_$LOG_UW(const uint16_t *v) { printf("%u\n", *v); }
void EXEC_$LOG_C(const uint8_t *v) { printf("%u\n", *v); }
void EXEC_$LOG_BOOL(const uint8_t *v) { dpc_print_bool(*v); }
void EXEC_$LOG_X(const uint32_t *v) { printf("0x%08x\n", *v); }

void EXEC_$LOG_MEM(const uint8_t *v, const uint16_t *n)
{
    printf("mem");
    for (unsigned i = 0; i < *n && i < 256; i++) printf(" %02x", v[i]);
    printf("\n");
}

/* By value: the caller extends a value of less than 32 bits to 32. */
void EXEC_$LOGV_L(int32_t v) { printf("%d\n", v); }
void EXEC_$LOGV_W(int32_t v) { printf("%d\n", (int16_t)v); }
void EXEC_$LOGV_C(int32_t v) { printf("%u\n", v & 0xff); }
void EXEC_$MARK(int32_t v) { printf("mark %d\n", (int16_t)v); }

/* Junk in the bits above a result smaller than 32 bits, as the m68k
   runner leaves it. */
int32_t EXEC_$ID_L(const int32_t *v) { return *v; }
int32_t EXEC_$ID_W(const int16_t *v) { return (int32_t)(0x5a5a0000u | (uint16_t)*v); }
int32_t EXEC_$ID_BOOL(const uint8_t *v) { return (int32_t)(0x5a5a5a00u | *v); }
const void *EXEC_$ID_P(const void *const *v) { return *v; }

/* Clobber the scratch registers a call may change (x0-x17). */
void EXEC_$CLOBBER(void)
{
    __asm__ volatile(
        "mov x1, #0x5a5a\n\tmov x2, #0x5a5a\n\tmov x3, #0x5a5a\n\tmov x4, #0x5a5a\n\t"
        "mov x5, #0x5a5a\n\tmov x6, #0x5a5a\n\tmov x7, #0x5a5a\n\tmov x8, #0x5a5a\n\t"
        "mov x9, #0x5a5a\n\tmov x10, #0x5a5a\n\tmov x11, #0x5a5a\n\tmov x12, #0x5a5a\n\t"
        "mov x13, #0x5a5a\n\tmov x14, #0x5a5a\n\tmov x15, #0x5a5a\n\tmov x16, #0x5a5a\n\t"
        "mov x17, #0x5a5a"
        ::: "x1", "x2", "x3", "x4", "x5", "x6", "x7", "x8", "x9", "x10", "x11", "x12",
            "x13", "x14", "x15", "x16", "x17");
}

/* options(nosave) (ref 5.5.8): on aarch64 these are ordinary C
   routines, which preserve x19-x28. */
void EXEC_$NOSAVE(void) {}
void EXEC_$NOSAVE_W(int32_t v) { printf("%d\n", (int16_t)v); }
int32_t EXEC_$NOSAVE_ID_L(const int32_t *v) { return *v; }

/* A device register on the m68k (the runner counts reads); a plain
   variable here: tests that rely on the counting are skipped
   (tests/exec/aarch64.txt). */
int16_t EXEC_$COUNTER;
