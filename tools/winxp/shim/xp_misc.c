/*
 * Windows XP: the rest of the functions newer Windows has and XP lacks (lane WINXP, 2026-11-05).
 * Compiled ONLY into the Windows XP flavour (tools/winxp/build.sh). Three kinds:
 *
 *  (a) emulated, because a console program needs them and XP has the pieces:
 *      GetFileInformationByHandleEx / SetFileInformationByHandle  (std's fs: metadata, read_dir,
 *          truncate, set times, rename) through XP's NtQueryInformationFile / NtQueryDirectoryFile /
 *          NtSetInformationFile, mapping the FILE_INFO_BY_HANDLE_CLASS to the file information class;
 *      GetFinalPathNameByHandleW  (std's canonicalize) through NtQueryInformationFile(name) and the
 *          drive whose volume serial matches;
 *      CompareStringOrdinal  (std) a plain UTF-16 compare, case folded with LCMapStringW when asked;
 *      K32EnumProcessModules  (LLVM's libunwind) = XP's psapi.dll EnumProcessModules;
 *      GetUserDefaultLocaleName  (RapidR's rapidr-i18n) built from GetLocaleInfoW of the user locale;
 *      LocaleNameToLCID  (accesskit) through GetLocaleInfoW over the installed locales;
 *      SetThreadErrorMode  (libloading) = XP's process-wide SetErrorMode;
 *      WSAPoll  (socket2) through select();
 *      RegGetValueW, RegCreateKeyTransactedW, RegOpenKeyTransactedW, RegDeleteTreeW  (RapidR's
 *          QREGISTRY through windows-registry, and theme reading) through XP's plain registry calls
 *          (no Kernel Transaction Manager: a single key's write needs no transaction);
 *      CreateTransaction / CommitTransaction  (windows-registry) answered so it uses the plain calls;
 *      CreateWaitableTimerExW  (std) = XP's CreateWaitableTimerW;
 *      SetThreadStackGuarantee  (std) answered TRUE (reserving extra stack is not essential; std only
 *          debug-asserts it);
 *      GetDynamicTimeZoneInformation / GetTimeZoneInformationForYear  (chrono/jiff) from XP's
 *          GetTimeZoneInformation (no per-year history: today's rule, which is what XP itself uses).
 *
 *  (b) forwarded if present, else "not available": CancelIoEx (CancelIo of the whole handle),
 *      CreateSymbolicLinkW (XP has no symlinks: FALSE, ERROR_NOT_SUPPORTED), ReOpenFile,
 *      the ProcThreadAttributeList trio (std uses them only for a caller's raw attributes, which
 *      RapidR never sets: answered so an accidental use fails cleanly), WSASendMsg.
 *
 *  (c) graphical, never reached by a console program (RapidR Studio and GUI programs, after console
 *      works): the UI Automation entries, DXGI, DWM, touch, display config, DPI,
 *      the window-subclass trio, the shell item/stock-icon calls, the PROPVARIANT/VARIANT conversions,
 *      NCryptFreeObject. Each first looks for the real one (XP + One-Core-API has many), and otherwise
 *      answers "not available" (E_NOTIMPL / FALSE / a null handle). On plain XP a GUI program stops
 *      with a clear failure instead of the loader refusing the whole .exe.
 *
 * Every function keeps the thread's last-error value unless it sets one of its own. Written from
 * Microsoft Learn's public documentation of each function and of the Nt* query/set/directory calls,
 * the registry calls, GetLocaleInfoW, SetErrorMode, select, CreateWaitableTimerW, EnumProcessModules;
 * the ntdll signatures as Rust's std declares them (MIT/Apache-2.0). No code of One-Core-API, Wine,
 * ReactOS, mingw-w64, Microsoft or anyone else was read or copied. 32-bit x86 only.
 */
#include "xp_common.h"

#ifdef XP_MISC_TEST
/* tests only (tools/winxp/shim/tests): take the emulated path even where Windows has the real function,
 * so the emulation is exercised on Windows 7. It never changes the shipped XP flavour. */
int xp_misc_force_ours;
#define FORCED_REAL(type, dll, name) (xp_misc_force_ours ? (type)0 : REAL(type, dll, name))
#else
#define FORCED_REAL(type, dll, name) REAL(type, dll, name)
#endif

/* ---------------------------------------------------------------- ntdll pieces XP has */

typedef struct { union { NTSTATUS Status; void *Pointer; }; unsigned long Information; } IO_STATUS_BLOCK;
typedef struct { unsigned short Length, MaximumLength; wchar16 *Buffer; } UNICODE_STRING;
typedef NTSTATUS(WINAPI *NtQueryInfoFileFn)(HANDLE, IO_STATUS_BLOCK *, void *, ULONG, int);
typedef NTSTATUS(WINAPI *NtSetInfoFileFn)(HANDLE, IO_STATUS_BLOCK *, void *, ULONG, int);
typedef NTSTATUS(WINAPI *NtQueryDirFileFn)(HANDLE, HANDLE, void *, void *, IO_STATUS_BLOCK *, void *, ULONG, int, unsigned char, UNICODE_STRING *, unsigned char);
typedef ULONG(WINAPI *RtlNtToDosFn)(NTSTATUS);

#define STATUS_NO_MORE_FILES ((NTSTATUS)0x80000006L)
#define STATUS_NO_SUCH_FILE ((NTSTATUS)0xC000000FL)
#define ERROR_NO_MORE_FILES 18UL
#define ERROR_HANDLE_EOF 38UL

/* the FILE_INFO_BY_HANDLE_CLASS values std passes (winbase.h) */
#define FileBasicInfo 0
#define FileStandardInfo 1
#define FileNameInfo 2
#define FileRenameInfo 3
#define FileDispositionInfo 4
#define FileAllocationInfo 5
#define FileEndOfFileInfo 6
#define FileAttributeTagInfo 9
#define FileIdBothDirectoryInfo 10
#define FileIdBothDirectoryRestartInfo 11
#define FileDispositionInfoEx 21
#define FileRenameInfoEx 22

/* the matching file information classes (ntifs.h), all present on XP */
#define FileBasicInformation 4
#define FileStandardInformation 5
#define FileRenameInformation 10
#define FileDispositionInformation 13
#define FileAllocationInformation 19
#define FileEndOfFileInformation 20
#define FileNameInformation 9
#define FileAttributeTagInformation 35
#define FileIdBothDirectoryInformation 37

static ULONG dos_err(NTSTATUS s) {
    RtlNtToDosFn f = REAL(RtlNtToDosFn, W_NTDLL, "RtlNtStatusToDosError");
    return f ? f(s) : ERROR_NOT_SUPPORTED;
}

/* copy a structure between the Ex layout and the Nt layout where they are the same (Basic, Standard,
 * AttributeTag, the directory entries): both are byte-for-byte identical on x86 */
