/*
 * Windows XP: the pipes Rust's standard library makes for a child process's stdin, stdout and
 * stderr (lane WINXP, 2026-11-05). Compiled ONLY into the Windows XP flavour (tools/winxp/build.sh):
 * no other RapidR program links this file.
 *
 * Why. Rust's std (the nightly the Windows 7 / XP builds use, library/std/src/sys/process/windows/
 * child_pipe.rs) makes those pipes with two NT calls: NtCreateNamedPipeFile with an EMPTY name relative
 * to the pipe file system's root (\Device\NamedPipe\), an unnamed pipe; then NtOpenFile with an empty
 * name relative to that pipe, its other end. Windows 7 and later accept both. Windows XP's pipe file
 * system answers STATUS_OBJECT_NAME_INVALID, which std reports as os error 123 ("The filename,
 * directory name, or volume label syntax is incorrect"): every program started with a captured
 * output fails, e.g. RapidR Studio's Run (F5) and Build.
 *
 * What this does. This object defines the names the program's calls to the two functions go through
 * (xp_answer.h), each aimed at a function of ours:
 *   - xp_NtCreateNamedPipeFile calls the real function first. Only when Windows refuses an unnamed
 *     pipe (STATUS_OBJECT_NAME_INVALID for an empty name under a root directory) does it make the same
 *     pipe again with a name nobody can guess (64 random bits from RtlGenRandom), relative to the same
 *     root, with a security descriptor that lets only this user open it, and remember (handle, name).
 *   - xp_NtOpenFile: an empty-name open relative to one of THOSE pipes opens it by its name instead;
 *     every other call goes straight to the real function, unchanged.
 * On Windows Vista, 7 and later the first real call succeeds, the table stays empty and nothing
 * changes. The real functions are ntdll.dll's (found with GetModuleHandleW + GetProcAddress).
 *
 * Written from public documentation only: Microsoft Learn's pages on NtOpenFile / ZwOpenFile,
 * NtCreateFile, OBJECT_ATTRIBUTES, UNICODE_STRING, IO_STATUS_BLOCK, NTSTATUS values, CreateNamedPipeW
 * (the default security of a named pipe), InitializeSecurityDescriptor, SetSecurityDescriptorDacl,
 * InitializeAcl, AddAccessAllowedAce, GetTokenInformation (TokenUser), RtlGenRandom; the signature of
 * NtCreateNamedPipeFile as Rust's std declares it (library/std/src/sys/pal/windows/c.rs, MIT/Apache-2.0);
 * and the PE/COFF import naming (`__imp_` slots, stdcall `@N` decoration). No code of One-Core-API, of
 * mingw-w64, of Microsoft or of anyone else was read or copied. It includes no system header.
 */

#include "xp_answer.h"

typedef unsigned short wchar16;
typedef unsigned long ULONG;
typedef long NTSTATUS;
typedef void *HANDLE;
typedef int BOOL;

#define WINAPI __attribute__((stdcall))
#define STATUS_OBJECT_NAME_INVALID ((NTSTATUS)0xC0000033L)
#define STATUS_NO_MEMORY ((NTSTATUS)0xC0000017L)

typedef struct {
    unsigned short Length;        /* bytes, without a terminating zero */
    unsigned short MaximumLength;
    wchar16 *Buffer;
} UNICODE_STRING;

typedef struct {
    ULONG Length;
    HANDLE RootDirectory;
    UNICODE_STRING *ObjectName;
    ULONG Attributes;
    void *SecurityDescriptor;
    void *SecurityQualityOfService;
} OBJECT_ATTRIBUTES;

typedef NTSTATUS(WINAPI *CreatePipeFn)(HANDLE *, ULONG, OBJECT_ATTRIBUTES *, void *, ULONG, ULONG, ULONG, ULONG,
                                       ULONG, ULONG, ULONG, ULONG, ULONG, void *);
typedef NTSTATUS(WINAPI *OpenFileFn)(HANDLE *, ULONG, OBJECT_ATTRIBUTES *, void *, ULONG, ULONG);

