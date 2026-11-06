/*
 * pipeprobe.exe: does this Windows make the pipes Rust's std makes for a child process (lane WINXP)?
 *
 *   1. std's way: NtOpenFile(\Device\NamedPipe\), NtCreateNamedPipeFile with an empty name under it,
 *      NtOpenFile with an empty name under that pipe (library/std/src/sys/process/windows/child_pipe.rs);
 *   2. tools/winxp/shim/xp_pipes.c's way (linked in): the same calls through xp_NtCreateNamedPipeFile /
 *      xp_NtOpenFile, then a real child, `cmd /c echo`, with its stdout on that pipe, read back.
 * Prints each NTSTATUS. Uses kernel32 and advapi32 only (no C runtime), PE subsystem 5.1, so it starts
 * on plain Windows XP. Ours, written from Microsoft's public documentation of the calls it makes.
 */
typedef unsigned short wchar16;
typedef unsigned long ULONG;
typedef long NTSTATUS;
typedef void *HANDLE;
typedef int BOOL;
#define WINAPI __attribute__((stdcall))

typedef struct { unsigned short Length, MaximumLength; wchar16 *Buffer; } UNICODE_STRING;
typedef struct { ULONG Length; HANDLE RootDirectory; UNICODE_STRING *ObjectName; ULONG Attributes; void *SecurityDescriptor; void *SecurityQualityOfService; } OBJECT_ATTRIBUTES;
typedef struct { void *Status; ULONG Information; } IO_STATUS_BLOCK;
typedef NTSTATUS(WINAPI *CreatePipeFn)(HANDLE *, ULONG, OBJECT_ATTRIBUTES *, void *, ULONG, ULONG, ULONG, ULONG, ULONG, ULONG, ULONG, ULONG, ULONG, void *);
typedef NTSTATUS(WINAPI *OpenFileFn)(HANDLE *, ULONG, OBJECT_ATTRIBUTES *, void *, ULONG, ULONG);

typedef struct { ULONG cb; wchar16 *lpReserved, *lpDesktop, *lpTitle; ULONG dwX, dwY, dwXSize, dwYSize, dwXCountChars, dwYCountChars, dwFillAttribute, dwFlags; unsigned short wShowWindow, cbReserved2; void *lpReserved2; HANDLE hStdInput, hStdOutput, hStdError; } STARTUPINFOW;
typedef struct { HANDLE hProcess, hThread; ULONG dwProcessId, dwThreadId; } PROCESS_INFORMATION;
typedef struct { ULONG dwOSVersionInfoSize, dwMajorVersion, dwMinorVersion, dwBuildNumber, dwPlatformId; wchar16 szCSDVersion[128]; } OSVERSIONINFOW;

__declspec(dllimport) void *WINAPI GetModuleHandleW(const wchar16 *);
__declspec(dllimport) void *WINAPI GetProcAddress(void *, const char *);
__declspec(dllimport) HANDLE WINAPI GetStdHandle(ULONG);
__declspec(dllimport) BOOL WINAPI WriteFile(HANDLE, const void *, ULONG, ULONG *, void *);
__declspec(dllimport) BOOL WINAPI ReadFile(HANDLE, void *, ULONG, ULONG *, void *);
__declspec(dllimport) BOOL WINAPI CloseHandle(HANDLE);
__declspec(dllimport) void WINAPI ExitProcess(unsigned);
__declspec(dllimport) BOOL WINAPI CreateProcessW(const wchar16 *, wchar16 *, void *, void *, BOOL, ULONG, void *, const wchar16 *, STARTUPINFOW *, PROCESS_INFORMATION *);
__declspec(dllimport) ULONG WINAPI WaitForSingleObject(HANDLE, ULONG);
__declspec(dllimport) ULONG WINAPI GetLastError(void);
__declspec(dllimport) BOOL WINAPI GetVersionExW(OSVERSIONINFOW *);