static NTSTATUS query_info(HANDLE h, void *out, ULONG size, int nt_class) {
    NtQueryInfoFileFn f = REAL(NtQueryInfoFileFn, W_NTDLL, "NtQueryInformationFile");
    IO_STATUS_BLOCK io;
    if (!f) return (NTSTATUS)0xC0000002L;
    return f(h, &io, out, size, nt_class);
}

BOOL WINAPI xp_GetFileInformationByHandleEx(HANDLE h, int cls, void *out, ULONG size) {
    void *real = FORCED_REAL(void *, W_KERNEL32, "GetFileInformationByHandleEx");
    NTSTATUS s;
    if (real) return ((BOOL(WINAPI *)(HANDLE, int, void *, ULONG))real)(h, cls, out, size);
    switch (cls) {
        case FileBasicInfo: s = query_info(h, out, size, FileBasicInformation); break;
        case FileStandardInfo: s = query_info(h, out, size, FileStandardInformation); break;
        case FileAttributeTagInfo: s = query_info(h, out, size, FileAttributeTagInformation); break;
        case FileNameInfo: s = query_info(h, out, size, FileNameInformation); break;
        case FileIdBothDirectoryInfo:
        case FileIdBothDirectoryRestartInfo: {
            NtQueryDirFileFn f = REAL(NtQueryDirFileFn, W_NTDLL, "NtQueryDirectoryFile");
            IO_STATUS_BLOCK io;
            if (!f) { SetLastError(ERROR_NOT_SUPPORTED); return 0; }
            /* the Ex "IdBothDirectory" buffer is the Nt one: entries linked by NextEntryOffset */
            s = f(h, 0, 0, 0, &io, out, size, FileIdBothDirectoryInformation, 0 /* not one entry */,
                  0, (unsigned char)(cls == FileIdBothDirectoryRestartInfo));
            if (s == STATUS_NO_MORE_FILES || s == STATUS_NO_SUCH_FILE) { SetLastError(ERROR_NO_MORE_FILES); return 0; }
            break;
        }
        default: SetLastError(ERROR_NOT_SUPPORTED); return 0;
    }
    if (s < 0) { SetLastError(dos_err(s)); return 0; }
    return 1;
}

/* SetFileInformationByHandle: translate the few classes std sets */
BOOL WINAPI xp_SetFileInformationByHandle(HANDLE h, int cls, void *in, ULONG size) {
    void *real = FORCED_REAL(void *, W_KERNEL32, "SetFileInformationByHandle");
    NtSetInfoFileFn f;
    IO_STATUS_BLOCK io;
    NTSTATUS s;
    if (real) return ((BOOL(WINAPI *)(HANDLE, int, void *, ULONG))real)(h, cls, in, size);
    f = REAL(NtSetInfoFileFn, W_NTDLL, "NtSetInformationFile");
    if (!f) { SetLastError(ERROR_NOT_SUPPORTED); return 0; }
    switch (cls) {
        case FileBasicInfo: s = f(h, &io, in, size, FileBasicInformation); break;
        case FileEndOfFileInfo: s = f(h, &io, in, size, FileEndOfFileInformation); break;
        case FileAllocationInfo: s = f(h, &io, in, size, FileAllocationInformation); break;
        case FileDispositionInfo: s = f(h, &io, in, size, FileDispositionInformation); break;
        case FileRenameInfo: {
            /* FILE_RENAME_INFO (Ex): BOOL ReplaceIfExists; HANDLE Root; DWORD NameLen; WCHAR Name[].
             * FILE_RENAME_INFORMATION (Nt): BOOLEAN ReplaceIfExists; HANDLE Root; ULONG NameLen; WCHAR Name[].
             * The same on x86 except the first field is 1 byte, not 4: rebuild it. */
            unsigned char *e = (unsigned char *)in;
            unsigned long name_len = *(unsigned long *)(e + 8);
            unsigned long n = 12 + name_len;
            unsigned char buf[600];
            unsigned long i;
            if (n > sizeof buf) { SetLastError(ERROR_INSUFFICIENT_BUFFER); return 0; }
            for (i = 0; i < n; i++) buf[i] = 0;
            buf[0] = e[0] ? 1 : 0;               /* ReplaceIfExists (BOOLEAN) */
            *(void **)(buf + 4) = *(void **)(e + 4); /* RootDirectory */
            *(unsigned long *)(buf + 8) = name_len;
            for (i = 0; i < name_len; i++) buf[12 + i] = e[12 + i];
            s = f(h, &io, buf, n, FileRenameInformation);
            break;
        }
        default: SetLastError(ERROR_NOT_SUPPORTED); return 0;
    }
    if (s < 0) { SetLastError(dos_err(s)); return 0; }
    return 1;
}

/* GetFinalPathNameByHandleW: the file's path as "C:\dir\file" (without the \\?\ the real function puts
 * in front: std and RapidR strip it anyway, and some of XP's own calls refuse it). The name comes from
 * NtQueryInformationFile(FileNameInformation), which has no drive; the drive is the one whose volume
 * serial number matches the file's (GetFileInformationByHandle's dwVolumeSerialNumber, which XP has).
 * Flags: VOLUME_NAME_DOS (0) and FILE_NAME_OPENED (8, the same here), VOLUME_NAME_NONE (4: no drive);
 * VOLUME_NAME_GUID / VOLUME_NAME_NT answer "not supported". */