__declspec(dllimport) void *WINAPI GetModuleHandleW(const wchar16 *name);
__declspec(dllimport) void *WINAPI GetProcAddress(void *module, const char *name);
__declspec(dllimport) ULONG WINAPI GetCurrentProcessId(void);
__declspec(dllimport) HANDLE WINAPI GetCurrentProcess(void);
__declspec(dllimport) BOOL WINAPI CloseHandle(HANDLE h);
__declspec(dllimport) void WINAPI Sleep(ULONG ms);
__declspec(dllimport) void *WINAPI HeapAlloc(HANDLE heap, ULONG flags, unsigned long bytes); /* 32-bit only */
__declspec(dllimport) HANDLE WINAPI GetProcessHeap(void);
__declspec(dllimport) BOOL WINAPI OpenProcessToken(HANDLE process, ULONG access, HANDLE *token);
__declspec(dllimport) BOOL WINAPI GetTokenInformation(HANDLE token, int cls, void *info, ULONG len, ULONG *needed);
__declspec(dllimport) BOOL WINAPI InitializeSecurityDescriptor(void *sd, ULONG revision);
__declspec(dllimport) BOOL WINAPI SetSecurityDescriptorDacl(void *sd, BOOL present, void *acl, BOOL defaulted);
__declspec(dllimport) BOOL WINAPI InitializeAcl(void *acl, ULONG len, ULONG revision);
__declspec(dllimport) BOOL WINAPI AddAccessAllowedAce(void *acl, ULONG revision, ULONG mask, void *sid);
__declspec(dllimport) ULONG WINAPI GetLengthSid(void *sid);
/* RtlGenRandom (advapi32 exports it as SystemFunction036; documented under RtlGenRandom) */
__declspec(dllimport) unsigned char WINAPI SystemFunction036(void *buffer, ULONG len);

#ifdef XP_PIPES_TEST
int xp_pipes_force_named;
#endif

static const wchar16 NTDLL[] = {'n', 't', 'd', 'l', 'l', '.', 'd', 'l', 'l', 0};

static CreatePipeFn real_create_pipe(void) {
    static CreatePipeFn fn;
    if (!fn) fn = (CreatePipeFn)GetProcAddress(GetModuleHandleW(NTDLL), "NtCreateNamedPipeFile");
    return fn;
}

static OpenFileFn real_open_file(void) {
    static OpenFileFn fn;
    if (!fn) fn = (OpenFileFn)GetProcAddress(GetModuleHandleW(NTDLL), "NtOpenFile");
    return fn;
}

/* ---- the pipes this object named: (server handle, root, name), a few at a time ---- */

#define SLOTS 32
#define NAME_CHARS 56
typedef struct {
    HANDLE pipe;  /* 0 = free */
    HANDLE root;
    wchar16 name[NAME_CHARS];
    unsigned short bytes;
} Named;
static Named named[SLOTS];
static volatile long table_lock;
static volatile long in_use; /* how many slots hold a pipe: 0 = nothing to look up (Windows 7+) */

static void lock(void) {
    while (!__sync_bool_compare_and_swap(&table_lock, 0, 1)) Sleep(0);
}
static void unlock(void) { __sync_lock_release(&table_lock); }

/* a name: "RapidR-pipe-<pid>-<16 random hex digits>" (UTF-16, no terminating zero needed) */
static int make_name(wchar16 *out, unsigned short *bytes) {
    static const char hex[] = "0123456789abcdef";
    static const char prefix[] = "RapidR-pipe-";
    unsigned char rnd[8];
    int n = 0, i;
    unsigned long pid = GetCurrentProcessId();
    char digits[12];
    int d = 0;
    if (!SystemFunction036(rnd, sizeof rnd)) return 0;
    for (i = 0; prefix[i]; i++) out[n++] = (wchar16)prefix[i];
    do { digits[d++] = (char)('0' + pid % 10); pid /= 10; } while (pid && d < 12);
    while (d) out[n++] = (wchar16)digits[--d];
    out[n++] = '-';
    for (i = 0; i < 8; i++) {
        out[n++] = (wchar16)hex[rnd[i] >> 4];
        out[n++] = (wchar16)hex[rnd[i] & 15];
    }
    *bytes = (unsigned short)(n * 2);
    return 1;
}

/* A security descriptor whose list lets only this process's user open the pipe (the default one of a
 * named pipe also lets Everyone read it). Made once; 0 if it can't be made. */
static void *owner_only_sd(void) {
    static void *sd;
    static volatile long made;
    if (made == 2) return sd;
    lock();
    if (made == 0) {
        HANDLE token = 0;
        void *user[64]; /* TOKEN_USER: { SID_AND_ATTRIBUTES { PSID Sid; DWORD Attributes } } + the SID */
        ULONG needed = 0;
        made = 1;
        if (OpenProcessToken(GetCurrentProcess(), 0x0008 /* TOKEN_QUERY */, &token)) {
            if (GetTokenInformation(token, 1 /* TokenUser */, user, sizeof user, &needed)) {
                void *sid = user[0];
                ULONG acl_len = 8 /* ACL header */ + 8 /* ACE header + mask */ + GetLengthSid(sid) + 8;
                unsigned char *mem = (unsigned char *)HeapAlloc(GetProcessHeap(), 8 /* zero */, 64 + acl_len);
                if (mem) {
                    void *acl = mem + 64; /* SECURITY_DESCRIPTOR (20 bytes on x86) first, the list after */
                    if (InitializeSecurityDescriptor(mem, 1) && InitializeAcl(acl, acl_len, 2) &&
                        AddAccessAllowedAce(acl, 2, 0x10000000 /* GENERIC_ALL */, sid) &&
                        SetSecurityDescriptorDacl(mem, 1, acl, 0))
                        sd = mem;
                }
            }
            CloseHandle(token);
        }
        made = 2;
    }
    unlock();
    return sd;
}

