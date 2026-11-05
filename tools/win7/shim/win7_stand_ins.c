/*
 * The six functions Windows 7 does not have, and the cube roots msvcrt.dll does not have (below), for the
 * runner programs (rapidrintr-runner[w].exe).
 *
 * rapidr.exe has these in crates/rapidr-cli/src/win7.rs (see tools/win7/clean-room.md for how and
 * why). The runner stubs that `rapidr build --interp` copies into a person's program are built from
 * another crate (interpreter/rapidr-runner-stub) that does not have that module, so without this file
 * the same six functions end up in their import tables and Windows 7 refuses to start every program
 * the Build command makes ("api-ms-win-core-winrt-l1-1-0.dll is missing"). tools/win7/runners.sh
 * compiles this file for the target and hands the object to the linker of the two runner programs
 * (and of nothing else: rapidr.exe has win7.rs, and two definitions would clash).
 *
 * It does what win7.rs does, for the same six functions, and nothing else:
 *   - a function of ours for each, with the signature of the real one;
 *   - the pointer slot `__imp_<Function>` that the program's calls go through (the `windows` crates
 *     are built with `--cfg windows_raw_dylib`, tools/win7/build.sh), defined here, so the linker never
 *     takes the import library's entry and the DLL never appears in the import table;
 *   - on the first call, the real function is looked up in the DLLs that can have it (GetModuleHandleW,
 *     LoadLibraryW from Windows' own folder, GetProcAddress) and called: on Windows 8 and later it is
 *     found and nothing changes. On Windows 7 the function answers "not available" in its own kind
 *     of answer (an error code every caller treats as an ordinary error).
 *
 * Written from Microsoft's public documentation of GetModuleHandleW, LoadLibraryW, GetProcAddress,
 * GetSystemDirectoryW and of the six functions (their signatures), and from RapidR's own win7.rs. No
 * code of any delay-load helper, of mingw-w64 or of anyone else was read or copied. It includes no
 * header: the four kernel32 functions it calls are declared below.
 */

typedef unsigned short wchar16;
typedef unsigned int size_t_;

/*
 * `cbrt` and `cbrtf` too: msvcrt.dll, the C runtime Windows 7 has, has no cube root, so the linker would
 * take mingw-w64's, which are derived from Cephes (a permission to copy that is vague; the project links no
 * Cephes-derived code, docs/licensing.md 3.3, docs/legal/review-2026-10-23-win7-alpha.md item 1).
 * rapidr.exe defines its own in crates/rapidr-cli/src/cbrt.rs; the runner stub does not, and
 * tools/win7/link_audit.sh ... runner found the two Cephes members in its link. These are the same method
 * (a first guess from the exponent bits, then Newton's iteration y = (2y + a / y^2) / 3) written for C:
 * an object's own definition comes before an archive member, so the library's never gets in.
 */
double cbrt(double x);
float cbrtf(float x);

double cbrt(double x)
{
    union { double d; unsigned long long u; } v;
    double a, y, r;
    int i;
    if (x == 0.0 || x != x || x - x != 0.0)
        return x;   /* +-0, NaN, +-infinity: themselves */
    a = x < 0.0 ? -x : x;
    if (a < 2.2250738585072014e-308)   /* subnormal: scaled by 2^54 into the normal range, the root by 2^18 */
        return cbrt(x * 18014398509481984.0) / 262144.0;
    v.d = a;
    v.u = v.u / 3 + 0x2A9F7893782DA1CEull;   /* exponent / 3 in the exponent bits: within a few percent of the root */
    y = v.d;
    for (i = 0; i < 5; i++)
        y = (2.0 * y + a / (y * y)) / 3.0;
    if (y < 4.0e15) {   /* (a whole number's cube comes out as that whole number; above this y is whole already) */
        r = (double)(long long)(y + 0.5);
        if (r * r * r == a)
            y = r;
    }
    return x < 0.0 ? -y : y;
}

float cbrtf(float x)
{
    return (float)cbrt((double)x);
}

__declspec(dllimport) void *__stdcall GetModuleHandleW(const wchar16 *name);
__declspec(dllimport) void *__stdcall LoadLibraryW(const wchar16 *name);
__declspec(dllimport) void *__stdcall GetProcAddress(void *module, const char *name);
__declspec(dllimport) unsigned __stdcall GetSystemDirectoryW(wchar16 *buffer, unsigned size);

#define E_NOTIMPL_ ((long)0x80004001)
#define REGDB_E_CLASSNOTREG_ ((long)0x80040154)