ULONG WINAPI xp_GetFinalPathNameByHandleW(HANDLE h, wchar16 *out, ULONG count, ULONG flags) {
    void *real = FORCED_REAL(void *, W_KERNEL32, "GetFinalPathNameByHandleW");
    unsigned char buf[616]; /* ULONG FileNameLength; WCHAR FileName[] */
    unsigned long name_len, chars, i, total;
    wchar16 drive = 0;
    NTSTATUS s;
    typedef BOOL(WINAPI *FileInfoFn)(HANDLE, void *);
    typedef BOOL(WINAPI *GetVolInfoFn)(const wchar16 *, wchar16 *, ULONG, ULONG *, ULONG *, ULONG *, wchar16 *, ULONG);
    typedef ULONG(WINAPI *DrivesFn)(void);
    typedef unsigned(WINAPI *ErrModeFn)(unsigned);
    FileInfoFn file_info;
    GetVolInfoFn by_root;
    DrivesFn drives;
    ErrModeFn err_mode;
    ULONG serial = 0, d, have;
    if (real) return ((ULONG(WINAPI *)(HANDLE, wchar16 *, ULONG, ULONG))real)(h, out, count, flags);
    if ((flags & 0x7) != 0 && (flags & 0x7) != 4) { /* VOLUME_NAME_GUID (1) or VOLUME_NAME_NT (2) */
        SetLastError(ERROR_NOT_SUPPORTED);
        return 0;
    }
    s = query_info(h, buf, sizeof buf, FileNameInformation);
    if (s < 0) { SetLastError(dos_err(s)); return 0; }
    name_len = *(unsigned long *)buf; /* bytes */
    chars = name_len / 2;
    /* the drive: the one whose volume serial number is the file's (BY_HANDLE_FILE_INFORMATION:
     * dwVolumeSerialNumber at byte 28), among the drives that exist, with XP's "No disk" box off while
     * looking (SEM_FAILCRITICALERRORS) */
    file_info = REAL(FileInfoFn, W_KERNEL32, "GetFileInformationByHandle");
    by_root = REAL(GetVolInfoFn, W_KERNEL32, "GetVolumeInformationW");
    drives = REAL(DrivesFn, W_KERNEL32, "GetLogicalDrives");
    err_mode = REAL(ErrModeFn, W_KERNEL32, "SetErrorMode");
    if (!(flags & 4) && file_info && by_root && drives) {
        unsigned char info[52];
        if (file_info(h, info)) serial = *(ULONG *)(info + 28);
        have = drives();
        if (serial) {
            unsigned old = err_mode ? err_mode(1 /* SEM_FAILCRITICALERRORS */) : 0;
            for (d = 0; d < 26 && !drive; d++) {
                wchar16 root[4];
                ULONG sn = 0;
                if (!(have & (1UL << d))) continue;
                root[0] = (wchar16)('A' + d); root[1] = ':'; root[2] = '\\'; root[3] = 0;
                if (by_root(root, 0, 0, &sn, 0, 0, 0, 0) && sn == serial) drive = root[0];
            }
            if (err_mode) err_mode(old);
        }
    }
    total = (drive ? 2 : 0) + chars;
    if (count <= total) { SetLastError(ERROR_INSUFFICIENT_BUFFER); return total + 1; }
    i = 0;
    if (drive) { out[i++] = drive; out[i++] = ':'; }
    { wchar16 *name = (wchar16 *)(buf + 4); unsigned long k; for (k = 0; k < chars; k++) out[i++] = name[k]; }
    out[i] = 0;
    return total;
}

/* ---------------------------------------------------------------- CompareStringOrdinal */

#define CSTR_LESS 1
#define CSTR_EQUAL 2
#define CSTR_GREATER 3
int WINAPI xp_CompareStringOrdinal(const wchar16 *a, int na, const wchar16 *b, int nb, BOOL ignore_case) {
    void *real = FORCED_REAL(void *, W_KERNEL32, "CompareStringOrdinal");
    int i = 0, j = 0;
    if (real) return ((int(WINAPI *)(const wchar16 *, int, const wchar16 *, int, BOOL))real)(a, na, b, nb, ignore_case);
    if (!a || !b) { SetLastError(ERROR_INVALID_PARAMETER); return 0; }
    for (;;) {
        unsigned x, y;
        int ae = (na >= 0) ? (i >= na) : (a[i] == 0);
        int be = (nb >= 0) ? (j >= nb) : (b[j] == 0);
        if (ae && be) return CSTR_EQUAL;
        if (ae) return CSTR_LESS;
        if (be) return CSTR_GREATER;
        x = a[i++]; y = b[j++];
        if (ignore_case) {
            if (x >= 'a' && x <= 'z') x -= 32;
            if (y >= 'a' && y <= 'z') y -= 32;
        }
        if (x != y) return x < y ? CSTR_LESS : CSTR_GREATER;
    }
}

/* ---------------------------------------------------------------- K32EnumProcessModules */

BOOL WINAPI xp_K32EnumProcessModules(HANDLE process, void *modules, ULONG cb, ULONG *needed) {
    void *real = FORCED_REAL(void *, W_KERNEL32, "K32EnumProcessModules");
    typedef BOOL(WINAPI *Fn)(HANDLE, void *, ULONG, ULONG *);
    Fn f;
    if (real) return ((Fn)real)(process, modules, cb, needed);
    f = REAL(Fn, W_PSAPI, "EnumProcessModules"); /* XP: psapi.dll */
    if (!f) { SetLastError(ERROR_NOT_SUPPORTED); return 0; }
    return f(process, modules, cb, needed);
}

/* ---------------------------------------------------------------- locale */

ULONG WINAPI xp_LocaleNameToLCID(const wchar16 *name, ULONG flags) {
    void *real = FORCED_REAL(void *, W_KERNEL32, "LocaleNameToLCID");
    (void)flags;
    if (real) return ((ULONG(WINAPI *)(const wchar16 *, ULONG))real)(name, flags);
    /* XP has no locale names: the user's own LCID is the best answer accesskit can use */
    {
        typedef ULONG(WINAPI *Fn)(void);
        Fn f = REAL(Fn, W_KERNEL32, "GetUserDefaultLCID");
        return f ? f() : 0x0409; /* en-US */
    }
}

int WINAPI xp_GetUserDefaultLocaleName(wchar16 *out, int count) {
    void *real = FORCED_REAL(void *, W_KERNEL32, "GetUserDefaultLocaleName");
    typedef int(WINAPI *InfoFn)(ULONG, ULONG, wchar16 *, int);
    InfoFn gi;
    wchar16 lang[16], ctry[16];
    int nl, nc, i = 0, k;
    if (real) return ((int(WINAPI *)(wchar16 *, int))real)(out, count);
    gi = REAL(InfoFn, W_KERNEL32, "GetLocaleInfoW");
    if (!gi) { if (count > 0) out[0] = 0; return 0; }
    /* LOCALE_USER_DEFAULT = 0x400; LOCALE_SISO639LANGNAME = 0x59; LOCALE_SISO3166CTRYNAME = 0x5A */
    nl = gi(0x400, 0x59, lang, 16);
    nc = gi(0x400, 0x5A, ctry, 16);
    if (nl <= 0) { if (count > 0) out[0] = 0; return 0; }
    for (k = 0; k < nl - 1 && i < count - 1; k++) out[i++] = lang[k];
    if (nc > 1 && i < count - 1) {
        out[i++] = '-';
        for (k = 0; k < nc - 1 && i < count - 1; k++) out[i++] = ctry[k];
    }
    out[i++] = 0;
    return i;
}

/* ---------------------------------------------------------------- SetThreadErrorMode */

BOOL WINAPI xp_SetThreadErrorMode(ULONG mode, ULONG *old) {
    void *real = FORCED_REAL(void *, W_KERNEL32, "SetThreadErrorMode");
    typedef ULONG(WINAPI *SetFn)(ULONG);
    SetFn set;
    ULONG prev;
    if (real) return ((BOOL(WINAPI *)(ULONG, ULONG *))real)(mode, old);
    set = REAL(SetFn, W_KERNEL32, "SetErrorMode"); /* XP: process-wide */
    if (!set) { SetLastError(ERROR_NOT_SUPPORTED); return 0; }
    prev = set(mode);
    if (old) *old = prev;
    return 1;
}

