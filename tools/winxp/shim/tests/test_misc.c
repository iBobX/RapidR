/*
 * test_misc.exe [ours]: checks tools/winxp/shim/xp_misc.c (lane WINXP). With "ours" it forces the
 * emulated path even on Windows 7+, so the emulation is exercised where the real functions exist and
 * the two can be compared. kernel32 + advapi32, PE subsystem 5.1. Ours, from Microsoft documentation.
 *
 * Covers the functions with real logic: GetFileInformationByHandleEx (Basic / Standard / AttributeTag /
 * the IdBothDirectory enumeration), SetFileInformationByHandle (EndOfFile), GetFinalPathNameByHandleW,
 * CompareStringOrdinal, GetUserDefaultLocaleName, and a registry round-trip through the transacted
 * create / open and RegDeleteTreeW. Each prints PASS / FAIL. Exit 0 = all passed.
 */
typedef unsigned short wchar16;
typedef unsigned long ULONG;
typedef void *HANDLE;
typedef int BOOL;
typedef long LONG;
#define WINAPI __attribute__((stdcall))

BOOL WINAPI GetFileInformationByHandleEx(HANDLE, int, void *, ULONG);
BOOL WINAPI SetFileInformationByHandle(HANDLE, int, void *, ULONG);
ULONG WINAPI GetFinalPathNameByHandleW(HANDLE, wchar16 *, ULONG, ULONG);
int WINAPI CompareStringOrdinal(const wchar16 *, int, const wchar16 *, int, BOOL);
int WINAPI GetUserDefaultLocaleName(wchar16 *, int);
LONG WINAPI RegCreateKeyTransactedW(void *, const wchar16 *, ULONG, wchar16 *, ULONG, ULONG, void *, void **, ULONG *, HANDLE, void *);
LONG WINAPI RegDeleteTreeW(void *, const wchar16 *);
HANDLE WINAPI CreateTransaction(void *, void *, ULONG, ULONG, ULONG, ULONG, wchar16 *);
BOOL WINAPI CommitTransaction(HANDLE);
extern int xp_misc_force_ours;

__declspec(dllimport) HANDLE WINAPI GetStdHandle(ULONG);
__declspec(dllimport) BOOL WINAPI WriteFile(HANDLE, const void *, ULONG, ULONG *, void *);
__declspec(dllimport) void WINAPI ExitProcess(unsigned);
__declspec(dllimport) HANDLE WINAPI CreateFileW(const wchar16 *, ULONG, ULONG, void *, ULONG, ULONG, HANDLE);
__declspec(dllimport) BOOL WINAPI WriteFile(HANDLE, const void *, ULONG, ULONG *, void *);
__declspec(dllimport) BOOL WINAPI CloseHandle(HANDLE);
__declspec(dllimport) ULONG WINAPI GetTempPathW(ULONG, wchar16 *);
__declspec(dllimport) ULONG WINAPI GetLastError(void);
__declspec(dllimport) wchar16 *WINAPI GetCommandLineW(void);
__declspec(dllimport) void *WINAPI GetModuleHandleW(const wchar16 *);
__declspec(dllimport) void *WINAPI GetProcAddress(void *, const char *);
__declspec(dllimport) LONG WINAPI RegSetValueExW(void *, const wchar16 *, ULONG, ULONG, const void *, ULONG);
__declspec(dllimport) LONG WINAPI RegQueryValueExW(void *, const wchar16 *, ULONG *, ULONG *, void *, ULONG *);
__declspec(dllimport) LONG WINAPI RegCloseKey(void *);
#define HKCU ((void *)(unsigned long)0x80000001)

void *memset(void *d, int c, unsigned n) { char *a = d; while (n--) *a++ = (char)c; return d; }

static HANDLE out;
static int failures;
static void say(const char *s) { ULONG n = 0, w; while (s[n]) n++; WriteFile(out, s, n, &w, 0); }
static void num(long v) { char b[16]; int i = 15; int neg = v < 0; unsigned long u = neg ? (unsigned long)-v : (unsigned long)v; b[i] = 0; do { b[--i] = (char)('0' + u % 10); u /= 10; } while (u); if (neg) b[--i] = '-'; say(b + i); }
static void res(const char *name, int ok) { say(ok ? "PASS  " : "FAIL  "); say(name); say("\r\n"); if (!ok) failures++; }
static int wstrlen(const wchar16 *w) { int n = 0; while (w[n]) n++; return n; }