NTSTATUS WINAPI xp_NtCreateNamedPipeFile(HANDLE *handle, ULONG access, OBJECT_ATTRIBUTES *attrs, void *io,
                                         ULONG share, ULONG disposition, ULONG options, ULONG type, ULONG mode,
                                         ULONG completion, ULONG instances, ULONG in_quota, ULONG out_quota,
                                         void *timeout) {
    CreatePipeFn real = real_create_pipe();
    NTSTATUS status;
    OBJECT_ATTRIBUTES again;
    UNICODE_STRING name;
    Named slot;
    void *sd;
    int i;
    if (!real) return (NTSTATUS)0xC0000002L; /* STATUS_NOT_IMPLEMENTED */
    status = real(handle, access, attrs, io, share, disposition, options, type, mode, completion, instances, in_quota,
                  out_quota, timeout);
#ifdef XP_PIPES_TEST
    /* (tools/winxp/probe only: the named way on any Windows, to test it where unnamed pipes work) */
    if (xp_pipes_force_named && status >= 0 && attrs && attrs->ObjectName && attrs->ObjectName->Length == 0) {
        CloseHandle(*handle);
        status = STATUS_OBJECT_NAME_INVALID;
    }
#endif
    if (status != STATUS_OBJECT_NAME_INVALID || !attrs || !attrs->RootDirectory || !attrs->ObjectName ||
        attrs->ObjectName->Length != 0)
        return status;
    /* Windows can't make an unnamed pipe here (XP): the same pipe, named */
    sd = owner_only_sd();
    if (!sd || !make_name(slot.name, &slot.bytes)) return status;
    name.Length = slot.bytes;
    name.MaximumLength = slot.bytes;
    name.Buffer = slot.name;
    again = *attrs;
    again.ObjectName = &name;
    again.SecurityDescriptor = sd;
    status = real(handle, access, &again, io, share, disposition, options, type, mode, completion, instances, in_quota,
                  out_quota, timeout);
    if (status < 0) return status;
    slot.pipe = *handle;
    slot.root = attrs->RootDirectory;
    lock();
    for (i = 0; i < SLOTS && named[i].pipe; i++) {}
    if (i == SLOTS) i = (int)(GetCurrentProcessId() % SLOTS); /* full (ends never opened): reuse one */
    else in_use++;
    named[i] = slot;
    unlock();
    return status;
}

NTSTATUS WINAPI xp_NtOpenFile(HANDLE *handle, ULONG access, OBJECT_ATTRIBUTES *attrs, void *io, ULONG share,
                              ULONG options) {
    OpenFileFn real = real_open_file();
    if (!real) return (NTSTATUS)0xC0000002L;
    if (in_use && attrs && attrs->RootDirectory && attrs->ObjectName && attrs->ObjectName->Length == 0) {
        Named slot;
        int i, found = 0;
        lock();
        for (i = 0; i < SLOTS; i++) {
            if (named[i].pipe && named[i].pipe == attrs->RootDirectory) {
                slot = named[i];
                named[i].pipe = 0;
                in_use--;
                found = 1;
                break;
            }
        }
        unlock();
        if (found) {
            /* the other end of a pipe we named: opened by its name, under the same root */
            OBJECT_ATTRIBUTES again = *attrs;
            UNICODE_STRING name;
            name.Length = slot.bytes;
            name.MaximumLength = slot.bytes;
            name.Buffer = slot.name;
            again.RootDirectory = slot.root;
            again.ObjectName = &name;
            return real(handle, access, &again, io, share, options);
        }
    }
    return real(handle, access, attrs, io, share, options);
}

/* Every name a caller can use for the two functions (xp_answer.h): the import thunk `_F@N` that
 * Rust's std calls, the slot `__imp__F@N` that C code (dllimport) loads, and the slot `__imp_F` of the
 * crates built with --cfg windows_raw_dylib. Defined here, the import library's entries never get in. */
XP_ANSWER(xp_NtCreateNamedPipeFile, NtCreateNamedPipeFile, 56)
XP_ANSWER(xp_NtOpenFile, NtOpenFile, 24)