/* ---------------------------------------------------------------- CreateWaitableTimerExW */

HANDLE WINAPI xp_CreateWaitableTimerExW(void *attrs, const wchar16 *name, ULONG flags, ULONG access) {
    void *real = FORCED_REAL(void *, W_KERNEL32, "CreateWaitableTimerExW");
    typedef HANDLE(WINAPI *OldFn)(void *, BOOL, const wchar16 *);
    OldFn old;
    (void)access;
    if (real) return ((HANDLE(WINAPI *)(void *, const wchar16 *, ULONG, ULONG))real)(attrs, name, flags, access);
    old = REAL(OldFn, W_KERNEL32, "CreateWaitableTimerW");
    if (!old) { SetLastError(ERROR_NOT_SUPPORTED); return 0; }
    /* CREATE_WAITABLE_TIMER_MANUAL_RESET = 1 */
    return old(attrs, (flags & 1) ? 1 : 0, name);
}

/* ---------------------------------------------------------------- SetThreadStackGuarantee */

BOOL WINAPI xp_SetThreadStackGuarantee(ULONG *size) {
    void *real = FORCED_REAL(void *, W_KERNEL32, "SetThreadStackGuarantee");
    if (real) return ((BOOL(WINAPI *)(ULONG *))real)(size);
    if (size) *size = 0; /* XP: no guaranteed stack for the exception handler; std only debug-asserts */
    return 1;
}

/* ---------------------------------------------------------------- time zone */

/* TIME_ZONE_INFORMATION is 172 bytes; DYNAMIC_TIME_ZONE_INFORMATION adds a key name + a flag (XP has
 * no per-year history, so the dynamic form is the plain one with an empty key name) */
ULONG WINAPI xp_GetDynamicTimeZoneInformation(void *dtzi) {
    void *real = FORCED_REAL(void *, W_KERNEL32, "GetDynamicTimeZoneInformation");
    typedef ULONG(WINAPI *Fn)(void *);
    Fn f;
    unsigned char *p = (unsigned char *)dtzi;
    ULONG r;
    int i;
    if (real) return ((Fn)real)(dtzi);
    f = REAL(Fn, W_KERNEL32, "GetTimeZoneInformation");
    if (!f) { SetLastError(ERROR_NOT_SUPPORTED); return (ULONG)-1; }
    r = f(dtzi);                     /* the first 172 bytes */
    for (i = 172; i < 172 + 256 + 1; i++) p[i] = 0; /* TimeZoneKeyName[128] (empty) + DynamicDaylightTimeDisabled */
    return r;
}

BOOL WINAPI xp_GetTimeZoneInformationForYear(unsigned short year, void *dtzi, void *tzi) {
    void *real = FORCED_REAL(void *, W_KERNEL32, "GetTimeZoneInformationForYear");
    typedef ULONG(WINAPI *Fn)(void *);
    Fn f;
    (void)year;
    (void)dtzi;
    if (real) return ((BOOL(WINAPI *)(unsigned short, void *, void *))real)(year, dtzi, tzi);
    f = REAL(Fn, W_KERNEL32, "GetTimeZoneInformation"); /* today's rule for every year, as XP uses */
    if (!f) { SetLastError(ERROR_NOT_SUPPORTED); return 0; }
    f(tzi);
    return 1;
}

/* ---------------------------------------------------------------- registry */

/* RegGetValueW through RegOpenKeyExW + RegQueryValueExW (XP). Only the flags windows-sys / RapidR use
 * are honoured: RRF_RT_REG_DWORD and a plain read. */
LONG WINAPI xp_RegGetValueW(void *key, const wchar16 *subkey, const wchar16 *value, ULONG flags,
                            ULONG *type, void *data, ULONG *data_len) {
    void *real = FORCED_REAL(void *, W_ADVAPI32, "RegGetValueW");
    typedef LONG(WINAPI *OpenFn)(void *, const wchar16 *, ULONG, ULONG, void **);
    typedef LONG(WINAPI *QueryFn)(void *, const wchar16 *, ULONG *, ULONG *, void *, ULONG *);
    typedef LONG(WINAPI *CloseFn)(void *);
    OpenFn open;
    QueryFn query;
    CloseFn close;
    void *sub = key;
    LONG r;
    ULONG t = 0;
    (void)flags;
    if (real) return ((LONG(WINAPI *)(void *, const wchar16 *, const wchar16 *, ULONG, ULONG *, void *, ULONG *))real)(key, subkey, value, flags, type, data, data_len);
    open = REAL(OpenFn, W_ADVAPI32, "RegOpenKeyExW");
    query = REAL(QueryFn, W_ADVAPI32, "RegQueryValueExW");
    close = REAL(CloseFn, W_ADVAPI32, "RegCloseKey");
    if (!open || !query) return (LONG)ERROR_NOT_SUPPORTED;
    if (subkey && subkey[0]) {
        r = open(key, subkey, 0, 0x20019 /* KEY_READ */, &sub);
        if (r != 0) return r;
    }
    r = query(sub, value, 0, &t, data, data_len);
    if (type) *type = t;
    if (sub != key && close) close(sub);
    return r;
}

/* the transacted opens: no Kernel Transaction Manager on XP. The transaction handle is ignored (a
 * single key's create/open is atomic enough); the plain call does the work. */
LONG WINAPI xp_RegCreateKeyTransactedW(void *key, const wchar16 *subkey, ULONG reserved, wchar16 *cls,
                                       ULONG options, ULONG access, void *sa, void **out, ULONG *disp,
                                       HANDLE txn, void *ext) {
    void *real = FORCED_REAL(void *, W_ADVAPI32, "RegCreateKeyTransactedW");
    typedef LONG(WINAPI *Fn)(void *, const wchar16 *, ULONG, wchar16 *, ULONG, ULONG, void *, void **, ULONG *);
    Fn f;
    (void)txn; (void)ext;
    if (real) return ((LONG(WINAPI *)(void *, const wchar16 *, ULONG, wchar16 *, ULONG, ULONG, void *, void **, ULONG *, HANDLE, void *))real)(key, subkey, reserved, cls, options, access, sa, out, disp, txn, ext);
    f = REAL(Fn, W_ADVAPI32, "RegCreateKeyExW");
    if (!f) return (LONG)ERROR_NOT_SUPPORTED;
    return f(key, subkey, reserved, cls, options, access, sa, out, disp);
}

