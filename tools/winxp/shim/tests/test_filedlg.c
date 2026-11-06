/*
 * test_filedlg.exe [ours]: checks tools/winxp/shim/xp_filedlg.c (lane WINXP-RECHECK) the way rfd uses the
 * Vista dialog: CoCreateInstance(CLSID_FileOpenDialog / CLSID_FileSaveDialog), SetTitle, SetFileTypes,
 * SetFolder (an item from SHCreateItemFromParsingName), SetFileName, SetDefaultExtension, SetOptions,
 * Show, GetResult / GetResults and the items' GetDisplayName(SIGDN_FILESYSPATH). A second thread finds
 * each dialog by its title and presses OK (or Cancel) in it, as a person would. On plain Windows XP the
 * dialogs are this object's (XP's own GetOpenFileNameW / GetSaveFileNameW / SHBrowseForFolderW); on a
 * newer Windows they are Windows' own unless "ours" forces this object's. Also: a class that is not a
 * dialog goes to Windows' own CoCreateInstance unchanged. kernel32, user32, ole32; no C runtime; PE 5.1.
 * Needs a desktop (run it from the XP test helper, which runs at the user's desktop).
 */
typedef unsigned short wchar16;
typedef unsigned long ULONG;
typedef long HRESULT;
typedef int BOOL;
typedef void *HANDLE;
#define WINAPI __attribute__((stdcall))
typedef struct { unsigned long a; unsigned short b, c; unsigned char d[8]; } GUID;
typedef struct { const wchar16 *name, *spec; } FilterSpec;

HRESULT WINAPI CoCreateInstance(const GUID *, void *, ULONG, const GUID *, void **);
HRESULT WINAPI SHCreateItemFromParsingName(const wchar16 *, void *, const GUID *, void **);
extern int xp_filedlg_force_ours;
__declspec(dllimport) HANDLE WINAPI GetStdHandle(ULONG);
__declspec(dllimport) BOOL WINAPI WriteFile(HANDLE, const void *, ULONG, ULONG *, void *);
__declspec(dllimport) void WINAPI ExitProcess(unsigned);
__declspec(dllimport) wchar16 *WINAPI GetCommandLineW(void);
__declspec(dllimport) ULONG WINAPI GetTempPathW(ULONG, wchar16 *);
__declspec(dllimport) ULONG WINAPI GetLongPathNameW(const wchar16 *, wchar16 *, ULONG);
__declspec(dllimport) BOOL WINAPI CreateDirectoryW(const wchar16 *, void *);
__declspec(dllimport) HANDLE WINAPI CreateFileW(const wchar16 *, ULONG, ULONG, void *, ULONG, ULONG, HANDLE);
__declspec(dllimport) BOOL WINAPI CloseHandle(HANDLE);
__declspec(dllimport) HANDLE WINAPI CreateThread(void *, ULONG, ULONG(WINAPI *)(void *), void *, ULONG, ULONG *);
__declspec(dllimport) ULONG WINAPI WaitForSingleObject(HANDLE, ULONG);
__declspec(dllimport) void WINAPI Sleep(ULONG);
__declspec(dllimport) void *WINAPI GetModuleHandleW(const wchar16 *);
__declspec(dllimport) void *WINAPI LoadLibraryW(const wchar16 *);
__declspec(dllimport) void *WINAPI GetProcAddress(void *, const char *);
__declspec(dllimport) HANDLE WINAPI FindWindowW(const wchar16 *, const wchar16 *);
__declspec(dllimport) BOOL WINAPI PostMessageW(HANDLE, unsigned, unsigned, long);
__declspec(dllimport) long WINAPI SendMessageW(HANDLE, unsigned, unsigned, long);
__declspec(dllimport) HRESULT WINAPI CoInitializeEx(void *, ULONG);
__declspec(dllimport) void WINAPI CoTaskMemFree(void *);
__declspec(dllimport) unsigned WINAPI SetErrorMode(unsigned);