/* One function that may not be there: the DLLs to look in, in order (the list ends with 0), and what was found. */
struct import {
    const char *name;
    const wchar16 *dlls[3];
    void *volatile address;   /* 0 = not asked yet, ABSENT = asked and Windows does not have it */
};
#define ABSENT ((void *)1)

/* `name` in the DLL called `dll`: in the copy that is loaded already, else in Windows' own folder
 * (never a same-named file next to the program). */
static void *find(const wchar16 *dll, const char *name)
{
    wchar16 path[320];
    void *module = GetModuleHandleW(dll);
    if (!module) {
        unsigned n = GetSystemDirectoryW(path, 260), i = 0;
        if (n == 0 || n > 260)
            return 0;
        path[n++] = (wchar16)'\\';
        while (dll[i] && n < 319)
            path[n++] = dll[i++];
        path[n] = 0;
        module = LoadLibraryW(path);
    }
    return module ? GetProcAddress(module, name) : 0;
}

/* The real function, or 0 on a Windows that does not have it. (Two threads asking at once find the same thing.) */
static void *real(struct import *imp)
{
    void *p = imp->address;
    if (!p) {
        int i;
        p = ABSENT;
        for (i = 0; i < 2 && imp->dlls[i]; i++) {
            void *found = find(imp->dlls[i], imp->name);
            if (found) {
                p = found;
                break;
            }
        }
        imp->address = p;
    }
    return p == ABSENT ? 0 : p;
}

/* The stand-ins and their slots. */
#define SLOT(symbol, shim) \
    extern void *slot_##shim __asm__("__imp_" symbol); \
    void *slot_##shim __asm__("__imp_" symbol) = (void *)shim

static struct import i_ro_get_activation_factory = {"RoGetActivationFactory", {L"combase.dll", 0, 0}, 0};
static long __stdcall ro_get_activation_factory(void *class_id, const void *iid, void **factory)
{
    void *p = real(&i_ro_get_activation_factory);
    if (p)
        return ((long(__stdcall *)(void *, const void *, void **))p)(class_id, iid, factory);
    if (factory)
        *factory = 0;
    return REGDB_E_CLASSNOTREG_;   /* no WinRT here */
}
SLOT("RoGetActivationFactory", ro_get_activation_factory);

static struct import i_ro_originate_error = {"RoOriginateErrorW", {L"combase.dll", 0, 0}, 0};
static int __stdcall ro_originate_error(long error, unsigned max, const wchar16 *message)
{
    void *p = real(&i_ro_originate_error);
    if (p)
        return ((int(__stdcall *)(long, unsigned, const wchar16 *))p)(error, max, message);
    return 0;   /* BOOL FALSE: no WinRT error object was made (the caller still has its own error) */
}
SLOT("RoOriginateErrorW", ro_originate_error);

static struct import i_path_cch_strip_prefix = {"PathCchStripPrefix", {L"kernelbase.dll", L"api-ms-win-core-path-l1-1-0.dll", 0}, 0};
static long __stdcall path_cch_strip_prefix(wchar16 *path, size_t_ len)
{
    void *p = real(&i_path_cch_strip_prefix);
    if (p)
        return ((long(__stdcall *)(wchar16 *, size_t_))p)(path, len);
    return E_NOTIMPL_;
}
SLOT("PathCchStripPrefix", path_cch_strip_prefix);

static struct import i_co_increment_mta_usage = {"CoIncrementMTAUsage", {L"ole32.dll", L"combase.dll", 0}, 0};
static long __stdcall co_increment_mta_usage(void **cookie)
{
    void *p = real(&i_co_increment_mta_usage);
    if (p)
        return ((long(__stdcall *)(void **))p)(cookie);
    return E_NOTIMPL_;
}
SLOT("CoIncrementMTAUsage", co_increment_mta_usage);

static struct import i_co_create_free_threaded_marshaler = {"CoCreateFreeThreadedMarshaler", {L"ole32.dll", L"combase.dll", 0}, 0};
static long __stdcall co_create_free_threaded_marshaler(void *outer, void **marshaler)
{
    void *p = real(&i_co_create_free_threaded_marshaler);
    if (p)
        return ((long(__stdcall *)(void *, void **))p)(outer, marshaler);
    return E_NOTIMPL_;
}
SLOT("CoCreateFreeThreadedMarshaler", co_create_free_threaded_marshaler);

static struct import i_co_task_mem_free = {"CoTaskMemFree", {L"ole32.dll", L"combase.dll", 0}, 0};
static void __stdcall co_task_mem_free(void *memory)
{
    void *p = real(&i_co_task_mem_free);
    if (p)
        ((void(__stdcall *)(void *))p)(memory);
}
SLOT("CoTaskMemFree", co_task_mem_free);