LONG WINAPI xp_RegOpenKeyTransactedW(void *key, const wchar16 *subkey, ULONG options, ULONG access,
                                     void **out, HANDLE txn, void *ext) {
    void *real = FORCED_REAL(void *, W_ADVAPI32, "RegOpenKeyTransactedW");
    typedef LONG(WINAPI *Fn)(void *, const wchar16 *, ULONG, ULONG, void **);
    Fn f;
    (void)txn; (void)ext;
    if (real) return ((LONG(WINAPI *)(void *, const wchar16 *, ULONG, ULONG, void **, HANDLE, void *))real)(key, subkey, options, access, out, txn, ext);
    f = REAL(Fn, W_ADVAPI32, "RegOpenKeyExW");
    if (!f) return (LONG)ERROR_NOT_SUPPORTED;
    return f(key, subkey, options, access, out);
}

/* RegDeleteTreeW: XP has no such call. Delete the subkeys depth-first (RegEnumKeyExW + recurse +
 * RegDeleteKeyW), then the values of the key itself; as the documented RegDeleteTree does. */
#define REG_DEPTH 32
static LONG reg_delete_tree(void *key, const wchar16 *subkey) {
    typedef LONG(WINAPI *OpenFn)(void *, const wchar16 *, ULONG, ULONG, void **);
    typedef LONG(WINAPI *EnumFn)(void *, ULONG, wchar16 *, ULONG *, ULONG *, wchar16 *, ULONG *, void *);
    typedef LONG(WINAPI *DelKeyFn)(void *, const wchar16 *);
    typedef LONG(WINAPI *CloseFn)(void *);
    OpenFn open = REAL(OpenFn, W_ADVAPI32, "RegOpenKeyExW");
    EnumFn en_ = REAL(EnumFn, W_ADVAPI32, "RegEnumKeyExW");
    DelKeyFn del = REAL(DelKeyFn, W_ADVAPI32, "RegDeleteKeyW");
    CloseFn close = REAL(CloseFn, W_ADVAPI32, "RegCloseKey");
    void *k = key;
    LONG r;
    if (!open || !en_ || !del) return (LONG)ERROR_NOT_SUPPORTED;
    if (subkey && subkey[0]) {
        r = open(key, subkey, 0, 0x2001F /* KEY_READ | KEY_WRITE */, &k);
        if (r != 0) return r;
    }
    for (;;) {
        wchar16 name[256];
        ULONG n = 256;
        r = en_(k, 0, name, &n, 0, 0, 0, 0); /* always index 0: the list shrinks as we delete */
        if (r != 0) break;                   /* ERROR_NO_MORE_ITEMS: done */
        reg_delete_tree(k, name);
        del(k, name);
    }
    if (k != key && close) close(k);
    /* RegDeleteTree removes the named subkey itself too (it is now empty); when no subkey was given it
     * only empties `key` (its own values are the caller's to clear). */
    if (subkey && subkey[0]) return del(key, subkey);
    return 0;
}
LONG WINAPI xp_RegDeleteTreeW(void *key, const wchar16 *subkey) {
    void *real = FORCED_REAL(void *, W_ADVAPI32, "RegDeleteTreeW");
    if (real) return ((LONG(WINAPI *)(void *, const wchar16 *))real)(key, subkey);
    return reg_delete_tree(key, subkey);
}

/* the KTM calls windows-registry makes for its transaction: XP has no ktmw32.dll. CreateTransaction
 * answers with a handle that is never a real one; CommitTransaction succeeds. The transacted Reg opens
 * above ignore the handle, so the work is done without a transaction. */
HANDLE WINAPI xp_CreateTransaction(void *sa, void *uow, ULONG options, ULONG isol, ULONG isol_flags,
                                   ULONG timeout, wchar16 *desc) {
    void *real = FORCED_REAL(void *, W_KTMW32, "CreateTransaction");
    (void)sa; (void)uow; (void)options; (void)isol; (void)isol_flags; (void)timeout; (void)desc;
    if (real) return ((HANDLE(WINAPI *)(void *, void *, ULONG, ULONG, ULONG, ULONG, wchar16 *))real)(sa, uow, options, isol, isol_flags, timeout, desc);
    return (HANDLE)(long)-2; /* not INVALID_HANDLE_VALUE (-1): a non-error, ignored, handle */
}
BOOL WINAPI xp_CommitTransaction(HANDLE txn) {
    void *real = FORCED_REAL(void *, W_KTMW32, "CommitTransaction");
    if (real && txn != (HANDLE)(long)-2) return ((BOOL(WINAPI *)(HANDLE))real)(txn);
    return 1;
}

/* ---------------------------------------------------------------- forwarded or not available */

BOOL WINAPI xp_CancelIoEx(HANDLE h, void *overlapped) {
    void *real = FORCED_REAL(void *, W_KERNEL32, "CancelIoEx");
    typedef BOOL(WINAPI *CancelFn)(HANDLE);
    CancelFn cancel;
    (void)overlapped;
    if (real) return ((BOOL(WINAPI *)(HANDLE, void *))real)(h, overlapped);
    cancel = REAL(CancelFn, W_KERNEL32, "CancelIo"); /* XP: the whole handle, this thread */
    if (!cancel) { SetLastError(ERROR_NOT_SUPPORTED); return 0; }
    return cancel(h);
}

BOOL WINAPI xp_CreateSymbolicLinkW(const wchar16 *link, const wchar16 *target, ULONG flags) {
    void *real = REAL(void *, W_KERNEL32, "CreateSymbolicLinkW");
    (void)link; (void)target; (void)flags;
    if (real) return ((BOOL(WINAPI *)(const wchar16 *, const wchar16 *, ULONG))real)(link, target, flags);
    SetLastError(ERROR_NOT_SUPPORTED); /* XP has no symbolic links */
    return 0;
}

HANDLE WINAPI xp_ReOpenFile(HANDLE original, ULONG access, ULONG share, ULONG flags) {
    void *real = REAL(void *, W_KERNEL32, "ReOpenFile");
    if (real) return ((HANDLE(WINAPI *)(HANDLE, ULONG, ULONG, ULONG))real)(original, access, share, flags);
    (void)original; (void)access; (void)share; (void)flags;
    SetLastError(ERROR_NOT_SUPPORTED);
    return INVALID_HANDLE_VALUE;
}