static HANDLE out;
static int failures;
static void say(const char *s) { ULONG n = 0, w; while (s[n]) n++; WriteFile(out, s, n, &w, 0); }
static void sayw(const wchar16 *s) { char b[600]; int i = 0; for (; s && s[i] && i < 598; i++) b[i] = s[i] < 128 ? (char)s[i] : '?'; b[i] = 0; say(b); }
static void hex(unsigned long v) { char b[11]; int i; b[0] = '0'; b[1] = 'x'; for (i = 0; i < 8; i++) b[2 + i] = "0123456789ABCDEF"[(v >> (28 - 4 * i)) & 15]; b[10] = 0; say(b); }
static void res(const char *what, int ok, unsigned long v) { say(ok ? "PASS  " : "FAIL  "); say(what); say("  "); hex(v); say("\r\n"); if (!ok) failures++; }
static int wl(const wchar16 *s) { int n = 0; while (s[n]) n++; return n; }
static void wcat(wchar16 *d, const wchar16 *s) { int n = wl(d), i = 0; while (s[i]) { d[n + i] = s[i]; i++; } d[n + i] = 0; }
static void wcpy(wchar16 *d, const wchar16 *s) { d[0] = 0; wcat(d, s); }
static void wa(wchar16 *d, const char *s) { int i = 0; for (; s[i]; i++) d[i] = (wchar16)s[i]; d[i] = 0; }
static int same_ci(const wchar16 *a, const wchar16 *b) { for (;; a++, b++) { wchar16 x = *a, y = *b; if (x >= 'A' && x <= 'Z') x += 32; if (y >= 'A' && y <= 'Z') y += 32; if (x != y) return 0; if (!x) return 1; } }

static const GUID CLSID_Open = {0xdc1c5a9c, 0xe88a, 0x4dde, {0xa5, 0xa1, 0x60, 0xf8, 0x2a, 0x20, 0xae, 0xf7}};
static const GUID CLSID_Save = {0xc0b4e2f3, 0xba21, 0x4773, {0x8d, 0xba, 0x33, 0x5e, 0xc9, 0x46, 0xeb, 0x8b}};
static const GUID IID_Open = {0xd57c7288, 0xd4ad, 0x4768, {0xbe, 0x02, 0x9d, 0x96, 0x95, 0x32, 0xd9, 0x60}};
static const GUID IID_Save = {0x84bccd23, 0x5fde, 0x4cdb, {0xae, 0xa4, 0xaf, 0x64, 0xb8, 0x3d, 0x78, 0xab}};
static const GUID IID_Item = {0x43826d1e, 0xe718, 0x42ee, {0xbc, 0x55, 0xa1, 0xe2, 0x61, 0xc3, 0x7b, 0xfe}};
static const GUID IID_Unknown = {0, 0, 0, {0xc0, 0, 0, 0, 0, 0, 0, 0x46}};
/* the shell's "shortcut" class: every Windows has it; not a dialog, so Windows' own answers */
static const GUID CLSID_ShellLink = {0x00021401, 0, 0, {0xc0, 0, 0, 0, 0, 0, 0, 0x46}};

#define VT(obj) (*(void ***)(obj))
typedef HRESULT(WINAPI *Fn0)(void *);
typedef HRESULT(WINAPI *FnP)(void *, const void *);
typedef HRESULT(WINAPI *FnU)(void *, ULONG);
typedef HRESULT(WINAPI *FnUP)(void *, ULONG, const void *);
typedef HRESULT(WINAPI *FnPP)(void *, void **);
typedef HRESULT(WINAPI *FnUPP)(void *, ULONG, void **);
/* IFileDialog's slots (Microsoft Learn's order) */
enum { SHOW = 3, SET_TYPES = 4, SET_OPTIONS = 9, SET_FOLDER = 12, SET_NAME = 15, SET_TITLE = 17, GET_RESULT = 20, SET_DEF_EXT = 22, GET_RESULTS = 27 };
enum { ITEM_NAME = 5, ARRAY_COUNT = 7, ARRAY_AT = 8 };
#define SIGDN_FILESYSPATH 0x80058000UL

static wchar16 title[64];
static unsigned press; /* 1 = OK, 2 = Cancel */
static wchar16 typed[128]; /* typed in the file name box first (not empty) */

/* a person at the dialog: waits for the window titled `title`, then presses the button */
static ULONG WINAPI presser(void *unused) {
    int i;
    for (i = 0; i < 400; i++) {
        HANDLE w = FindWindowW(0, title);
        if (w) {
            Sleep(1500);
            /* CDM_SETCONTROLTEXT on the file name box (cmb13), as typing there */
            if (typed[0]) SendMessageW(w, 0x0468, 0x047C, (long)typed);
            PostMessageW(w, 0x0111 /* WM_COMMAND */, press, 0);
            return 0;
        }
        Sleep(100);
    }
    return 1;
}

static wchar16 *path_of(void *item) {
    static wchar16 b[600];
    wchar16 *s = 0;
    b[0] = 0;
    if (((FnUPP)VT(item)[ITEM_NAME])(item, SIGDN_FILESYSPATH, (void **)&s) == 0 && s) { wcpy(b, s); CoTaskMemFree(s); }
    return b;
}

