/*
 * xpclick.exe <x> <y> [double]: a left click (or a double click) at a point of the screen, as a person
 * would make it, for the Windows XP test VM (lane WINXP; XP's script host can type but not click).
 * user32 + kernel32 only, no C runtime, PE subsystem 5.1. Ours, written from Microsoft's public
 * documentation of SetCursorPos, mouse_event and GetCommandLineW.
 */
typedef unsigned short wchar16;
typedef unsigned long ULONG;
typedef int BOOL;
#define WINAPI __attribute__((stdcall))

__declspec(dllimport) BOOL WINAPI SetCursorPos(int, int);
__declspec(dllimport) void WINAPI mouse_event(ULONG, ULONG, ULONG, ULONG, unsigned long);
__declspec(dllimport) wchar16 *WINAPI GetCommandLineW(void);
__declspec(dllimport) void WINAPI Sleep(ULONG);
__declspec(dllimport) void WINAPI ExitProcess(unsigned);

#define LEFTDOWN 0x0002
#define LEFTUP 0x0004

static wchar16 *skip_word(wchar16 *c) {
    if (*c == '"') { c++; while (*c && *c != '"') c++; if (*c) c++; }
    else while (*c && *c != ' ') c++;
    while (*c == ' ') c++;
    return c;
}
static int number(wchar16 **c) {
    int v = 0;
    while (**c >= '0' && **c <= '9') { v = v * 10 + (**c - '0'); (*c)++; }
    while (**c == ' ') (*c)++;
    return v;
}

void click_main(void) {
    wchar16 *c = skip_word(GetCommandLineW());
    int x = number(&c), y = number(&c), times = (*c == 'd') ? 2 : 1, i;
    if (!SetCursorPos(x, y)) ExitProcess(2);
    Sleep(150);
    for (i = 0; i < times; i++) {
        mouse_event(LEFTDOWN, 0, 0, 0, 0);
        Sleep(60);
        mouse_event(LEFTUP, 0, 0, 0, 0);
        Sleep(90);
    }
    ExitProcess(0);
}