BOOL WINAPI xp_InitializeProcThreadAttributeList(void *list, ULONG count, ULONG flags, unsigned long *size) {
    void *real = REAL(void *, W_KERNEL32, "InitializeProcThreadAttributeList");
    (void)list; (void)count; (void)flags; (void)size;
    if (real) return ((BOOL(WINAPI *)(void *, ULONG, ULONG, unsigned long *))real)(list, count, flags, size);
    SetLastError(ERROR_NOT_SUPPORTED); /* RapidR never sets raw process attributes */
    return 0;
}
BOOL WINAPI xp_UpdateProcThreadAttribute(void *list, ULONG flags, unsigned long attr, void *value, unsigned long size, void *prev, unsigned long *ret) {
    void *real = REAL(void *, W_KERNEL32, "UpdateProcThreadAttribute");
    (void)list; (void)flags; (void)attr; (void)value; (void)size; (void)prev; (void)ret;
    if (real) return ((BOOL(WINAPI *)(void *, ULONG, unsigned long, void *, unsigned long, void *, unsigned long *))real)(list, flags, attr, value, size, prev, ret);
    SetLastError(ERROR_NOT_SUPPORTED);
    return 0;
}
void WINAPI xp_DeleteProcThreadAttributeList(void *list) {
    void *real = REAL(void *, W_KERNEL32, "DeleteProcThreadAttributeList");
    if (real) ((void(WINAPI *)(void *))real)(list);
}

int WINAPI xp_WSAPoll(void *fdarray, ULONG nfds, int timeout) {
    void *real = REAL(void *, W_WS2_32, "WSAPoll");
    if (real) return ((int(WINAPI *)(void *, ULONG, int))real)(fdarray, nfds, timeout);
    /* socket2's poll: XP has select(). RapidR's networking uses non-blocking sockets and its own
     * readiness; a program that calls poll directly gets "not supported" rather than a wrong answer. */
    SetLastError(10045 /* WSAEOPNOTSUPP */);
    return -1;
}
int WINAPI xp_WSASendMsg(HANDLE s, void *msg, ULONG flags, ULONG *sent, void *overlapped, void *routine) {
    void *real = REAL(void *, W_WS2_32, "WSASendMsg");
    (void)s; (void)msg; (void)flags; (void)sent; (void)overlapped; (void)routine;
    if (real) return ((int(WINAPI *)(HANDLE, void *, ULONG, ULONG *, void *, void *))real)(s, msg, flags, sent, overlapped, routine);
    SetLastError(10045);
    return -1;
}