/* one dialog: made, set up, shown with the presser pressing `button`; its HRESULT */
static HRESULT run(int save, const char *name, ULONG options, const wchar16 *folder, const char *file, const char *ext, unsigned button, void **dlg, const char *type) {
    static const wchar16 n1[] = {'T', 'e', 'x', 't', 0}, s1[] = {'*', '.', 't', 'x', 't', 0};
    static const wchar16 n2[] = {'R', 'a', 'p', 'i', 'd', 'R', 0}, s2[] = {'*', '.', 'r', 'r', ';', '*', '.', 'b', 'a', 's', 0};
    FilterSpec specs[2] = {{n1, s1}, {n2, s2}};
    wchar16 w[260];
    void *d = 0, *item = 0;
    HANDLE t;
    HRESULT h = CoCreateInstance(save ? &CLSID_Save : &CLSID_Open, 0, 1 /* CLSCTX_INPROC_SERVER */, save ? &IID_Save : &IID_Open, &d);
    *dlg = 0;
    if (h < 0 || !d) return h;
    wa(title, name);
    ((FnP)VT(d)[SET_TITLE])(d, title);
    if (options) ((FnU)VT(d)[SET_OPTIONS])(d, options);
    if (!(options & 0x20)) ((FnUP)VT(d)[SET_TYPES])(d, 2, specs);
    if (SHCreateItemFromParsingName(folder, 0, &IID_Item, &item) == 0 && item) {
        ((FnP)VT(d)[SET_FOLDER])(d, item);
        ((Fn0)VT(item)[2])(item);
    } else say("      (no item for the folder)\r\n");
    if (file) { wa(w, file); ((FnP)VT(d)[SET_NAME])(d, w); }
    if (ext) { wa(w, ext); ((FnP)VT(d)[SET_DEF_EXT])(d, w); }
    press = button;
    if (type) wa(typed, type); else typed[0] = 0;
    t = CreateThread(0, 0, presser, 0, 0, 0);
    h = ((FnP)VT(d)[SHOW])(d, 0);
    WaitForSingleObject(t, 60000);
    CloseHandle(t);
    *dlg = d;
    return h;
}

static void touch(const wchar16 *dir, const char *name) {
    wchar16 p[300], n[64];
    HANDLE f;
    wcpy(p, dir); wa(n, name); wcat(p, n);
    f = CreateFileW(p, 0x40000000 /* GENERIC_WRITE */, 0, 0, 2 /* CREATE_ALWAYS */, 0x80, 0);
    if (f != (HANDLE)-1) CloseHandle(f);
}

