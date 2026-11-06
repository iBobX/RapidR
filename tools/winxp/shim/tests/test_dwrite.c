/*
 * test_dwrite.exe [ours]: checks tools/winxp/shim/xp_dwrite.c (lane WINXP): whether this Windows has a
 * dwrite.dll, then the calls fontique makes: DWriteCreateFactory(IDWriteFactory), QueryInterface for
 * IDWriteFactory2 (must be refused on the empty factory), GetSystemFontCollection, GetFontFamilyCount,
 * FindFamilyName. "ours": the empty factory even where DirectWrite exists. kernel32 only, PE 5.1.
 */
typedef unsigned short wchar16;
typedef unsigned long ULONG;
typedef long HRESULT;
typedef int BOOL;
typedef void *HANDLE;
#define WINAPI __attribute__((stdcall))
typedef struct { unsigned long a; unsigned short b, c; unsigned char d[8]; } GUID;

HRESULT WINAPI DWriteCreateFactory(int type, const GUID *iid, void **out);
extern int xp_dwrite_force_ours;
__declspec(dllimport) HANDLE WINAPI GetStdHandle(ULONG);
__declspec(dllimport) BOOL WINAPI WriteFile(HANDLE, const void *, ULONG, ULONG *, void *);
__declspec(dllimport) void WINAPI ExitProcess(unsigned);
__declspec(dllimport) void *WINAPI LoadLibraryW(const wchar16 *);
__declspec(dllimport) wchar16 *WINAPI GetCommandLineW(void);

static HANDLE out;
static int failures;
static void say(const char *s) { ULONG n = 0, w; while (s[n]) n++; WriteFile(out, s, n, &w, 0); }
static void hex(unsigned long v) { char b[11]; int i; b[0] = '0'; b[1] = 'x'; for (i = 0; i < 8; i++) b[2 + i] = "0123456789ABCDEF"[(v >> (28 - 4 * i)) & 15]; b[10] = 0; say(b); }
static void res(const char *what, int ok, unsigned long v) { say(ok ? "PASS  " : "FAIL  "); say(what); say("  "); hex(v); say("\r\n"); if (!ok) failures++; }

typedef HRESULT(WINAPI *QiFn)(void *, const GUID *, void **);
typedef ULONG(WINAPI *RefFn)(void *);
typedef HRESULT(WINAPI *SysFontsFn)(void *, void **, BOOL);
typedef ULONG(WINAPI *CountFn)(void *);
typedef HRESULT(WINAPI *FindFn)(void *, const wchar16 *, unsigned *, BOOL *);
#define VT(obj) (*(void ***)(obj))

void test_main(void) {
    static const wchar16 DW[] = {'d', 'w', 'r', 'i', 't', 'e', '.', 'd', 'l', 'l', 0};
    static const GUID IID_F = {0xb859ee5a, 0xd838, 0x4b5b, {0xa2, 0xe8, 0x1a, 0xdc, 0x7d, 0x93, 0xdb, 0x48}};
    static const GUID IID_F2 = {0x0439fc60, 0xca44, 0x4994, {0x8d, 0xee, 0x3a, 0x9a, 0xf7, 0xb7, 0x32, 0xec}};
    static const wchar16 ARIAL[] = {'A', 'r', 'i', 'a', 'l', 0};
    wchar16 *c = GetCommandLineW();
    void *f = 0, *f2 = 0, *col = 0;
    HRESULT h;
    out = GetStdHandle((ULONG)-11);
    for (; *c; c++) if (c[0] == 'o' && c[1] == 'u' && c[2] == 'r' && c[3] == 's') xp_dwrite_force_ours = 1;
    say("dwrite.dll on this Windows: ");
    say(LoadLibraryW(DW) ? "YES (Windows' own DirectWrite is used)\r\n" : "no (the empty factory is used)\r\n");
    if (xp_dwrite_force_ours) say("(ours: the empty factory forced)\r\n");
    h = DWriteCreateFactory(0 /* shared */, &IID_F, &f);
    res("DWriteCreateFactory(IDWriteFactory)", h == 0 && f != 0, (unsigned long)h);
    if (!f) ExitProcess(1);
    h = ((QiFn)VT(f)[0])(f, &IID_F2, &f2);
    say("      QueryInterface(IDWriteFactory2): "); hex((unsigned long)h); say(f2 ? " (got one)\r\n" : " (none)\r\n");
    h = ((SysFontsFn)VT(f)[3])(f, &col, 0);
    res("GetSystemFontCollection", h == 0 && col != 0, (unsigned long)h);
    if (col) {
        unsigned idx = 7;
        BOOL exists = 1;
        ULONG n = ((CountFn)VT(col)[3])(col);
        say("      families: "); hex(n); say("\r\n");
        h = ((FindFn)VT(col)[5])(col, ARIAL, &idx, &exists);
        res("FindFamilyName(\"Arial\") answers", h == 0, (unsigned long)h);
        ((RefFn)VT(col)[2])(col);
    }
    ((RefFn)VT(f)[2])(f);
    say(failures ? "RESULT: FAILED\r\n" : "RESULT: all passed\r\n");
    ExitProcess(failures ? 1 : 0);
}
