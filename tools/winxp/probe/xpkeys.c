/*
 * xpkeys.exe <chord> [<chord> ...]: key presses as a keyboard makes them, for the Windows XP test VM
 * (lane WINXP-RECHECK). WshShell.SendKeys's Ctrl and Alt chords did not reach RapidR Studio on XP; this
 * sends each key with both its virtual-key code and its scan code (SendInput, as the keyboard driver
 * does), modifiers held around it, to the window in front.
 *   chords: ctrl+s, ctrl+shift+b, alt+f, shift+f10, apps (the Menu key), f5, enter, esc, tab, down ...
 * user32 + kernel32 only, no C runtime, PE subsystem 5.1. Ours, written from Microsoft's public
 * documentation of SendInput, INPUT, KEYBDINPUT, MapVirtualKeyW and the virtual-key codes.
 */
typedef unsigned short wchar16;
typedef unsigned long ULONG;
typedef int BOOL;
#define WINAPI __attribute__((stdcall))

typedef struct {
    ULONG type;
    unsigned short vk, scan;
    ULONG flags, time;
    unsigned long extra;
    ULONG pad[2]; /* (INPUT's union is as large as MOUSEINPUT: 24 bytes) */
} Input;
typedef char input_size_check[sizeof(Input) == 28 ? 1 : -1];

__declspec(dllimport) unsigned WINAPI SendInput(unsigned, Input *, int);
__declspec(dllimport) unsigned WINAPI MapVirtualKeyW(unsigned, unsigned);
__declspec(dllimport) wchar16 *WINAPI GetCommandLineW(void);
__declspec(dllimport) void WINAPI Sleep(ULONG);
__declspec(dllimport) void WINAPI ExitProcess(unsigned);

#define KEYUP 0x0002
#define EXTENDED 0x0001

static void key(unsigned vk, int up) {
    Input in;
    char *p = (char *)&in;
    unsigned i;
    for (i = 0; i < sizeof in; i++) p[i] = 0;
    in.type = 1; /* INPUT_KEYBOARD */
    in.vk = (unsigned short)vk;
    in.scan = (unsigned short)MapVirtualKeyW(vk, 0);
    in.flags = (up ? KEYUP : 0);
    /* (the keys of the extended block: the arrows, the Menu key, Insert, Delete, Home, End, Page Up/Down) */
    if (vk == 0x5D || (vk >= 0x21 && vk <= 0x28) || vk == 0x2D || vk == 0x2E) in.flags |= EXTENDED;
    SendInput(1, &in, sizeof in);
    Sleep(40);
}

static int lower(int c) { return c >= 'A' && c <= 'Z' ? c + 32 : c; }
static int same(const char *a, const char *b) {
    while (*a && *a == *b) { a++; b++; }
    return *a == *b;
}
/* a word's virtual-key code (0: not known) */
static unsigned code(const wchar16 *w, int n) {
    char s[12];
    int i;
    if (n <= 0 || n > 10) return 0;
    for (i = 0; i < n; i++) s[i] = (char)lower(w[i]);
    s[n] = 0;
    if (n == 1 && s[0] >= 'a' && s[0] <= 'z') return (unsigned)(s[0] - 32);
    if (n == 1 && s[0] >= '0' && s[0] <= '9') return (unsigned)s[0];
    if (s[0] == 'f' && n >= 2) {
        int v = 0;
        for (i = 1; i < n; i++) { if (s[i] < '0' || s[i] > '9') { v = 0; break; } v = v * 10 + (s[i] - '0'); }
        if (v >= 1 && v <= 12) return 0x70 + (unsigned)(v - 1);
    }
#define IS(t) same(s, t)
    if (IS("apps") || IS("menu")) return 0x5D;
    if (IS("enter")) return 0x0D;
    if (IS("esc")) return 0x1B;
    if (IS("tab")) return 0x09;
    if (IS("space")) return 0x20;
    if (IS("down")) return 0x28;
    if (IS("up")) return 0x26;
    if (IS("left")) return 0x25;
    if (IS("right")) return 0x27;
    if (IS("ctrl")) return 0x11;
    if (IS("shift")) return 0x10;
    if (IS("alt")) return 0x12;
    return 0;
}

void keys_main(void) {
    wchar16 *c = GetCommandLineW();
    /* (past the program's own name) */
    if (*c == '"') { c++; while (*c && *c != '"') c++; if (*c) c++; } else while (*c && *c != ' ') c++;
    for (;;) {
        unsigned held[4], n = 0, k = 0, i;
        while (*c == ' ') c++;
        if (!*c) break;
        /* one chord: words joined by + */
        for (;;) {
            const wchar16 *w = c;
            unsigned v;
            while (*c && *c != '+' && *c != ' ') c++;
            v = code(w, (int)(c - w));
            if (!v) ExitProcess(2);
            if ((v == 0x11 || v == 0x10 || v == 0x12) && *c == '+') { if (n < 4) held[n++] = v; } else k = v;
            if (*c == '+') { c++; continue; }
            break;
        }
        for (i = 0; i < n; i++) key(held[i], 0);
        if (k) { key(k, 0); key(k, 1); }
        for (i = n; i > 0; i--) key(held[i - 1], 1);
        Sleep(250);
    }
    ExitProcess(0);
}