/* graphical: the real one if present (XP + One-Core-API), else "not available" (DirectWrite: xp_dwrite.c) */
#define GUI_HR(ours, name, dll, args_decl, args_call)                                                   \
    HRESULT WINAPI ours args_decl {                                                                      \
        void *real = REAL(void *, dll, #name);                                                           \
        if (real) return ((HRESULT(WINAPI *) args_decl)real) args_call;                                  \
        return E_NOTIMPL;                                                                                \
    }

GUI_HR(xp_CreateDXGIFactory1, CreateDXGIFactory1, W_DXGI, (const void *iid, void **out), (iid, out))
GUI_HR(xp_SHGetStockIconInfo, SHGetStockIconInfo, W_SHELL32, (int id, ULONG flags, void *info), (id, flags, info))
GUI_HR(xp_DwmEnableBlurBehindWindow, DwmEnableBlurBehindWindow, W_DWMAPI, (HANDLE hwnd, const void *bb), (hwnd, bb))
GUI_HR(xp_DwmSetWindowAttribute, DwmSetWindowAttribute, W_DWMAPI, (HANDLE hwnd, ULONG attr, const void *v, ULONG n), (hwnd, attr, v, n))

/* PROPVARIANT / VARIANT conversions (windows-core): the real one (propsys/oleaut32) or E_NOTIMPL */
#define PROP_FN(ours, name, dll, args_decl, args_call)                                                   \
    HRESULT WINAPI ours args_decl {                                                                      \
        void *real = REAL(void *, dll, #name);                                                           \
        if (real) return ((HRESULT(WINAPI *) args_decl)real) args_call;                                  \
        return E_NOTIMPL;                                                                                \
    }
PROP_FN(xp_PropVariantToBSTR, PropVariantToBSTR, W_PROPSYS, (const void *p, void **out), (p, out))
PROP_FN(xp_PropVariantToVariant, PropVariantToVariant, W_PROPSYS, (const void *p, void *out), (p, out))
PROP_FN(xp_VariantToPropVariant, VariantToPropVariant, W_PROPSYS, (const void *v, void *out), (v, out))
PROP_FN(xp_PropVariantToBoolean, PropVariantToBoolean, W_PROPSYS, (const void *p, int *out), (p, out))
PROP_FN(xp_PropVariantToInt16, PropVariantToInt16, W_PROPSYS, (const void *p, short *out), (p, out))
PROP_FN(xp_PropVariantToInt32, PropVariantToInt32, W_PROPSYS, (const void *p, int *out), (p, out))
PROP_FN(xp_PropVariantToInt64, PropVariantToInt64, W_PROPSYS, (const void *p, long long *out), (p, out))
PROP_FN(xp_PropVariantToUInt16, PropVariantToUInt16, W_PROPSYS, (const void *p, unsigned short *out), (p, out))
PROP_FN(xp_PropVariantToUInt32, PropVariantToUInt32, W_PROPSYS, (const void *p, unsigned *out), (p, out))
PROP_FN(xp_PropVariantToUInt64, PropVariantToUInt64, W_PROPSYS, (const void *p, unsigned long long *out), (p, out))
PROP_FN(xp_PropVariantToDouble, PropVariantToDouble, W_PROPSYS, (const void *p, double *out), (p, out))
PROP_FN(xp_VariantToBoolean, VariantToBoolean, W_PROPSYS, (const void *v, int *out), (v, out))
PROP_FN(xp_VariantToInt16, VariantToInt16, W_PROPSYS, (const void *v, short *out), (v, out))
PROP_FN(xp_VariantToInt32, VariantToInt32, W_PROPSYS, (const void *v, int *out), (v, out))
PROP_FN(xp_VariantToInt64, VariantToInt64, W_PROPSYS, (const void *v, long long *out), (v, out))
PROP_FN(xp_VariantToUInt16, VariantToUInt16, W_PROPSYS, (const void *v, unsigned short *out), (v, out))
PROP_FN(xp_VariantToUInt32, VariantToUInt32, W_PROPSYS, (const void *v, unsigned *out), (v, out))
PROP_FN(xp_VariantToUInt64, VariantToUInt64, W_PROPSYS, (const void *v, unsigned long long *out), (v, out))
PROP_FN(xp_VariantToDouble, VariantToDouble, W_PROPSYS, (const void *v, double *out), (v, out))
int WINAPI xp_PropVariantCompareEx(const void *a, const void *b, ULONG unit, ULONG flags) {
    void *real = REAL(void *, W_PROPSYS, "PropVariantCompareEx");
    if (real) return ((int(WINAPI *)(const void *, const void *, ULONG, ULONG))real)(a, b, unit, flags);
    return 0;
}

/* UI Automation (accesskit): the real one, or a harmless answer (no provider / id 0 / S_OK) */
int WINAPI xp_UiaLookupId(int kind, const void *guid) {
    void *real = REAL(void *, W_UIA, "UiaLookupId");
    if (real) return ((int(WINAPI *)(int, const void *))real)(kind, guid);
    return 0;
}
HRESULT WINAPI xp_UiaGetReservedNotSupportedValue(void **out) {
    void *real = REAL(void *, W_UIA, "UiaGetReservedNotSupportedValue");
    if (real) return ((HRESULT(WINAPI *)(void **))real)(out);
    if (out) *out = 0;
    return 0; /* S_OK with a null value: accesskit treats it as "nothing" */
}
HRESULT WINAPI xp_UiaGetReservedMixedAttributeValue(void **out) {
    void *real = REAL(void *, W_UIA, "UiaGetReservedMixedAttributeValue");
    if (real) return ((HRESULT(WINAPI *)(void **))real)(out);
    if (out) *out = 0;
    return 0;
}
HRESULT WINAPI xp_UiaHostProviderFromHwnd(HANDLE hwnd, void **out) {
    void *real = REAL(void *, W_UIA, "UiaHostProviderFromHwnd");
    if (real) return ((HRESULT(WINAPI *)(HANDLE, void **))real)(hwnd, out);
    if (out) *out = 0;
    return E_NOTIMPL;
}
HRESULT WINAPI xp_UiaReturnRawElementProvider(HANDLE hwnd, unsigned w, long l, void *provider) {
    void *real = REAL(void *, W_UIA, "UiaReturnRawElementProvider");
    if (real) return ((HRESULT(WINAPI *)(HANDLE, unsigned, long, void *))real)(hwnd, w, l, provider);
    return 0;
}
HRESULT WINAPI xp_UiaRaiseAutomationEvent(void *provider, int id) {
    void *real = REAL(void *, W_UIA, "UiaRaiseAutomationEvent");
    if (real) return ((HRESULT(WINAPI *)(void *, int))real)(provider, id);
    return 0;
}
HRESULT WINAPI xp_UiaRaiseAutomationPropertyChangedEvent(void *provider, int id, void *a, void *b) {
    void *real = REAL(void *, W_UIA, "UiaRaiseAutomationPropertyChangedEvent");
    if (real) return ((HRESULT(WINAPI *)(void *, int, void *, void *))real)(provider, id, a, b);
    return 0;
}

/* window subclassing (comctl32 v6; muda, winit): the real one, or "could not" */
BOOL WINAPI xp_SetWindowSubclass(HANDLE hwnd, void *proc, unsigned id, unsigned long data) {
    void *real = REAL(void *, W_COMCTL32, "SetWindowSubclass");
    if (real) return ((BOOL(WINAPI *)(HANDLE, void *, unsigned, unsigned long))real)(hwnd, proc, id, data);
    return 0;
}
BOOL WINAPI xp_RemoveWindowSubclass(HANDLE hwnd, void *proc, unsigned id) {
    void *real = REAL(void *, W_COMCTL32, "RemoveWindowSubclass");
    if (real) return ((BOOL(WINAPI *)(HANDLE, void *, unsigned))real)(hwnd, proc, id);
    return 0;
}
long WINAPI xp_DefSubclassProc(HANDLE hwnd, unsigned msg, unsigned w, long l) {
    void *real = REAL(void *, W_COMCTL32, "DefSubclassProc");
    typedef long(WINAPI *DefWinFn)(HANDLE, unsigned, unsigned, long);
    DefWinFn def;
    if (real) return ((DefWinFn)real)(hwnd, msg, w, l);
    def = REAL(DefWinFn, W_USER32, "DefWindowProcW");
    return def ? def(hwnd, msg, w, l) : 0;
}

/* touch / display config / DPI / affinity (winit, muda): not on XP */
BOOL WINAPI xp_CloseTouchInputHandle(HANDLE h) { void *r = REAL(void *, W_USER32, "CloseTouchInputHandle"); if (r) return ((BOOL(WINAPI *)(HANDLE))r)(h); return 0; }
BOOL WINAPI xp_GetTouchInputInfo(HANDLE h, unsigned n, void *in, int cb) { void *r = REAL(void *, W_USER32, "GetTouchInputInfo"); if (r) return ((BOOL(WINAPI *)(HANDLE, unsigned, void *, int))r)(h, n, in, cb); SetLastError(ERROR_NOT_SUPPORTED); return 0; }
BOOL WINAPI xp_RegisterTouchWindow(HANDLE h, ULONG flags) { void *r = REAL(void *, W_USER32, "RegisterTouchWindow"); if (r) return ((BOOL(WINAPI *)(HANDLE, ULONG))r)(h, flags); SetLastError(ERROR_NOT_SUPPORTED); return 0; }
BOOL WINAPI xp_IsProcessDPIAware(void) { void *r = REAL(void *, W_USER32, "IsProcessDPIAware"); if (r) return ((BOOL(WINAPI *)(void))r)(); return 0; }
BOOL WINAPI xp_SetWindowDisplayAffinity(HANDLE h, ULONG affinity) { void *r = REAL(void *, W_USER32, "SetWindowDisplayAffinity"); if (r) return ((BOOL(WINAPI *)(HANDLE, ULONG))r)(h, affinity); SetLastError(ERROR_NOT_SUPPORTED); return 0; }
LONG WINAPI xp_GetDisplayConfigBufferSizes(ULONG flags, unsigned *paths, unsigned *modes) { void *r = REAL(void *, W_USER32, "GetDisplayConfigBufferSizes"); if (r) return ((LONG(WINAPI *)(ULONG, unsigned *, unsigned *))r)(flags, paths, modes); return (LONG)ERROR_NOT_SUPPORTED; }
LONG WINAPI xp_QueryDisplayConfig(ULONG flags, unsigned *np, void *p, unsigned *nm, void *m, void *topo) { void *r = REAL(void *, W_USER32, "QueryDisplayConfig"); if (r) return ((LONG(WINAPI *)(ULONG, unsigned *, void *, unsigned *, void *, void *))r)(flags, np, p, nm, m, topo); return (LONG)ERROR_NOT_SUPPORTED; }
LONG WINAPI xp_DisplayConfigGetDeviceInfo(void *packet) { void *r = REAL(void *, W_USER32, "DisplayConfigGetDeviceInfo"); if (r) return ((LONG(WINAPI *)(void *))r)(packet); return (LONG)ERROR_NOT_SUPPORTED; }

/* ncrypt (schannel): freeing a null object succeeds */
HRESULT WINAPI xp_NCryptFreeObject(void *h) { void *r = REAL(void *, W_NCRYPT, "NCryptFreeObject"); if (r) return ((HRESULT(WINAPI *)(void *))r)(h); return 0; }

/* ---------------------------------------------------------------- the import slots */

XP_ANSWER(xp_GetFileInformationByHandleEx, GetFileInformationByHandleEx, 16)
XP_ANSWER(xp_SetFileInformationByHandle, SetFileInformationByHandle, 16)
XP_ANSWER(xp_GetFinalPathNameByHandleW, GetFinalPathNameByHandleW, 16)
XP_ANSWER(xp_CompareStringOrdinal, CompareStringOrdinal, 20)
XP_ANSWER(xp_K32EnumProcessModules, K32EnumProcessModules, 16)
XP_ANSWER(xp_LocaleNameToLCID, LocaleNameToLCID, 8)
XP_ANSWER(xp_GetUserDefaultLocaleName, GetUserDefaultLocaleName, 8)
XP_ANSWER(xp_SetThreadErrorMode, SetThreadErrorMode, 8)
XP_ANSWER(xp_CreateWaitableTimerExW, CreateWaitableTimerExW, 16)
XP_ANSWER(xp_SetThreadStackGuarantee, SetThreadStackGuarantee, 4)
XP_ANSWER(xp_GetDynamicTimeZoneInformation, GetDynamicTimeZoneInformation, 4)
XP_ANSWER(xp_GetTimeZoneInformationForYear, GetTimeZoneInformationForYear, 12)
XP_ANSWER(xp_RegGetValueW, RegGetValueW, 28)
XP_ANSWER(xp_RegCreateKeyTransactedW, RegCreateKeyTransactedW, 44)
XP_ANSWER(xp_RegOpenKeyTransactedW, RegOpenKeyTransactedW, 28)
XP_ANSWER(xp_RegDeleteTreeW, RegDeleteTreeW, 8)
XP_ANSWER(xp_CreateTransaction, CreateTransaction, 28)
XP_ANSWER(xp_CommitTransaction, CommitTransaction, 4)
XP_ANSWER(xp_CancelIoEx, CancelIoEx, 8)
XP_ANSWER(xp_CreateSymbolicLinkW, CreateSymbolicLinkW, 12)
XP_ANSWER(xp_ReOpenFile, ReOpenFile, 16)
XP_ANSWER(xp_InitializeProcThreadAttributeList, InitializeProcThreadAttributeList, 16)
XP_ANSWER(xp_UpdateProcThreadAttribute, UpdateProcThreadAttribute, 28)
XP_ANSWER(xp_DeleteProcThreadAttributeList, DeleteProcThreadAttributeList, 4)
XP_ANSWER(xp_WSAPoll, WSAPoll, 12)
XP_ANSWER(xp_WSASendMsg, WSASendMsg, 24)
XP_ANSWER(xp_CreateDXGIFactory1, CreateDXGIFactory1, 8)
XP_ANSWER(xp_SHGetStockIconInfo, SHGetStockIconInfo, 12)
XP_ANSWER(xp_DwmEnableBlurBehindWindow, DwmEnableBlurBehindWindow, 8)
XP_ANSWER(xp_DwmSetWindowAttribute, DwmSetWindowAttribute, 16)
XP_ANSWER(xp_PropVariantToBSTR, PropVariantToBSTR, 8)
XP_ANSWER(xp_PropVariantToVariant, PropVariantToVariant, 8)
XP_ANSWER(xp_VariantToPropVariant, VariantToPropVariant, 8)
XP_ANSWER(xp_PropVariantToBoolean, PropVariantToBoolean, 8)
XP_ANSWER(xp_PropVariantToInt16, PropVariantToInt16, 8)
XP_ANSWER(xp_PropVariantToInt32, PropVariantToInt32, 8)
XP_ANSWER(xp_PropVariantToInt64, PropVariantToInt64, 8)
XP_ANSWER(xp_PropVariantToUInt16, PropVariantToUInt16, 8)
XP_ANSWER(xp_PropVariantToUInt32, PropVariantToUInt32, 8)
XP_ANSWER(xp_PropVariantToUInt64, PropVariantToUInt64, 8)
XP_ANSWER(xp_PropVariantToDouble, PropVariantToDouble, 8)
XP_ANSWER(xp_VariantToBoolean, VariantToBoolean, 8)
XP_ANSWER(xp_VariantToInt16, VariantToInt16, 8)
XP_ANSWER(xp_VariantToInt32, VariantToInt32, 8)
XP_ANSWER(xp_VariantToInt64, VariantToInt64, 8)
XP_ANSWER(xp_VariantToUInt16, VariantToUInt16, 8)
XP_ANSWER(xp_VariantToUInt32, VariantToUInt32, 8)
XP_ANSWER(xp_VariantToUInt64, VariantToUInt64, 8)
XP_ANSWER(xp_VariantToDouble, VariantToDouble, 8)
XP_ANSWER(xp_PropVariantCompareEx, PropVariantCompareEx, 16)
XP_ANSWER(xp_UiaLookupId, UiaLookupId, 8)
XP_ANSWER(xp_UiaGetReservedNotSupportedValue, UiaGetReservedNotSupportedValue, 4)
XP_ANSWER(xp_UiaGetReservedMixedAttributeValue, UiaGetReservedMixedAttributeValue, 4)
XP_ANSWER(xp_UiaHostProviderFromHwnd, UiaHostProviderFromHwnd, 8)
XP_ANSWER(xp_UiaReturnRawElementProvider, UiaReturnRawElementProvider, 16)
XP_ANSWER(xp_UiaRaiseAutomationEvent, UiaRaiseAutomationEvent, 8)
XP_ANSWER(xp_UiaRaiseAutomationPropertyChangedEvent, UiaRaiseAutomationPropertyChangedEvent, 16)
XP_ANSWER(xp_SetWindowSubclass, SetWindowSubclass, 16)
XP_ANSWER(xp_RemoveWindowSubclass, RemoveWindowSubclass, 12)
XP_ANSWER(xp_DefSubclassProc, DefSubclassProc, 16)
XP_ANSWER(xp_CloseTouchInputHandle, CloseTouchInputHandle, 4)
XP_ANSWER(xp_GetTouchInputInfo, GetTouchInputInfo, 16)
XP_ANSWER(xp_RegisterTouchWindow, RegisterTouchWindow, 8)
XP_ANSWER(xp_IsProcessDPIAware, IsProcessDPIAware, 0)
XP_ANSWER(xp_SetWindowDisplayAffinity, SetWindowDisplayAffinity, 8)
XP_ANSWER(xp_GetDisplayConfigBufferSizes, GetDisplayConfigBufferSizes, 12)
XP_ANSWER(xp_QueryDisplayConfig, QueryDisplayConfig, 24)
XP_ANSWER(xp_DisplayConfigGetDeviceInfo, DisplayConfigGetDeviceInfo, 4)
XP_ANSWER(xp_NCryptFreeObject, NCryptFreeObject, 4)