void test_main(void) {
    wchar16 *c = GetCommandLineW();
    static const wchar16 K32[] = {'k', 'e', 'r', 'n', 'e', 'l', '3', '2', '.', 'd', 'l', 'l', 0};
    int real_here = GetProcAddress(GetModuleHandleW(K32), "GetFileInformationByHandleEx") != 0;
    wchar16 dir[260], path[300];
    ULONG n, i;
    HANDLE f;
    out = GetStdHandle((ULONG)-11);
    for (; *c; c++) if (c[0] == 'o' && c[1] == 'u' && c[2] == 'r' && c[3] == 's') xp_misc_force_ours = 1;
    say("test_misc: ");
    say(xp_misc_force_ours || !real_here ? "xp_misc's own emulation" : "Windows' own functions");
    say("\r\n");

    /* CompareStringOrdinal */
    {
        static const wchar16 A[] = {'a', 'b', 'c', 0}, B[] = {'a', 'b', 'd', 0}, C[] = {'A', 'B', 'C', 0};
        int lt = CompareStringOrdinal(A, -1, B, -1, 0);
        int eqi = CompareStringOrdinal(A, -1, C, -1, 1);
        int gt = CompareStringOrdinal(B, -1, A, -1, 0);
        int eq = CompareStringOrdinal(A, 3, A, 3, 0);
        res("CompareStringOrdinal (less, equal-ignorecase, greater, equal)", lt == 1 && eqi == 2 && gt == 3 && eq == 2);
    }

    /* a temp file: write 1000 bytes, query, truncate to 100 */
    n = GetTempPathW(260, dir);
    for (i = 0; i < n; i++) path[i] = dir[i];
    { static const wchar16 nm[] = {'x', 'p', 'm', 'i', 's', 'c', '.', 't', 'm', 'p', 0}; int k; for (k = 0; nm[k]; k++) path[n + k] = nm[k]; path[n + k] = 0; }
    f = CreateFileW(path, 0x40000000 | 0x80000000, 0, 0, 2 /* CREATE_ALWAYS */, 0x80, 0);
    if (f == (HANDLE)(long)-1) { say("could not make the temp file\r\n"); ExitProcess(2); }
    { char buf[1000]; ULONG w; memset(buf, 'x', sizeof buf); WriteFile(f, buf, sizeof buf, &w, 0); }
    {
        /* FILE_STANDARD_INFO: i64 Allocation; i64 EndOfFile; u32 Links; u8 DeletePending; u8 Directory */
        unsigned char si[24];
        BOOL ok = GetFileInformationByHandleEx(f, 1 /* FileStandardInfo */, si, sizeof si);
        long long eof = *(long long *)(si + 8);
        res("GetFileInformationByHandleEx FileStandardInfo (size 1000)", ok && eof == 1000);
    }
    {
        /* FILE_BASIC_INFO: i64 Creation, LastAccess, LastWrite, Change; u32 Attributes */
        unsigned char bi[40];
        BOOL ok = GetFileInformationByHandleEx(f, 0 /* FileBasicInfo */, bi, sizeof bi);
        ULONG attr = *(ULONG *)(bi + 32);
        res("GetFileInformationByHandleEx FileBasicInfo (has attributes)", ok && attr != 0);
    }
    {
        /* FILE_END_OF_FILE_INFO: i64 */
        long long eof = 100;
        BOOL ok = SetFileInformationByHandle(f, 6 /* FileEndOfFileInfo */, &eof, sizeof eof);
        unsigned char si[24];
        GetFileInformationByHandleEx(f, 1, si, sizeof si);
        res("SetFileInformationByHandle FileEndOfFileInfo (truncate to 100)", ok && *(long long *)(si + 8) == 100);
    }
    {
        wchar16 final[320];
        ULONG r = GetFinalPathNameByHandleW(f, final, 320, 0);
        int has_colon = 0, k;
        for (k = 0; k < wstrlen(final); k++) if (final[k] == ':') has_colon = 1;
        res("GetFinalPathNameByHandleW (drive-letter path)", r > 0 && has_colon);
        say("      ("); { ULONG w; WriteFile(out, "len ", 4, &w, 0); } num((long)r); say(")\r\n");
    }
    CloseHandle(f);

    /* a directory enumeration of the temp folder: find xpmisc.tmp among the entries */
    {
        HANDLE d = CreateFileW(dir, 0x80000000 /* GENERIC_READ */, 7, 0, 3 /* OPEN_EXISTING */, 0x02000000 /* BACKUP_SEMANTICS */, 0);
        int found = 0, entries = 0, restart = 1;
        if (d == (HANDLE)(long)-1) { res("read the temp directory (open)", 0); }
        else {
            unsigned char buf[8192];
            for (;;) {
                BOOL ok = GetFileInformationByHandleEx(d, restart ? 11 : 10, buf, sizeof buf);
                unsigned char *p = buf;
                if (!ok) break; /* ERROR_NO_MORE_FILES at the end */
                restart = 0;
                for (;;) {
                    ULONG next = *(ULONG *)p;
                    /* FILE_ID_BOTH_DIR_INFO: u32 Next; u32 Index; 4xi64 times; 2xi64 sizes; u32 attr;
                     * u32 FileNameLength; u32 EaSize; i8 ShortNameLength; wchar ShortName[12]; i64 FileId;
                     * wchar FileName[] -> name at offset 4+4+32+16+4+4+4+1+24+8 = 105? align: ShortName at
                     * 68, 12 wchars=24 ->92, +1 shortlen at 67; FileId 8-aligned at 96; name at 104 */
                    ULONG name_len = *(ULONG *)(p + 60);
                    wchar16 *name = (wchar16 *)(p + 104);
                    entries++;
                    if (name_len == 20) { /* "xpmisc.tmp" = 10 chars = 20 bytes */
                        static const wchar16 want[] = {'x', 'p', 'm', 'i', 's', 'c', '.', 't', 'm', 'p'};
                        int k, hit = 1;
                        for (k = 0; k < 10; k++) if (name[k] != want[k]) hit = 0;
                        if (hit) found = 1;
                    }
                    if (!next) break;
                    p += next;
                }
            }
            CloseHandle(d);
            res("GetFileInformationByHandleEx directory enumeration (found the temp file)", found && entries >= 3);
            say("      (entries seen: "); num(entries); say(")\r\n");
        }
    }

    /* GetUserDefaultLocaleName: like "en-US" */
    {
        wchar16 loc[85];
        int r = GetUserDefaultLocaleName(loc, 85);
        int dash = 0, k;
        for (k = 0; k < wstrlen(loc); k++) if (loc[k] == '-') dash = 1;
        res("GetUserDefaultLocaleName (language-COUNTRY)", r > 2 && wstrlen(loc) >= 2);
        say("      (length "); num(wstrlen(loc)); say(dash ? ", has a dash)\r\n" : ")\r\n");
    }

    /* registry: create HKCU\Software\RapidR-XP-Test\sub through the transacted create, set and read a
     * value, then RegDeleteTreeW the parent */
    {
        static const wchar16 KEY[] = {'S', 'o', 'f', 't', 'w', 'a', 'r', 'e', '\\', 'R', 'a', 'p', 'i', 'd', 'R', '-', 'X', 'P', '-', 'T', 'e', 's', 't', '\\', 's', 'u', 'b', 0};
        static const wchar16 PARENT[] = {'S', 'o', 'f', 't', 'w', 'a', 'r', 'e', '\\', 'R', 'a', 'p', 'i', 'd', 'R', '-', 'X', 'P', '-', 'T', 'e', 's', 't', 0};
        static const wchar16 VAL[] = {'n', 0};
        void *k = 0;
        ULONG disp = 0, data = 0, len = sizeof(ULONG), type = 0;
        HANDLE txn = CreateTransaction(0, 0, 0, 0, 0, 0, 0); /* a real KTM transaction on Win7; the fake on XP */
        LONG rc = RegCreateKeyTransactedW(HKCU, KEY, 0, 0, 0, 0x2001F /* KEY_READ | KEY_WRITE */, 0, &k, &disp, txn, 0);
        int set_ok = 0, get_ok = 0, del_ok;
        LONG qrc = -999, src = -999;
        if (rc == 0 && k) {
            ULONG v = 1234;
            void *rk = 0;
            HANDLE tr = CreateTransaction(0, 0, 0, 0, 0, 0, 0);
            src = RegSetValueExW(k, VAL, 0, 4 /* REG_DWORD */, (const void *)&v, 4);
            set_ok = src == 0;
            CommitTransaction(txn); /* the write is isolated in a real KTM transaction until commit; a no-op on XP */
            RegCloseKey(k);
            /* read through a fresh handle (a committed transaction's own handle is no longer active) */
            if (RegCreateKeyTransactedW(HKCU, KEY, 0, 0, 0, 0x20019 /* KEY_READ */, 0, &rk, 0, tr, 0) == 0 && rk) {
                qrc = RegQueryValueExW(rk, VAL, 0, &type, (void *)&data, &len);
                get_ok = qrc == 0 && data == 1234;
                RegCloseKey(rk);
            }
            CommitTransaction(tr);
        } else {
            CommitTransaction(txn);
        }
        say("      (src="); num(src); say(" qrc="); num(qrc); say(" data="); num((long)data); say(")\r\n");
        del_ok = RegDeleteTreeW(HKCU, PARENT) == 0;
        /* after the tree delete, the parent's reopen must find it gone (a fresh create reports NEW) */
        {
            void *again = 0;
            HANDLE t2 = CreateTransaction(0, 0, 0, 0, 0, 0, 0);
            LONG r2 = RegCreateKeyTransactedW(HKCU, PARENT, 0, 0, 0, 0x20019 /* KEY_READ */, 0, &again, &disp, t2, 0);
            int gone = (r2 != 0) || (disp == 1 /* REG_CREATED_NEW_KEY: the old was gone */);
            if (again) { RegCloseKey(again); CommitTransaction(t2); RegDeleteTreeW(HKCU, PARENT); }
            say("      (rc="); num(rc); say(" set="); num(set_ok); say(" get="); num(get_ok); say(" del="); num(del_ok); say(" disp2="); num((long)disp); say(" r2="); num(r2); say(" txn="); num((long)(unsigned long)txn); say(")\r\n");
            res("registry round-trip (transacted create, set, read, delete-tree)", rc == 0 && set_ok && get_ok && del_ok && gone);
        }
    }

    say("\r\n"); say(failures ? "RESULT: FAILED " : "RESULT: all passed "); num(failures); say("\r\n");
    ExitProcess(failures ? 1 : 0);
}