NTSTATUS WINAPI xp_NtCreateNamedPipeFile(HANDLE *, ULONG, OBJECT_ATTRIBUTES *, void *, ULONG, ULONG, ULONG, ULONG, ULONG, ULONG, ULONG, ULONG, ULONG, void *);
NTSTATUS WINAPI xp_NtOpenFile(HANDLE *, ULONG, OBJECT_ATTRIBUTES *, void *, ULONG, ULONG);
extern int xp_pipes_force_named;
__declspec(dllimport) wchar16 *WINAPI GetCommandLineW(void);

void *memcpy(void *d, const void *s, unsigned n) { char *a = d; const char *b = s; while (n--) *a++ = *b++; return d; }
void *memset(void *d, int c, unsigned n) { char *a = d; while (n--) *a++ = (char)c; return d; }

static HANDLE out;
static void say(const char *s) { ULONG n = 0, w; while (s[n]) n++; WriteFile(out, s, n, &w, 0); }
static void hex(ULONG v) { char b[11]; int i; b[0] = '0'; b[1] = 'x'; for (i = 0; i < 8; i++) b[2 + i] = "0123456789ABCDEF"[(v >> (28 - 4 * i)) & 15]; b[10] = 0; say(b); }
static void num(ULONG v) { char b[12]; int i = 11; b[i] = 0; do { b[--i] = (char)('0' + v % 10); v /= 10; } while (v); say(b + i); }
static void line(const char *what, NTSTATUS s) { say(what); say(": "); hex((ULONG)s); say(s >= 0 ? "  ok\r\n" : "  FAILED\r\n"); }

#define SYNCHRONIZE 0x00100000UL
#define GENERIC_READ 0x80000000UL
#define GENERIC_WRITE 0x40000000UL

/* one pipe, made as std makes it (create, then the other end), with the given pair of functions;
 * ours = the readable end (std's stdout case), theirs = the writable end, inheritable */
static int pipe_pair(CreatePipeFn create, OpenFileFn open, HANDLE root, HANDLE *ours, HANDLE *theirs, const char *tag) {
    OBJECT_ATTRIBUTES oa = {sizeof(OBJECT_ATTRIBUTES), 0, 0, 0, 0, 0};
    UNICODE_STRING empty = {0, 0, 0};
    IO_STATUS_BLOCK io;
    long long timeout = -500000; /* 50 ms, relative */
    NTSTATUS s;
    oa.ObjectName = &empty;
    oa.RootDirectory = root;
    /* (synchronous here, so the probe can read it with ReadFile: std's own end is overlapped) */
    s = create(ours, SYNCHRONIZE | GENERIC_READ, &oa, &io, 2 /* FILE_SHARE_WRITE */, 2 /* FILE_CREATE */, 0x20 /* FILE_SYNCHRONOUS_IO_NONALERT */,
               0, 0, 0, 1, 65536, 65536, &timeout);
    say(tag); line(" NtCreateNamedPipeFile, empty name", s);
    if (s < 0) return 0;
    oa.RootDirectory = *ours;
    oa.Attributes = 2; /* OBJ_INHERIT */
    s = open(theirs, SYNCHRONIZE | GENERIC_WRITE | 0x80 /* FILE_READ_ATTRIBUTES */, &oa, &io, 0, 0x40 | 0x20 /* NON_DIRECTORY | SYNCHRONOUS */);
    say(tag); line(" NtOpenFile, its other end", s);
    return s >= 0;
}

