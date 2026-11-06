/*
 * xpshot.exe <file.bmp>: a picture of the whole screen (the desktop the user sees) as a 24-bit BMP, for
 * the Windows XP test VM (lane WINXP; XP has no screenshot command). user32 + gdi32 + kernel32 only, no C
 * runtime, PE subsystem 5.1. Ours, written from Microsoft's public documentation of GetDC, BitBlt,
 * CreateDIBSection and the BMP file format (BITMAPFILEHEADER, BITMAPINFOHEADER).
 */
typedef unsigned short wchar16;
typedef unsigned long ULONG;
typedef void *HANDLE;
typedef int BOOL;
#define WINAPI __attribute__((stdcall))

typedef struct { ULONG biSize; long biWidth, biHeight; unsigned short biPlanes, biBitCount; ULONG biCompression, biSizeImage; long biXPelsPerMeter, biYPelsPerMeter; ULONG biClrUsed, biClrImportant; } BITMAPINFOHEADER;
typedef struct { BITMAPINFOHEADER h; ULONG colors[1]; } BITMAPINFO;

__declspec(dllimport) HANDLE WINAPI GetDC(HANDLE);
__declspec(dllimport) int WINAPI ReleaseDC(HANDLE, HANDLE);
__declspec(dllimport) int WINAPI GetSystemMetrics(int);
__declspec(dllimport) HANDLE WINAPI CreateCompatibleDC(HANDLE);
__declspec(dllimport) HANDLE WINAPI CreateDIBSection(HANDLE, BITMAPINFO *, unsigned, void **, HANDLE, ULONG);
__declspec(dllimport) HANDLE WINAPI SelectObject(HANDLE, HANDLE);
__declspec(dllimport) BOOL WINAPI BitBlt(HANDLE, int, int, int, int, HANDLE, int, int, ULONG);
__declspec(dllimport) BOOL WINAPI DeleteDC(HANDLE);
__declspec(dllimport) BOOL WINAPI DeleteObject(HANDLE);
__declspec(dllimport) BOOL WINAPI GdiFlush(void);
__declspec(dllimport) wchar16 *WINAPI GetCommandLineW(void);
__declspec(dllimport) HANDLE WINAPI CreateFileW(const wchar16 *, ULONG, ULONG, void *, ULONG, ULONG, HANDLE);
__declspec(dllimport) BOOL WINAPI WriteFile(HANDLE, const void *, ULONG, ULONG *, void *);
__declspec(dllimport) BOOL WINAPI CloseHandle(HANDLE);
__declspec(dllimport) void WINAPI ExitProcess(unsigned);

void *memset(void *d, int c, unsigned n) { char *a = d; while (n--) *a++ = (char)c; return d; }
void *memcpy(void *d, const void *s, unsigned n) { char *a = d; const char *b = s; while (n--) *a++ = *b++; return d; }

void shot_main(void) {
    wchar16 *cmd = GetCommandLineW(), *path;
    int w = GetSystemMetrics(0), h = GetSystemMetrics(1);
    BITMAPINFO bi;
    void *bits = 0;
    HANDLE screen, dc, bmp, file;
    ULONG stride, size, wrote;
    unsigned char head[14];
    /* the first argument: after the program's own name (quoted or not) */
    if (*cmd == '"') { cmd++; while (*cmd && *cmd != '"') cmd++; if (*cmd) cmd++; }
    else while (*cmd && *cmd != ' ') cmd++;
    while (*cmd == ' ') cmd++;
    path = cmd;
    if (!*path) ExitProcess(2);
    memset(&bi, 0, sizeof bi);
    bi.h.biSize = sizeof bi.h;
    bi.h.biWidth = w;
    bi.h.biHeight = h; /* bottom-up, as BMP files are */
    bi.h.biPlanes = 1;
    bi.h.biBitCount = 24;
    stride = ((ULONG)w * 3 + 3) & ~3UL;
    size = stride * (ULONG)h;
    screen = GetDC(0);
    dc = CreateCompatibleDC(screen);
    bmp = CreateDIBSection(screen, &bi, 0, &bits, 0, 0);
    if (!bmp || !bits) ExitProcess(3);
    SelectObject(dc, bmp);
    BitBlt(dc, 0, 0, w, h, screen, 0, 0, 0x00CC0020 /* SRCCOPY */ | 0x40000000 /* CAPTUREBLT */);
    GdiFlush();
    file = CreateFileW(path, 0x40000000 /* GENERIC_WRITE */, 0, 0, 2 /* CREATE_ALWAYS */, 0x80, 0);
    if (file == (HANDLE)-1) ExitProcess(4);
    head[0] = 'B'; head[1] = 'M';
    {
        ULONG total = 14 + sizeof bi.h + size, off = 14 + sizeof bi.h;
        int i;
        for (i = 0; i < 4; i++) head[2 + i] = (unsigned char)(total >> (8 * i));
        head[6] = head[7] = head[8] = head[9] = 0;
        for (i = 0; i < 4; i++) head[10 + i] = (unsigned char)(off >> (8 * i));
    }
    WriteFile(file, head, 14, &wrote, 0);
    WriteFile(file, &bi.h, sizeof bi.h, &wrote, 0);
    WriteFile(file, bits, size, &wrote, 0);
    CloseHandle(file);
    DeleteDC(dc);
    DeleteObject(bmp);
    ReleaseDC(0, screen);
    ExitProcess(0);
}