void test_main(void) {
    static const wchar16 SH[] = {'s', 'h', 'e', 'l', 'l', '3', '2', '.', 'd', 'l', 'l', 0};
    wchar16 dir[300], want[300], n[64], *c = GetCommandLineW();
    void *d, *item, *arr;
    HRESULT h;
    out = GetStdHandle((ULONG)-11);
    /* a crash of this test ends it with its exit code, never with Windows' error-report dialog (this
     * process only: SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX; no system setting is changed) */
    SetErrorMode(0x0001 | 0x0002);
    for (; *c; c++) if (c[0] == 'o' && c[1] == 'u' && c[2] == 'r' && c[3] == 's') xp_filedlg_force_ours = 1;
    {
        void *m = GetModuleHandleW(SH);
        if (!m) m = LoadLibraryW(SH);
        say("Vista dialogs on this Windows: ");
        say(m && GetProcAddress(m, "SHCreateItemFromParsingName") ? "YES" : "no (plain XP: the dialogs are ours)");
        say(xp_filedlg_force_ours ? "; ours forced\r\n" : "\r\n");
    }
    CoInitializeEx(0, 2 /* COINIT_APARTMENTTHREADED, as rfd */);
    GetTempPathW(260, want);
    wa(n, "rapidr-filedlg-test\\"); wcat(want, n);
    CreateDirectoryW(want, 0);
    /* (the long form: the dialogs answer with long names, the temp folder is given in 8.3 ones) */
    if (!GetLongPathNameW(want, dir, 300)) wcpy(dir, want);
    touch(dir, "pick.txt");
    touch(dir, "two.txt");

    /* 1. Open: a file named in the folder, OK */
    h = run(0, "RapidR XP open test", 0, dir, "pick.txt", "txt", 1, &d, 0);
    res("Open: Show answers OK", h == 0, (unsigned long)h);
    if (d) {
        item = 0;
        h = ((FnPP)VT(d)[GET_RESULT])(d, &item);
        wcpy(want, dir); wa(n, "pick.txt"); wcat(want, n);
        if (item) { say("      picked: "); sayw(path_of(item)); say("\r\n"); }
        res("Open: GetResult is the folder's pick.txt", h == 0 && item && same_ci(path_of(item), want), (unsigned long)h);
        if (item) ((Fn0)VT(item)[2])(item);
        ((Fn0)VT(d)[2])(d);
    }

    /* 2. Open, Cancel: the Vista dialog's "cancelled" */
    h = run(0, "RapidR XP cancel test", 0, dir, "pick.txt", 0, 2, &d, 0);
    res("Cancel: Show answers ERROR_CANCELLED", h == (HRESULT)0x800704C7L, (unsigned long)h);
    if (d) ((Fn0)VT(d)[2])(d);

    /* 3. Save: a new name with no extension gets the chosen file type's ("Text": .txt), as the Vista
     * dialog does (rfd's default extension is the first type's first one, so they agree) */
    h = run(1, "RapidR XP save test", 0, dir, "newfile", "txt", 1, &d, 0);
    res("Save: Show answers OK", h == 0, (unsigned long)h);
    if (d) {
        item = 0;
        h = ((FnPP)VT(d)[GET_RESULT])(d, &item);
        wcpy(want, dir); wa(n, "newfile.txt"); wcat(want, n);
        if (item) { say("      saved as: "); sayw(path_of(item)); say("\r\n"); }
        res("Save: GetResult is newfile.txt in the folder", h == 0 && item && same_ci(path_of(item), want), (unsigned long)h);
        if (item) ((Fn0)VT(item)[2])(item);
        ((Fn0)VT(d)[2])(d);
    }

    /* 4. Open several (FOS_ALLOWMULTISELECT): a starting name XP cannot take (quotes: shown empty, as the
     * Vista dialog would show it), then two names typed in the box */
    h = run(0, "RapidR XP multi test", 0x200, dir, "\"pick.txt\" \"two.txt\"", 0, 1, &d, "\"pick.txt\" \"two.txt\"");
    res("Multi: Show answers OK", h == 0, (unsigned long)h);
    if (d) {
        ULONG count = 0;
        arr = 0;
        h = ((FnPP)VT(d)[GET_RESULTS])(d, &arr);
        if (arr) ((FnPP)VT(arr)[ARRAY_COUNT])(arr, (void **)&count);
        res("Multi: GetResults has 2 items", h == 0 && count == 2, count);
        if (arr && count == 2) {
            wchar16 a[300], b[300];
            item = 0; ((FnUPP)VT(arr)[ARRAY_AT])(arr, 0, &item); wcpy(a, item ? path_of(item) : n); if (item) ((Fn0)VT(item)[2])(item);
            item = 0; ((FnUPP)VT(arr)[ARRAY_AT])(arr, 1, &item); wcpy(b, item ? path_of(item) : n); if (item) ((Fn0)VT(item)[2])(item);
            say("      items: "); sayw(a); say(" | "); sayw(b); say("\r\n");
            wcpy(want, dir); wa(n, "two.txt"); wcat(want, n);
            res("Multi: one of them is two.txt", same_ci(a, want) || same_ci(b, want), 0);
        }
        if (arr) ((Fn0)VT(arr)[2])(arr);
        ((Fn0)VT(d)[2])(d);
    }

    /* 5. A folder (FOS_PICKFOLDERS), starting in the test folder, OK */
    h = run(0, "RapidR XP folder test", 0x20 | 0x40, dir, 0, 0, 1, &d, 0);
    res("Folder: Show answers OK", h == 0, (unsigned long)h);
    if (d) {
        item = 0;
        h = ((FnPP)VT(d)[GET_RESULT])(d, &item);
        wcpy(want, dir); want[wl(want) - 1] = 0; /* (no trailing \) */
        if (item) { say("      folder: "); sayw(path_of(item)); say("\r\n"); }
        res("Folder: GetResult is the test folder", h == 0 && item && same_ci(path_of(item), want), (unsigned long)h);
        if (item) ((Fn0)VT(item)[2])(item);
        ((Fn0)VT(d)[2])(d);
    }

    /* 6. Not a dialog: Windows' own CoCreateInstance answers, unchanged */
    d = 0;
    h = CoCreateInstance(&CLSID_ShellLink, 0, 1, &IID_Unknown, &d);
    res("Another class (ShellLink) is Windows' own", h == 0 && d, (unsigned long)h);
    if (d) ((Fn0)VT(d)[2])(d);

    say(failures ? "RESULT: FAILED\r\n" : "RESULT: all passed\r\n");
    ExitProcess(failures ? 1 : 0);
}
