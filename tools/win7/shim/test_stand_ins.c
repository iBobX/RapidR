/*
 * A check of win7_stand_ins.c: calls the six functions through their __imp_ slots, as a program built
 * with --cfg windows_raw_dylib does, once as Windows finds them and once as a Windows 7 would answer.
 *
 *   tools/win7/shim/test.sh            builds it for i686 (LLVM-MinGW) -> $TMPDIR/stand_ins_test.exe
 *   stand_ins_test.exe                 on Windows 8 or later: the real functions are found and called
 *   stand_ins_test.exe absent          the lookups answer "not there": the stand-ins' own answers
 *
 * Prints "ok" / "FAIL" lines; exit status 0 when every line is ok. (Windows 11 runs both.)
 */
#include "win7_stand_ins.c"

#include <stdio.h>
#include <string.h>

static int failures;
#define CHECK(what, cond) do { int ok_ = (cond); printf("  %s  %s\n", ok_ ? "ok  " : "FAIL", what); if (!ok_) failures++; } while (0)

int main(int argc, char **argv)
{
    int absent = argc > 1 && strcmp(argv[1], "absent") == 0;
    void *out = (void *)0x1234;
    long hr;
    wchar16 path[32] = {'\\', '\\', '?', '\\', 'C', ':', '\\', 'x', 0};

    printf("stand-ins, %s\n", absent ? "as Windows 7 answers (every lookup says: not there)" : "as this Windows has them");
    if (absent) {
        i_ro_get_activation_factory.address = ABSENT;
        i_ro_originate_error.address = ABSENT;
        i_path_cch_strip_prefix.address = ABSENT;
        i_co_increment_mta_usage.address = ABSENT;
        i_co_create_free_threaded_marshaler.address = ABSENT;
        i_co_task_mem_free.address = ABSENT;
    }

    /* the slots are the stand-ins' addresses */
    CHECK("__imp_RoGetActivationFactory holds our function", slot_ro_get_activation_factory == (void *)ro_get_activation_factory);
    CHECK("__imp_CoTaskMemFree holds our function", slot_co_task_mem_free == (void *)co_task_mem_free);

    hr = ((long(__stdcall *)(void *, const void *, void **))slot_ro_get_activation_factory)(0, 0, &out);
    if (absent) CHECK("RoGetActivationFactory: REGDB_E_CLASSNOTREG and a null factory", hr == REGDB_E_CLASSNOTREG_ && out == 0);
    else CHECK("RoGetActivationFactory (null class): an error from the real one, no crash", hr != 0 || out == 0);

    hr = ((int(__stdcall *)(long, unsigned, const wchar16 *))slot_ro_originate_error)(E_NOTIMPL_, 0, 0);
    if (absent) CHECK("RoOriginateErrorW: FALSE", hr == 0);
    else CHECK("RoOriginateErrorW: answers (any BOOL)", hr == 0 || hr == 1);

    hr = ((long(__stdcall *)(wchar16 *, size_t_))slot_path_cch_strip_prefix)(path, 32);
    if (absent) CHECK("PathCchStripPrefix: E_NOTIMPL, the path as it was", hr == E_NOTIMPL_ && path[0] == '\\');
    else CHECK("PathCchStripPrefix: strips \\\\?\\ -> C:\\x", hr == 0 && path[0] == 'C' && path[1] == ':' && path[2] == '\\' && path[3] == 'x' && path[4] == 0);

    out = (void *)0x1234;
    hr = ((long(__stdcall *)(void **))slot_co_increment_mta_usage)(&out);
    if (absent) CHECK("CoIncrementMTAUsage: E_NOTIMPL", hr == E_NOTIMPL_);
    else CHECK("CoIncrementMTAUsage: S_OK and a cookie", hr == 0 && out != (void *)0x1234);

    out = (void *)0x1234;
    hr = ((long(__stdcall *)(void *, void **))slot_co_create_free_threaded_marshaler)(0, &out);
    if (absent) CHECK("CoCreateFreeThreadedMarshaler: E_NOTIMPL", hr == E_NOTIMPL_);
    else CHECK("CoCreateFreeThreadedMarshaler: S_OK and a marshaler", hr == 0 && out != (void *)0x1234 && out != 0);

    ((void(__stdcall *)(void *))slot_co_task_mem_free)(0);
    CHECK("CoTaskMemFree(NULL): returns", 1);

    /* the cube roots (ours: the library's Cephes-derived ones are never linked) */
    {
        double worst = 0.0, v;
        int n, whole = 1;
        for (n = 1; n <= 1000; n++) {
            double d = n;
            if (cbrt(d * d * d) != d || cbrt(-(d * d * d)) != -d) whole = 0;
        }
        CHECK("cbrt: the cubes of 1..1000 come out whole, either sign", whole);
        CHECK("cbrt: 0, -0 (sign kept), infinity, NaN are themselves", cbrt(0.0) == 0.0 && 1.0 / cbrt(-0.0) < 0.0 && cbrt(1.0 / 0.0) == 1.0 / 0.0 && cbrt(0.0 / 0.0) != cbrt(0.0 / 0.0));
        for (v = 5e-324; v < 1e300; v *= 3.1) {
            double r = cbrt(v), back = r * r * r, rel = (back - v) / v;
            if (rel < 0) rel = -rel;
            if (rel > worst) worst = rel;
        }
        CHECK("cbrt: r*r*r is within 1e-15 of x from the smallest subnormal to 1e300", worst < 1e-15);
        CHECK("cbrtf(27) is 3, cbrtf(-8) is -2", cbrtf(27.0f) == 3.0f && cbrtf(-8.0f) == -2.0f);
    }

    printf("%s\n", failures ? "FAILED" : "all ok");
    return failures != 0;
}
