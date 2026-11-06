/*
 * Shared pieces of the Windows XP stand-ins (lane WINXP). 32-bit x86 only. Ours, from Microsoft
 * Learn's documentation of GetModuleHandleW, LoadLibraryW, GetSystemDirectoryW, GetProcAddress,
 * Get/SetLastError.
 *
 * xp_real(dll, name, &cache): Windows' own `name` from `dll` (a DLL of Windows' own folder, loaded
 * from there by its full path if the program has not loaded it), looked up once; 0 where this Windows
 * does not have it. The thread's last-error value is kept as it was.
 */
#ifndef XP_COMMON_H
#define XP_COMMON_H
#include "xp_answer.h"

typedef unsigned short wchar16;
typedef unsigned long ULONG;
typedef long LONG;
typedef long HRESULT;
typedef long NTSTATUS;
typedef void *HANDLE;
typedef int BOOL;
#define WINAPI __attribute__((stdcall))
#define E_NOTIMPL ((HRESULT)0x80004001L)
#define ERROR_CALL_NOT_IMPLEMENTED 120UL
#define ERROR_NOT_SUPPORTED 50UL
#define ERROR_INVALID_PARAMETER 87UL
#define ERROR_INSUFFICIENT_BUFFER 122UL
#define ERROR_MORE_DATA 234UL
#define INVALID_HANDLE_VALUE ((HANDLE)(long)-1)

__declspec(dllimport) void *WINAPI GetModuleHandleW(const wchar16 *);
__declspec(dllimport) void *WINAPI LoadLibraryW(const wchar16 *);
__declspec(dllimport) void *WINAPI GetProcAddress(void *, const char *);
__declspec(dllimport) unsigned WINAPI GetSystemDirectoryW(wchar16 *, unsigned);
__declspec(dllimport) ULONG WINAPI GetLastError(void);
__declspec(dllimport) void WINAPI SetLastError(ULONG);

#define XP_ABSENT ((void *)1)

static void *xp_real(const wchar16 *dll, const char *name, void *volatile *cache) {
    void *f = *cache;
    if (!f) {
        ULONG err = GetLastError();
        void *m = GetModuleHandleW(dll);
        if (!m) {
            wchar16 path[300];
            unsigned n = GetSystemDirectoryW(path, 260), i = 0;
            if (n > 0 && n < 260) {
                path[n++] = '\\';
                while (dll[i] && n < 298) path[n++] = dll[i++];
                path[n] = 0;
                m = LoadLibraryW(path);
            }
        }
        f = m ? GetProcAddress(m, name) : 0;
        if (!f) f = XP_ABSENT;
        __sync_synchronize();
        *cache = f;
        SetLastError(err);
    }
    return f == XP_ABSENT ? 0 : f;
}

/* REAL(type, dll, name): a static cache and the lookup, as a typed function pointer (0 = absent) */
#define REAL(type, dll, name)                                                                            \
    ({                                                                                                   \
        static void *volatile cache_;                                                                    \
        (type) xp_real(dll, name, &cache_);                                                              \
    })

static const wchar16 W_KERNEL32[] = {'k', 'e', 'r', 'n', 'e', 'l', '3', '2', '.', 'd', 'l', 'l', 0};
static const wchar16 W_ADVAPI32[] = {'a', 'd', 'v', 'a', 'p', 'i', '3', '2', '.', 'd', 'l', 'l', 0};
static const wchar16 W_NTDLL[] = {'n', 't', 'd', 'l', 'l', '.', 'd', 'l', 'l', 0};
static const wchar16 W_PSAPI[] = {'p', 's', 'a', 'p', 'i', '.', 'd', 'l', 'l', 0};
static const wchar16 W_USER32[] = {'u', 's', 'e', 'r', '3', '2', '.', 'd', 'l', 'l', 0};
static const wchar16 W_SHELL32[] = {'s', 'h', 'e', 'l', 'l', '3', '2', '.', 'd', 'l', 'l', 0};
static const wchar16 W_COMCTL32[] = {'c', 'o', 'm', 'c', 't', 'l', '3', '2', '.', 'd', 'l', 'l', 0};
static const wchar16 W_WS2_32[] = {'w', 's', '2', '_', '3', '2', '.', 'd', 'l', 'l', 0};
static const wchar16 W_DWMAPI[] = {'d', 'w', 'm', 'a', 'p', 'i', '.', 'd', 'l', 'l', 0};
static const wchar16 W_DXGI[] = {'d', 'x', 'g', 'i', '.', 'd', 'l', 'l', 0};
static const wchar16 W_DWRITE[] = {'d', 'w', 'r', 'i', 't', 'e', '.', 'd', 'l', 'l', 0};
static const wchar16 W_UIA[] = {'u', 'i', 'a', 'u', 't', 'o', 'm', 'a', 't', 'i', 'o', 'n', 'c', 'o', 'r', 'e', '.', 'd', 'l', 'l', 0};
static const wchar16 W_PROPSYS[] = {'p', 'r', 'o', 'p', 's', 'y', 's', '.', 'd', 'l', 'l', 0};
static const wchar16 W_KTMW32[] = {'k', 't', 'm', 'w', '3', '2', '.', 'd', 'l', 'l', 0};
static const wchar16 W_NCRYPT[] = {'n', 'c', 'r', 'y', 'p', 't', '.', 'd', 'l', 'l', 0};
#endif