void probe_main(void) {
    static const wchar16 NTDLL[] = {'n', 't', 'd', 'l', 'l', '.', 'd', 'l', 'l', 0};
    static wchar16 root_name[] = {'\\', 'D', 'e', 'v', 'i', 'c', 'e', '\\', 'N', 'a', 'm', 'e', 'd', 'P', 'i', 'p', 'e', '\\'};
    void *ntdll = GetModuleHandleW(NTDLL);
    CreatePipeFn create = (CreatePipeFn)GetProcAddress(ntdll, "NtCreateNamedPipeFile");
    OpenFileFn open = (OpenFileFn)GetProcAddress(ntdll, "NtOpenFile");
    OBJECT_ATTRIBUTES oa = {sizeof(OBJECT_ATTRIBUTES), 0, 0, 0, 0, 0};
    UNICODE_STRING rn = {sizeof root_name, sizeof root_name, root_name};
    IO_STATUS_BLOCK io;
    HANDLE root = 0, ours = 0, theirs = 0;
    OSVERSIONINFOW v;
    NTSTATUS s;
    int std_ok, our_ok, child_ok = 0;
    out = GetStdHandle((ULONG)-11);
    /* "pipeprobe force": xp_pipes takes the named way even where unnamed pipes work (to test it) */
    {
        wchar16 *c = GetCommandLineW();
        for (; *c; c++) if (c[0] == 'f' && c[1] == 'o' && c[2] == 'r' && c[3] == 'c' && c[4] == 'e') xp_pipes_force_named = 1;
    }
    v.dwOSVersionInfoSize = sizeof v;
    GetVersionExW(&v);
    say("pipeprobe: Windows "); num(v.dwMajorVersion); say("."); num(v.dwMinorVersion); say(" build "); num(v.dwBuildNumber); say("\r\n");
    if (!create || !open) { say("ntdll has no NtCreateNamedPipeFile / NtOpenFile\r\n"); ExitProcess(2); }
    oa.ObjectName = &rn;
    s = open(&root, SYNCHRONIZE | GENERIC_READ, &oa, &io, 3, 0x20);
    line("NtOpenFile \\Device\\NamedPipe\\", s);
    if (s < 0) ExitProcess(2);

    std_ok = pipe_pair(create, open, root, &ours, &theirs, "std's way:");
    if (ours) CloseHandle(ours);
    if (theirs) CloseHandle(theirs);
    ours = theirs = 0;

    our_ok = pipe_pair(xp_NtCreateNamedPipeFile, xp_NtOpenFile, root, &ours, &theirs, "xp_pipes:");
    if (our_ok) {
        static wchar16 cmd[] = {'c', 'm', 'd', '.', 'e', 'x', 'e', ' ', '/', 'c', ' ', 'e', 'c', 'h', 'o', ' ', 'h', 'e', 'l', 'l', 'o', '-', 'f', 'r', 'o', 'm', '-', 't', 'h', 'e', '-', 'c', 'h', 'i', 'l', 'd', 0};
        STARTUPINFOW si;
        PROCESS_INFORMATION pi;
        char buf[256];
        ULONG got = 0, total = 0;
        memset(&si, 0, sizeof si);
        si.cb = sizeof si;
        si.dwFlags = 0x100; /* STARTF_USESTDHANDLES */
        si.hStdInput = GetStdHandle((ULONG)-10);
        si.hStdOutput = theirs;
        si.hStdError = theirs;
        if (CreateProcessW(0, cmd, 0, 0, 1, 0x08000000 /* CREATE_NO_WINDOW */, 0, 0, &si, &pi)) {
            CloseHandle(theirs);
            theirs = 0;
            while (total < sizeof buf - 1 && ReadFile(ours, buf + total, sizeof buf - 1 - total, &got, 0) && got) total += got;
            buf[total] = 0;
            WaitForSingleObject(pi.hProcess, 10000);
            CloseHandle(pi.hProcess);
            CloseHandle(pi.hThread);
            say("child's output through the pipe: ");
            say(buf);
            child_ok = total >= 5;
            {
                const char *want = "hello-from-the-child";
                unsigned i, j, hit = 0;
                for (i = 0; i + 20 <= total && !hit; i++) { for (j = 0; j < 20 && buf[i + j] == want[j]; j++) {} hit = j == 20; }
                child_ok = hit;
            }
        } else {
            say("CreateProcessW failed, error "); num(GetLastError()); say("\r\n");
        }
    }
    if (xp_pipes_force_named) say("(force: xp_pipes named every pipe)\r\n");
    say("\r\nRESULT: std's way "); say(std_ok ? "works" : "FAILS");
    say("; xp_pipes "); say(our_ok ? "works" : "FAILS");
    say("; child through xp_pipes "); say(child_ok ? "works" : "FAILS"); say("\r\n");
    ExitProcess(child_ok ? 0 : 1);
}
